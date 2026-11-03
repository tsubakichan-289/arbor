use arbor::{
    app::{Action, BrowserState},
    command::{
        builtin,
        parser::{self, Parsed},
    },
    fs::scanner::Scanner,
    layout::{LayoutConfig, connector::ConnectorKind},
};
use std::{fs, path::Path};

#[test]
fn lexer_distinguishes_builtins_from_shell_programs_and_compounds() {
    for text in [
        "cdrom",
        "cd src && cargo test",
        "echo cd",
        "cd src; pwd",
        "pwd | cat",
        "cd $(echo /tmp)",
        "cd /tmp\npwd",
    ] {
        assert!(
            matches!(parser::parse(text).unwrap(), Parsed::External(_)),
            "{text}"
        );
    }
    for text in [
        "cd 'unfinished",
        "cd a b",
        "cd $HOME",
        "cd *.rs",
        "cd foo\\",
        "pwd extra",
    ] {
        assert!(parser::parse(text).is_err(), "{text}");
    }
    for (text, expected, expand_home) in [
        ("cd 'a b'", "a b", false),
        ("cd a\\ b", "a b", false),
        ("cd \"a\\\"b\"", "a\"b", false),
        ("cd ~/projects", "~/projects", true),
        ("cd '~'", "~", false),
        ("cd 'literal$*'", "literal$*", false),
    ] {
        let Parsed::Cd(Some(word)) = parser::parse(text).unwrap() else {
            panic!()
        };
        assert_eq!(word.text, expected);
        assert_eq!(word.expand_home, expand_home);
    }
}

fn cd(state: &mut BrowserState, text: &str, home: &Path) -> Result<String, String> {
    builtin::execute(&parser::parse(text)?, state, &mut Scanner, Some(home))
}

#[test]
fn builtins_share_navigation_rebuild_paths_and_preserve_failed_state() {
    let temp = tempfile::tempdir().unwrap();
    let base = temp.path().canonicalize().unwrap();
    for path in ["src", "another", "home/projects", "a b", "~"] {
        fs::create_dir_all(base.join(path)).unwrap();
    }
    let mut state = BrowserState::new(&base, &mut Scanner);
    let home = base.join("home");
    cd(&mut state, "cd src", &home).unwrap();
    assert_eq!(state.active().directory, base.join("src"));
    cd(&mut state, "cd ../another", &home).unwrap();
    assert_eq!(state.active().directory, base.join("another"));
    cd(&mut state, "cd -", &home).unwrap();
    assert_eq!(state.active().directory, base.join("src"));
    cd(&mut state, "cd .", &home).unwrap();
    cd(&mut state, "cd -", &home).unwrap();
    assert_eq!(state.active().directory, base.join("another"));
    cd(&mut state, "cd ..", &home).unwrap();
    cd(&mut state, "cd 'a b'", &home).unwrap();
    assert_eq!(state.active().directory, base.join("a b"));
    cd(&mut state, "cd", &home).unwrap();
    assert_eq!(state.active().directory, home);
    cd(&mut state, "cd ~/projects", &home).unwrap();
    assert_eq!(state.active().directory, home.join("projects"));
    let pwd = builtin::execute(&Parsed::Pwd, &mut state, &mut Scanner, None).unwrap();
    assert_eq!(pwd, home.join("projects").display().to_string());
    for pair in state.columns.windows(2) {
        assert_eq!(pair[0].selected_entry().unwrap().path, pair[1].directory);
    }
    let before = state.scene(120, 24, LayoutConfig::default());
    assert!(cd(&mut state, "cd /does/not/exist/arbor", &home).is_err());
    let after = state.scene(120, 24, LayoutConfig::default());
    assert_eq!(before.nodes, after.nodes);
    assert_eq!(before.segments, after.segments);
    assert!(
        after
            .segments
            .iter()
            .filter(|line| line.kind == ConnectorKind::Path)
            .all(|line| line.from.y == 12 && line.to.y == 12)
    );
    state.apply(Action::Left, &mut Scanner);
    cd(&mut state, "cd -", &home).unwrap();
    assert_eq!(state.active().directory, home.join("projects"));
}

#[test]
fn permission_failure_and_reload_preserve_selection_or_choose_valid_fallback() {
    use std::os::unix::fs::PermissionsExt;
    let temp = tempfile::tempdir().unwrap();
    let base = temp.path().canonicalize().unwrap();
    fs::create_dir(base.join("locked")).unwrap();
    let mut state = BrowserState::new(&base, &mut Scanner);
    for mode in [0o000, 0o400] {
        fs::set_permissions(base.join("locked"), fs::Permissions::from_mode(mode)).unwrap();
        let result = state.navigate_to(&base.join("locked"), &mut Scanner);
        fs::set_permissions(base.join("locked"), fs::Permissions::from_mode(0o700)).unwrap();
        if unsafe { libc::geteuid() } != 0 {
            assert!(result.is_err());
            assert_eq!(state.active().directory, base);
        }
    }
    for name in ["b", "c"] {
        fs::write(base.join(name), []).unwrap();
    }
    state.reload(&mut Scanner).unwrap();
    state.apply(Action::Down, &mut Scanner); // b
    assert_eq!(state.active().selected_entry().unwrap().name, "b");
    fs::write(base.join("a"), []).unwrap();
    state.reload(&mut Scanner).unwrap();
    assert_eq!(state.active().selected_entry().unwrap().name, "b");
    fs::remove_file(base.join("b")).unwrap();
    state.reload(&mut Scanner).unwrap();
    assert_eq!(state.active().selected_entry().unwrap().name, "c");
    state
        .navigate_to(&base.join("locked"), &mut Scanner)
        .unwrap();
    fs::remove_dir(base.join("locked")).unwrap();
    assert!(state.reload(&mut Scanner).is_err());
    assert_eq!(state.active().directory, base);
}
