use arbor::{
    app::{BrowserState, controller::Application, mode::AppMode},
    command::{
        input::CommandLine,
        quoting::{insert_argument, quote_argument},
        shell::Shell,
    },
    fs::scanner::Scanner,
};
use crossterm::event::KeyCode;
use std::{ffi::OsString, os::unix::ffi::OsStringExt, process::Command};

fn shell() -> Shell {
    Shell {
        path: "/bin/sh".into(),
    }
}

#[test]
fn shell_round_trip_preserves_metacharacters_and_never_executes_them() {
    let temp = tempfile::tempdir().unwrap();
    let marker = temp.path().join("injected");
    for name in [
        "plain".to_owned(),
        "my notes (final).md".into(),
        "it's\\a file\n日本語".into(),
        format!(
            "$(touch {}) ; echo injected | cat > {}",
            marker.display(),
            marker.display()
        ),
        "-rf".into(),
    ] {
        let quoted = quote_argument(name.as_ref(), true, &shell()).unwrap();
        let output = Command::new("/bin/sh")
            .args(["-c", &format!("printf '%s\\0' {quoted}")])
            .output()
            .unwrap();
        assert!(output.status.success());
        let expected = if name.starts_with('-') {
            format!("./{name}\0")
        } else {
            format!("{name}\0")
        };
        assert_eq!(output.stdout, expected.as_bytes());
        assert!(!marker.exists());
    }
}

#[test]
fn invalid_bytes_unsupported_shell_and_unsafe_context_are_rejected() {
    assert!(quote_argument(&OsString::from_vec(vec![0xff]), false, &shell()).is_err());
    assert!(
        quote_argument(
            "a".as_ref(),
            false,
            &Shell {
                path: "/bin/fish".into()
            }
        )
        .is_err()
    );
    for prefix in [
        "echo '",
        "echo \"",
        "echo \\",
        "echo $(",
        "cat <<EOF ",
        "# comment ",
        "echo $'",
    ] {
        let mut line = CommandLine::default();
        line.insert(prefix);
        let cursor = line.cursor();
        assert!(insert_argument(&mut line, "'x'").is_err(), "{prefix}");
        assert_eq!(line.text(), prefix);
        assert_eq!(line.cursor(), cursor);
    }
}

#[test]
fn insertion_at_cursor_preserves_suffix_and_separates_repeated_arguments() {
    let mut line = CommandLine::default();
    line.insert("diff  --color");
    line.home();
    for _ in 0..5 {
        line.right();
    }
    insert_argument(&mut line, "'one'").unwrap();
    insert_argument(&mut line, "'two'").unwrap();
    assert_eq!(line.text(), "diff 'one' 'two'  --color");
}

#[test]
fn browser_shortcuts_use_selected_original_name_path_and_current_directory() {
    let dir = tempfile::tempdir().unwrap();
    let name = "-it's a file";
    std::fs::write(dir.path().join(name), []).unwrap();
    let mut app = Application::new(BrowserState::new(dir.path(), &mut Scanner));
    app.mode = AppMode::Command(CommandLine::default());
    for key in ['f', 'p', 'd'] {
        app.insert_selected(KeyCode::Char(key), &shell()).unwrap();
    }
    let AppMode::Command(line) = &app.mode else {
        panic!()
    };
    let output = Command::new("/bin/sh")
        .args(["-c", &format!("printf '%s\\0' {}", line.text())])
        .output()
        .unwrap();
    let values: Vec<_> = output.stdout.split(|&b| b == 0).collect();
    assert_eq!(values[0], format!("./{name}").as_bytes());
    assert_eq!(
        values[1],
        dir.path().join(name).as_os_str().as_encoded_bytes()
    );
    assert_eq!(values[2], dir.path().as_os_str().as_encoded_bytes());
    std::fs::remove_file(dir.path().join(name)).unwrap();
    let mut empty = Application::new(BrowserState::new(dir.path(), &mut Scanner));
    empty.mode = AppMode::Command(CommandLine::default());
    assert!(empty.insert_selected(KeyCode::Char('f'), &shell()).is_err());
}
