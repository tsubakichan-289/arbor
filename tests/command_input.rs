use arbor::{
    app::{
        BrowserState,
        controller::{Application, Effect},
        mode::AppMode,
    },
    command::input::{CommandFocus, CommandLine},
    fs::scanner::Scanner,
    layout::LayoutConfig,
    ui::command_line::InputView,
};
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};

fn key(code: KeyCode) -> Event {
    Event::Key(KeyEvent::new(code, KeyModifiers::NONE))
}

#[test]
fn unicode_editor_moves_and_deletes_whole_graphemes() {
    let mut line = CommandLine::default();
    line.insert("Ae\u{301}日本👩‍💻Z");
    line.left();
    line.backspace();
    assert_eq!(line.text(), "Ae\u{301}日本Z");
    line.home();
    line.right();
    line.delete();
    assert_eq!(line.text(), "A日本Z");
    line.insert("中");
    assert_eq!(line.text(), "A中日本Z");
    line.end();
    line.delete();
    line.home();
    line.backspace();
    assert_eq!(line.text(), "A中日本Z");
    line.insert("e");
    line.insert("\u{301}");
    line.backspace();
    assert_eq!(line.text(), "A中日本Z");
    for backspace in [false, true] {
        let mut flag = CommandLine::default();
        flag.insert("🇯x🇵");
        flag.home();
        flag.right();
        if backspace {
            flag.right();
            flag.backspace();
        } else {
            flag.delete();
        }
        assert_eq!(flag.text(), "🇯🇵");
        assert_eq!(flag.cursor(), flag.text().len());
        flag.backspace();
        assert_eq!(flag.text(), "");
    }
}

#[test]
fn input_view_keeps_cursor_visible_at_every_width() {
    let mut line = CommandLine::default();
    line.insert("echo 日本語 e\u{301} 👩‍💻 longlonglong");
    for width in 1..50 {
        for _ in 0..30 {
            let view = InputView::new(&line, width);
            assert!(view.cursor_x < width);
            assert!(
                unicode_width::UnicodeWidthStr::width(view.text.as_str()) <= usize::from(width)
            );
            line.left();
        }
        line.end();
    }
}

#[test]
fn mode_focus_and_navigation_preserve_input_and_layout() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir(dir.path().join("child")).unwrap();
    std::fs::write(dir.path().join("file"), []).unwrap();
    let mut app = Application::new(BrowserState::new(dir.path(), &mut Scanner));
    let before = app.browser.scene(100, 24, LayoutConfig::default());
    app.handle_event(&key(KeyCode::Char(':')), &mut Scanner);
    for c in "diff ".chars() {
        app.handle_event(&key(KeyCode::Char(c)), &mut Scanner);
    }
    app.handle_event(&key(KeyCode::Left), &mut Scanner);
    app.handle_event(&key(KeyCode::Tab), &mut Scanner);
    assert_eq!(app.handle_event(&key(KeyCode::Enter), &mut Scanner), None);
    app.handle_event(&key(KeyCode::Char('q')), &mut Scanner);
    assert!(!app.browser.quit);
    app.handle_event(&key(KeyCode::Right), &mut Scanner);
    assert_eq!(app.browser.active().directory, dir.path().join("child"));
    let AppMode::Command(line) = &app.mode else {
        panic!()
    };
    assert_eq!(line.text(), "diff ");
    assert_eq!(line.cursor(), 4);
    assert_eq!(line.focus, CommandFocus::Browser);
    app.handle_event(&key(KeyCode::Left), &mut Scanner);
    let after = app.browser.scene(100, 24, LayoutConfig::default());
    assert_eq!(before.nodes, after.nodes);
    app.handle_event(&key(KeyCode::Tab), &mut Scanner);
    assert_eq!(
        app.handle_event(&key(KeyCode::Enter), &mut Scanner),
        Some(Effect::Submit("diff ".into()))
    );
    app.handle_event(&key(KeyCode::Esc), &mut Scanner);
    assert!(matches!(app.mode, AppMode::Normal));
    assert!(!app.browser.quit);
}

#[test]
fn builtin_errors_cancel_and_external_lifecycle_are_explicit_states() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = Application::new(BrowserState::new(dir.path(), &mut Scanner));
    app.handle_event(&key(KeyCode::Char(':')), &mut Scanner);
    assert!(
        app.submit(
            "cd /arbor-missing-directory",
            &mut Scanner,
            Some(dir.path())
        )
        .is_none()
    );
    assert!(matches!(app.mode, AppMode::Command(_)));
    assert!(app.message.is_some());
    app.handle_event(
        &Event::Key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL)),
        &mut Scanner,
    );
    assert!(matches!(app.mode, AppMode::Normal));
    app.handle_event(&key(KeyCode::Char(':')), &mut Scanner);
    app.submit("pwd", &mut Scanner, None);
    assert!(matches!(app.mode, AppMode::Command(_)));
    assert!(
        app.message
            .as_ref()
            .unwrap()
            .contains(&dir.path().display().to_string())
    );
    app.handle_event(&key(KeyCode::Char(':')), &mut Scanner);
    assert_eq!(
        app.submit("echo test", &mut Scanner, None),
        Some(Effect::Submit("echo test".into()))
    );
    assert!(matches!(app.mode, AppMode::ExternalProcess));
    assert_eq!(
        app.handle_event(&key(KeyCode::Char('q')), &mut Scanner),
        None
    );
    assert!(!app.browser.quit);
    std::fs::write(dir.path().join("new-file"), []).unwrap();
    app.finish_external("status 0".into(), &mut Scanner);
    assert!(matches!(app.mode, AppMode::Normal));
    assert!(
        app.browser
            .active()
            .entries()
            .iter()
            .any(|entry| entry.name == "new-file")
    );
    assert_eq!(
        app.handle_event(&key(KeyCode::Char('!')), &mut Scanner),
        Some(Effect::Shell)
    );
    assert!(matches!(app.mode, AppMode::ExternalProcess));
}

#[test]
fn command_footer_never_overlaps_browser_nodes_even_on_tiny_screens() {
    use arbor::ui::command_line::CommandLineWidget;
    use ratatui::{buffer::Buffer, layout::Rect, widgets::Widget};
    let dir = tempfile::tempdir().unwrap();
    let mut browser = BrowserState::new(dir.path(), &mut Scanner);
    let mut line = CommandLine::default();
    line.insert("echo 日本語e\u{301}abcdefghijklmnop");
    for width in 0..=40 {
        for height in 0..=8 {
            let scene = browser.scene(width, height, LayoutConfig::default());
            let mut buffer = Buffer::empty(Rect::new(0, 0, width, height));
            if height > 0 {
                assert!(
                    scene
                        .nodes
                        .iter()
                        .all(|node| node.position.y < i64::from(height - 1))
                );
                CommandLineWidget { line: &line }
                    .render(Rect::new(0, height - 1, width, 1), &mut buffer);
            }
        }
    }
}
