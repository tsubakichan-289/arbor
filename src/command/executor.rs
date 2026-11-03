use super::shell::Shell;
use std::{
    io,
    os::unix::process::{CommandExt, ExitStatusExt},
    path::PathBuf,
    process::{Command, ExitStatus, Stdio},
};

pub struct ExternalRequest {
    pub shell: Shell,
    pub directory: PathBuf,
    /// None starts an interactive shell on the existing controlling terminal.
    pub command: Option<String>,
}

pub fn execute(request: &ExternalRequest, foreground: bool) -> io::Result<ExitStatus> {
    let mut command = Command::new(&request.shell.path);
    command
        .current_dir(&request.directory)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());
    match &request.command {
        Some(text) => {
            command.arg("-c").arg(text);
        }
        None => {
            command.arg("-i");
        }
    }
    if !foreground {
        return command.status();
    }
    let mut terminal = Foreground::capture()?;
    // SAFETY: after fork, only async-signal-safe libc calls are made. All data
    // is stack allocated, and errors contain only the OS error number.
    unsafe {
        command.pre_exec(|| {
            if libc::setpgid(0, 0) != 0 || libc::tcsetpgrp(libc::STDIN_FILENO, libc::getpid()) != 0
            {
                return Err(io::Error::last_os_error());
            }
            let mut action: libc::sigaction = std::mem::zeroed();
            libc::sigemptyset(&mut action.sa_mask);
            action.sa_sigaction = libc::SIG_DFL;
            for signal in [
                libc::SIGINT,
                libc::SIGQUIT,
                libc::SIGTSTP,
                libc::SIGTTIN,
                libc::SIGTTOU,
            ] {
                if libc::sigaction(signal, &action, std::ptr::null_mut()) != 0 {
                    return Err(io::Error::last_os_error());
                }
            }
            if libc::sigprocmask(libc::SIG_SETMASK, &action.sa_mask, std::ptr::null_mut()) != 0 {
                return Err(io::Error::last_os_error());
            }
            Ok(())
        });
    }
    let result = command.spawn().and_then(|mut child| {
        loop {
            let mut status = 0;
            // SAFETY: child.id identifies our unreaped child; status is writable.
            let waited =
                unsafe { libc::waitpid(child.id() as libc::pid_t, &mut status, libc::WUNTRACED) };
            if waited < 0 {
                let error = io::Error::last_os_error();
                if error.kind() == io::ErrorKind::Interrupted {
                    continue;
                }
                let _ = child.kill();
                let _ = child.wait();
                return Err(error);
            }
            if libc::WIFSTOPPED(status) {
                // Arbor has no background-job table. Resume a stopped top-level
                // command instead of leaving the terminal stranded indefinitely.
                eprintln!("arbor: background suspension is unavailable; resuming command");
                // SAFETY: the negative PID denotes the child's process group.
                unsafe {
                    libc::kill(-(child.id() as libc::pid_t), libc::SIGCONT);
                }
                continue;
            }
            return Ok(ExitStatus::from_raw(status));
        }
    });
    let restored = terminal.restore();
    match (result, restored) {
        (Ok(status), Ok(())) => Ok(status),
        (Err(error), Ok(())) => Err(error),
        (result, Err(error)) => Err(io::Error::new(
            io::ErrorKind::BrokenPipe,
            format!("cannot reclaim terminal: {error}; process result: {result:?}"),
        )),
    }
}

/// Used while the terminal is cooked. Children explicitly restore default handlers.
pub struct SignalGuard {
    saved: Vec<(libc::c_int, libc::sigaction)>,
}
impl SignalGuard {
    pub fn ignore(signals: &[libc::c_int]) -> io::Result<Self> {
        let mut guard = Self { saved: Vec::new() };
        for &signal in signals {
            // SAFETY: sigaction values are initialized before use; no Rust callback
            // is installed. Main-thread terminal ownership serializes these changes.
            unsafe {
                let mut action: libc::sigaction = std::mem::zeroed();
                let mut old: libc::sigaction = std::mem::zeroed();
                action.sa_sigaction = libc::SIG_IGN;
                libc::sigemptyset(&mut action.sa_mask);
                if libc::sigaction(signal, &action, &mut old) != 0 {
                    return Err(io::Error::last_os_error());
                }
                guard.saved.push((signal, old));
            }
        }
        Ok(guard)
    }
}
impl Drop for SignalGuard {
    fn drop(&mut self) {
        for (signal, action) in self.saved.iter().rev() {
            // SAFETY: these are the saved dispositions for the corresponding signals.
            unsafe {
                libc::sigaction(*signal, action, std::ptr::null_mut());
            }
        }
    }
}

struct Foreground {
    group: libc::pid_t,
    restored: bool,
    _signals: SignalGuard,
}
impl Foreground {
    fn capture() -> io::Result<Self> {
        // SAFETY: querying the controlling terminal does not change ownership.
        let group = unsafe { libc::tcgetpgrp(libc::STDIN_FILENO) };
        if group < 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(Self {
            group,
            restored: false,
            _signals: SignalGuard::ignore(&[libc::SIGTTOU])?,
        })
    }
    fn restore(&mut self) -> io::Result<()> {
        // SAFETY: saved group belongs to the same session. SIGTTOU is ignored
        // while Arbor is a background process reclaiming its terminal.
        if unsafe { libc::tcsetpgrp(libc::STDIN_FILENO, self.group) } != 0 {
            return Err(io::Error::last_os_error());
        }
        self.restored = true;
        Ok(())
    }
}
impl Drop for Foreground {
    fn drop(&mut self) {
        if !self.restored {
            let _ = self.restore();
        }
    }
}

pub fn wait_for_enter() -> io::Result<()> {
    use io::Write;
    print!("\r\n[Arbor] Press Enter to return...");
    io::stdout().flush()?;
    loop {
        let mut input = String::new();
        match io::stdin().read_line(&mut input) {
            Ok(_) => return Ok(()),
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            Err(error) => return Err(error),
        }
    }
}
