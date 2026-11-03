use super::{executor::ExternalRequest, output::CommandOutput};
use std::{
    io::{self, Read},
    os::{
        fd::OwnedFd,
        unix::{net::UnixStream, process::CommandExt},
    },
    process::{Child, Command, ExitStatus, Stdio},
};

/// Nonblocking output keeps rendering, resize and cancellation responsive.
/// stdout/stderr share one stream, so writes retain their observed order.
pub struct CapturedProcess {
    child: Child,
    output: UnixStream,
    reaped: bool,
}
impl CapturedProcess {
    pub fn spawn(request: &ExternalRequest) -> io::Result<Self> {
        let text = request
            .command
            .as_ref()
            .ok_or_else(|| io::Error::other("interactive shells require terminal handoff"))?;
        let (reader, writer) = UnixStream::pair()?;
        reader.set_nonblocking(true)?;
        let stderr: OwnedFd = writer.try_clone()?.into();
        let stdout: OwnedFd = writer.into();
        let mut command = Command::new(&request.shell.path);
        command
            .arg("-c")
            .arg(text)
            .current_dir(&request.directory)
            .stdin(Stdio::null())
            .stdout(Stdio::from(stdout))
            .stderr(Stdio::from(stderr));
        // No controlling terminal: interactive tools belong in the ! shell.
        // SAFETY: setsid is async-signal-safe and has no Rust allocation.
        unsafe {
            command.pre_exec(|| {
                if libc::setsid() < 0 {
                    return Err(io::Error::last_os_error());
                }
                Ok(())
            });
        }
        Ok(Self {
            child: command.spawn()?,
            output: reader,
            reaped: false,
        })
    }
    fn drain(&mut self, output: &mut CommandOutput) -> io::Result<()> {
        let mut bytes = [0; 8192];
        // Bound each frame's work even for an infinite output producer.
        for _ in 0..32 {
            match self.output.read(&mut bytes) {
                Ok(0) => break,
                Ok(n) => output.append(&bytes[..n]),
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => break,
                Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
                Err(e) => return Err(e),
            }
        }
        Ok(())
    }
    pub fn poll(&mut self, output: &mut CommandOutput) -> io::Result<Option<ExitStatus>> {
        self.drain(output)?;
        let status = self.child.try_wait()?;
        if status.is_some() {
            self.reaped = true;
            self.drain(output)?;
        }
        Ok(status)
    }
    pub fn interrupt(&self) -> io::Result<()> {
        // SAFETY: the child leads its own session/process group until reaped.
        if !self.reaped && unsafe { libc::kill(-(self.child.id() as i32), libc::SIGINT) } < 0 {
            let error = io::Error::last_os_error();
            if error.raw_os_error() != Some(libc::ESRCH) {
                return Err(error);
            }
        }
        Ok(())
    }
}
impl Drop for CapturedProcess {
    fn drop(&mut self) {
        if !self.reaped {
            // SAFETY: only our unreaped child's process group is targeted.
            unsafe {
                libc::kill(-(self.child.id() as i32), libc::SIGKILL);
            }
            let _ = self.child.wait();
        }
    }
}
