use std::{fs, io, path::Path};

use super::{
    DirectorySource,
    entry::{Entry, EntryKind, safe_text},
};
use crate::layout::column::Column;

#[derive(Default)]
pub struct Scanner;

impl DirectorySource for Scanner {
    fn resolve(&mut self, directory: &Path) -> Result<std::path::PathBuf, String> {
        use std::{ffi::CString, os::unix::ffi::OsStrExt};
        let metadata = fs::symlink_metadata(directory).map_err(|error| error.to_string())?;
        if !metadata.is_dir() {
            return Err("not a directory (symlinks are not followed)".into());
        }
        let path = fs::canonicalize(directory).map_err(|error| error.to_string())?;
        let raw = CString::new(path.as_os_str().as_bytes()).map_err(|error| error.to_string())?;
        // SAFETY: raw is a live NUL-terminated path; this performs no mutation.
        if unsafe {
            libc::faccessat(
                libc::AT_FDCWD,
                raw.as_ptr(),
                libc::R_OK | libc::X_OK,
                libc::AT_EACCESS,
            )
        } != 0
        {
            return Err(io::Error::last_os_error().to_string());
        }
        fs::read_dir(&path).map_err(|error| error.to_string())?;
        Ok(path)
    }

    fn scan(&mut self, directory: &Path) -> Column {
        let mut entries = Vec::new();
        let mut error = None;
        // Do not follow a final symlink, even if it replaced a previously listed directory.
        let listing = fs::symlink_metadata(directory).and_then(|metadata| {
            if metadata.is_dir() {
                fs::read_dir(directory)
            } else {
                Err(io::Error::new(
                    io::ErrorKind::NotADirectory,
                    "not a directory (symlinks are not followed)",
                ))
            }
        });
        match listing {
            Err(err) => error = Some(safe_text(&err.to_string())),
            Ok(listing) => {
                for item in listing {
                    match item.and_then(|item| Ok((item.file_type()?, item))) {
                        Ok((file_type, item)) => {
                            let kind = if file_type.is_symlink() {
                                EntryKind::Symlink
                            } else if file_type.is_dir() {
                                EntryKind::Directory
                            } else if file_type.is_file() {
                                EntryKind::File
                            } else {
                                EntryKind::Other
                            };
                            entries.push(Entry::new(item.file_name(), item.path(), kind));
                        }
                        Err(err) => {
                            error.get_or_insert_with(|| safe_text(&err.to_string()));
                        }
                    }
                }
            }
        }
        entries.sort_by(|a, b| {
            (a.kind != EntryKind::Directory)
                .cmp(&(b.kind != EntryKind::Directory))
                .then_with(|| a.name.cmp(&b.name))
                .then_with(|| a.path.cmp(&b.path))
        });
        Column::new(directory.to_path_buf(), entries, error)
    }
}
