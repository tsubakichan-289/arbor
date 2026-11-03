use arbor::{
    app::{
        Action, BrowserState,
        cli::{self, Command},
    },
    fs::{DirectorySource, entry::EntryKind, scanner::Scanner},
    layout::LayoutConfig,
};
use std::{fs, path::Path};

#[test]
fn real_scanner_sorts_directories_first_and_navigates() {
    let temp = tempfile::tempdir().unwrap();
    for dir in ["z-dir", "a-dir", "日本語"] {
        fs::create_dir(temp.path().join(dir)).unwrap();
    }
    for file in ["z.txt", "a.txt"] {
        fs::write(temp.path().join(file), "test").unwrap();
    }
    let column = Scanner.scan(temp.path());
    let names: Vec<_> = column.entries().iter().map(|e| e.label()).collect();
    assert_eq!(names, ["a-dir/", "z-dir/", "日本語/", "a.txt", "z.txt"]);
    let mut state = BrowserState::new(temp.path(), &mut Scanner);
    state.apply(Action::Right, &mut Scanner);
    assert_eq!(state.active().directory, temp.path().join("a-dir"));
    assert_eq!(state.active().selected, None);
    state.apply(Action::Left, &mut Scanner);
    assert_eq!(state.active().selected_entry().unwrap().name, "a-dir");
}

#[test]
fn missing_directory_is_a_recoverable_column_error() {
    let temp = tempfile::tempdir().unwrap();
    let column = Scanner.scan(&temp.path().join("missing"));
    assert!(column.error.is_some());
    assert_eq!(column.selected, None);
    assert!(column.entries().is_empty());
}

#[cfg(unix)]
#[test]
fn invalid_utf8_control_characters_and_symlinks_are_safe() {
    use std::{
        ffi::OsString,
        os::unix::{ffi::OsStringExt, fs::symlink},
    };
    let temp = tempfile::tempdir().unwrap();
    let raw = OsString::from_vec(b"bad-\xff".to_vec());
    let other = OsString::from_vec(b"bad-\xfe".to_vec());
    for name in [&raw, &other] {
        fs::write(temp.path().join(name), []).unwrap();
    }
    fs::write(temp.path().join("escape\x1b[2J\nname"), []).unwrap();
    fs::create_dir(temp.path().join("dir")).unwrap();
    symlink(temp.path().join("dir"), temp.path().join("link")).unwrap();
    symlink(temp.path().join("missing"), temp.path().join("broken")).unwrap();
    let column = Scanner.scan(temp.path());
    assert!(column.error.is_none());
    let bad: Vec<_> = column
        .entries()
        .iter()
        .filter(|e| e.label().starts_with("bad-"))
        .collect();
    assert_eq!(bad.len(), 2);
    assert_eq!(bad[0].label(), bad[1].label());
    assert_ne!(bad[0].path, bad[1].path);
    assert!(
        column
            .entries()
            .iter()
            .any(|e| e.name == raw && e.path == temp.path().join(&raw))
    );
    assert!(
        column
            .entries()
            .iter()
            .all(|e| !e.label().contains(['\x1b', '\n']))
    );
    assert_eq!(
        column
            .entries()
            .iter()
            .filter(|e| e.kind == EntryKind::Symlink)
            .count(),
        2
    );
    assert!(Scanner.scan(&temp.path().join("link")).error.is_some());
    assert!(
        cli::parse(["link".into()], temp.path())
            .unwrap_err()
            .contains("symbolic link")
    );
    let mut state = BrowserState::new(temp.path(), &mut Scanner);
    let scene = state.scene(40, 10, LayoutConfig::default());
    assert!(!scene.nodes.is_empty());
}

#[cfg(unix)]
#[test]
fn unreadable_directory_is_visible_and_can_be_left() {
    use std::os::unix::fs::PermissionsExt;
    let temp = tempfile::tempdir().unwrap();
    let locked = temp.path().join("locked");
    fs::create_dir(&locked).unwrap();
    fs::set_permissions(&locked, fs::Permissions::from_mode(0o000)).unwrap();
    let actual = fs::read_dir(&locked);
    let mut state = BrowserState::new(temp.path(), &mut Scanner);
    let preview_error = state.preview.as_ref().unwrap().error.is_some();
    state.apply(Action::Right, &mut Scanner);
    // Restore permissions before assertions so even a failed assertion cleans up.
    fs::set_permissions(&locked, fs::Permissions::from_mode(0o700)).unwrap();
    if actual.is_ok() {
        eprintln!(
            "permission denial cannot be produced for this privileged user; use an unprivileged test run"
        );
        return;
    }
    assert!(preview_error);
    assert!(state.navigation_error.is_some());
    assert_eq!(state.active().directory, temp.path());
    let scene = state.scene(80, 24, LayoutConfig::default());
    assert!(scene.nodes.iter().any(|n| n.error && n.label == "(error)"));
    assert_eq!(state.active().directory, temp.path());
}

#[test]
fn cli_accepts_cwd_relative_absolute_and_rejects_bad_paths() {
    let temp = tempfile::tempdir().unwrap();
    fs::create_dir(temp.path().join("child")).unwrap();
    fs::write(temp.path().join("file"), []).unwrap();
    assert_eq!(
        cli::parse([], temp.path()).unwrap(),
        Command::Browse(temp.path().canonicalize().unwrap().into())
    );
    for arg in ["child".into(), temp.path().join("child").into_os_string()] {
        assert_eq!(
            cli::parse([arg], temp.path()).unwrap(),
            Command::Browse(temp.path().join("child").canonicalize().unwrap().into())
        );
    }
    assert!(
        cli::parse(["missing".into()], temp.path())
            .unwrap_err()
            .contains("cannot open")
    );
    assert!(
        cli::parse(["file".into()], temp.path())
            .unwrap_err()
            .contains("not a directory")
    );
    assert!(cli::parse(["a".into(), "b".into()], temp.path()).is_err());
    assert_eq!(
        cli::parse(["--help".into()], Path::new("/")).unwrap(),
        Command::Help
    );
}
