use crate::fs::entry::safe_text;
use std::{
    ffi::OsString,
    path::{Path, PathBuf},
};

pub const HELP: &str = "Arbor — mind-map terminal file browser\n\nUsage: arbor [--cwd-file FILE] [DIRECTORY]\n       arbor --help\n       arbor --version\n\n↑/↓ select   ← parent   → enter   : command   ! shell   q/Esc quit\nCommand: Tab focus   Ctrl+F name   Ctrl+P path   Ctrl+D directory\n--cwd-file requires an existing private temporary file; writes path + NUL on success.\nDirectory symlinks are not traversed.\n";

#[derive(Debug, PartialEq, Eq)]
pub struct BrowseOptions {
    pub directory: PathBuf,
    pub cwd_file: Option<PathBuf>,
}
impl From<PathBuf> for BrowseOptions {
    fn from(directory: PathBuf) -> Self {
        Self {
            directory,
            cwd_file: None,
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum Command {
    Browse(BrowseOptions),
    Help,
    Version,
}

pub fn parse(args: impl IntoIterator<Item = OsString>, cwd: &Path) -> Result<Command, String> {
    let mut args = args.into_iter();
    let mut directory = None;
    let mut cwd_file = None;
    let mut operands = false;
    while let Some(arg) = args.next() {
        if !operands {
            match arg.to_str() {
                Some("--") => {
                    operands = true;
                    continue;
                }
                Some("--help" | "-h") => return Ok(Command::Help),
                Some("--version" | "-V") => return Ok(Command::Version),
                Some("--cwd-file") => {
                    if cwd_file.is_some() {
                        return Err("--cwd-file was specified more than once".into());
                    }
                    let path = PathBuf::from(args.next().ok_or("--cwd-file requires a file path")?);
                    if path.as_os_str().is_empty() {
                        return Err("--cwd-file requires a nonempty path".into());
                    }
                    cwd_file = Some(if path.is_absolute() {
                        path
                    } else {
                        cwd.join(path)
                    });
                    continue;
                }
                Some(option) if option.starts_with('-') => {
                    return Err(format!(
                        "unknown option: {} (use --help)",
                        safe_text(option)
                    ));
                }
                _ => {}
            }
        }
        if directory.replace(PathBuf::from(arg)).is_some() {
            return Err("expected at most one directory (use --help)".into());
        }
    }
    let path = directory.unwrap_or_else(|| cwd.to_path_buf());
    let path = if path.is_absolute() {
        path
    } else {
        cwd.join(path)
    };
    let display = safe_text(&path.to_string_lossy());
    // An unreadable starting directory is still shown as an error column.
    let metadata = std::fs::symlink_metadata(&path)
        .map_err(|err| format!("cannot open '{display}': {err}"))?;
    if metadata.is_symlink() {
        return Err(format!(
            "'{display}' is a symbolic link; directory links are not followed"
        ));
    }
    if !metadata.is_dir() {
        return Err(format!("'{display}' is not a directory"));
    }
    let directory =
        std::fs::canonicalize(&path).map_err(|err| format!("cannot open '{display}': {err}"))?;
    Ok(Command::Browse(BrowseOptions {
        directory,
        cwd_file,
    }))
}
