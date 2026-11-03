use arbor::{
    app::cli::{self, Command},
    command::cwd_file::CwdFile,
};
use std::{
    ffi::OsString,
    fs,
    os::unix::{ffi::OsStringExt, fs::symlink},
    path::PathBuf,
};

#[test]
fn cwd_file_preserves_path_bytes_and_rejects_unsafe_targets() {
    let temp = tempfile::tempdir().unwrap();
    let target = tempfile::NamedTempFile::new_in(temp.path()).unwrap();
    fs::write(target.path(), b"old contents").unwrap();
    let path = PathBuf::from(OsString::from_vec(b"/tmp/a \n'\xff directory".to_vec()));
    let mut output = CwdFile::open(target.path()).unwrap();
    assert_eq!(fs::read(target.path()).unwrap(), b"old contents");
    output.write_directory(&path).unwrap();
    assert_eq!(
        fs::read(target.path()).unwrap(),
        b"/tmp/a \n'\xff directory\0"
    );
    symlink(target.path(), temp.path().join("link")).unwrap();
    assert!(CwdFile::open(&temp.path().join("link")).is_err());
    fs::hard_link(target.path(), temp.path().join("hard-link")).unwrap();
    assert!(CwdFile::open(target.path()).is_err());
    assert!(CwdFile::open(&temp.path().join("missing")).is_err());
}

#[test]
fn cli_preserves_cwd_file_option_and_paths() {
    let temp = tempfile::tempdir().unwrap();
    let Command::Browse(options) = cli::parse(
        ["--cwd-file".into(), "out file".into(), temp.path().into()],
        temp.path(),
    )
    .unwrap() else {
        panic!()
    };
    assert_eq!(options.cwd_file, Some(temp.path().join("out file")));
    assert_eq!(options.directory, temp.path().canonicalize().unwrap());
    assert!(cli::parse(["--cwd-file".into()], temp.path()).is_err());
    assert!(
        cli::parse(
            [
                "--cwd-file".into(),
                "a".into(),
                "--cwd-file".into(),
                "b".into()
            ],
            temp.path()
        )
        .is_err()
    );
}
