use std::{
    fs::{File, OpenOptions},
    io::{self, Seek, Write},
    os::unix::{
        ffi::OsStrExt,
        fs::{MetadataExt, OpenOptionsExt},
    },
    path::Path,
};

/// Open once before the TUI. Never create, follow symlinks, or truncate before
/// checking the caller-provided file. Holding the fd prevents path replacement races.
pub struct CwdFile(File);
impl CwdFile {
    pub fn open(path: &Path) -> io::Result<Self> {
        let file = OpenOptions::new()
            .write(true)
            .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK | libc::O_CLOEXEC)
            .open(path)?;
        let metadata = file.metadata()?;
        // SAFETY: geteuid has no arguments and only queries the process identity.
        let uid = unsafe { libc::geteuid() };
        if !metadata.is_file()
            || metadata.uid() != uid
            || metadata.nlink() != 1
            || metadata.mode() & 0o077 != 0
        {
            return Err(io::Error::other(
                "--cwd-file must be a private, owned, single-link regular file created by the caller (use mktemp)",
            ));
        }
        Ok(Self(file))
    }
    pub fn write_directory(&mut self, directory: &Path) -> io::Result<()> {
        self.0.rewind()?;
        self.0.set_len(0)?;
        self.0.write_all(directory.as_os_str().as_bytes())?;
        self.0.write_all(&[0])?;
        self.0.flush()
    }
}
