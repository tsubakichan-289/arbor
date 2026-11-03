use std::{
    env,
    ffi::OsString,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
};

#[derive(Clone, Debug)]
pub struct Shell {
    pub path: PathBuf,
}

impl Shell {
    pub fn current() -> Self {
        Self::resolve(env::var_os("SHELL"))
    }
    pub fn resolve(preferred: Option<OsString>) -> Self {
        let path = preferred
            .and_then(|name| {
                if name.is_empty() {
                    return None;
                }
                let path = PathBuf::from(&name);
                if path.is_absolute() {
                    return executable(&path).then_some(path);
                }
                env::split_paths(&env::var_os("PATH").unwrap_or_default())
                    .map(|dir| dir.join(&name))
                    .find(|path| executable(path))
            })
            .unwrap_or_else(|| PathBuf::from("/bin/sh"));
        Self { path }
    }
    pub fn supports_posix_quoting(&self) -> bool {
        matches!(
            self.path.file_name().and_then(|s| s.to_str()),
            Some("sh" | "bash" | "dash" | "zsh" | "ksh" | "ash")
        )
    }
}

fn executable(path: &Path) -> bool {
    path.metadata()
        .is_ok_and(|metadata| metadata.is_file() && metadata.permissions().mode() & 0o111 != 0)
}
