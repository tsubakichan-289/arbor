use arbor::{
    app::{BrowserState, controller::Application, mode::AppMode},
    command::{
        captured::CapturedProcess, executor::ExternalRequest, output::CommandOutput, shell::Shell,
    },
    fs::scanner::Scanner,
    layout::{LayoutConfig, screen::ScreenAreas},
    ui::{command_line::CommandLineWidget, command_output::CommandOutputWidget},
};
use ratatui::{buffer::Buffer, layout::Rect, widgets::Widget};
use std::{
    thread,
    time::{Duration, Instant},
};

fn request(text: &str) -> ExternalRequest {
    ExternalRequest {
        shell: Shell {
            path: "/bin/sh".into(),
        },
        directory: "/tmp".into(),
        command: Some(text.into()),
    }
}
fn wait(process: &mut CapturedProcess, output: &mut CommandOutput) -> std::process::ExitStatus {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        if let Some(status) = process.poll(output).unwrap() {
            return status;
        }
        assert!(Instant::now() < deadline, "command did not finish");
        thread::sleep(Duration::from_millis(5));
    }
}

#[test]
fn captured_output_streams_both_channels_and_preserves_status_and_cwd() {
    let mut process =
        CapturedProcess::spawn(&request("printf first; printf second >&2; pwd; exit 7")).unwrap();
    let mut output = CommandOutput::default();
    assert_eq!(wait(&mut process, &mut output).code(), Some(7));
    assert_eq!(output.text(), "firstsecond/tmp\n");
    let mut bad = request("true");
    bad.shell.path = "/arbor-no-such-shell".into();
    assert!(CapturedProcess::spawn(&bad).is_err());
}

#[test]
fn streaming_can_be_interrupted_without_waiting_for_completion() {
    let mut process = CapturedProcess::spawn(&request("printf ready; sleep 30")).unwrap();
    let mut output = CommandOutput::default();
    let deadline = Instant::now() + Duration::from_secs(5);
    while output.is_empty() {
        assert!(process.poll(&mut output).unwrap().is_none());
        assert!(Instant::now() < deadline);
        thread::sleep(Duration::from_millis(5));
    }
    assert_eq!(output.text(), "ready");
    process.interrupt().unwrap();
    assert!(!wait(&mut process, &mut output).success());
}

#[test]
fn logs_are_bounded_and_strip_terminal_controls_and_preserve_split_utf8() {
    let mut output = CommandOutput::default();
    let text = "日本語".as_bytes();
    output.append(&text[..2]);
    output.append(&text[2..]);
    output.append(b"\x1b[31m red\x1b[0m\x1b]52;c;secret\x07\n");
    assert_eq!(output.text(), "日本語 red\n");
    output.append(&vec![b'x'; CommandOutput::LIMIT * 3]);
    assert_eq!(output.len(), CommandOutput::LIMIT);
    assert!(output.truncated);
    output.clear();
    assert!(output.is_empty());
    assert!(!output.truncated);
    let mut process =
        CapturedProcess::spawn(&request("head -c 400000 /dev/zero; printf final")).unwrap();
    assert!(wait(&mut process, &mut output).success());
    assert!(output.text().ends_with("final"));
    assert_eq!(output.len(), CommandOutput::LIMIT);
}

#[test]
fn results_are_above_input_and_browser_nodes_never_enter_the_reserved_area() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("file"), []).unwrap();
    let mut app = Application::new(BrowserState::new(dir.path(), &mut Scanner));
    for width in 0..=45 {
        for height in 0..=30 {
            let area = Rect::new(2, 3, width, height);
            let areas = ScreenAreas::new(area);
            let scene = app
                .browser
                .scene(width, areas.browser.height, LayoutConfig::default());
            assert!(
                scene
                    .nodes
                    .iter()
                    .all(|node| node.position.y + i64::from(area.y) < i64::from(areas.output.y))
            );
            assert_eq!(areas.output.bottom(), areas.input.y);
            let mut output = CommandOutput::default();
            output.append("old\n日本語\nlast-result".as_bytes());
            let mut buffer = Buffer::empty(area);
            CommandOutputWidget {
                output: &output,
                message: Some("status 0"),
            }
            .render(areas.output, &mut buffer);
            CommandLineWidget {
                line: &Default::default(),
            }
            .render(areas.input, &mut buffer);
            if height >= 15 && width >= 20 {
                let rendered: String = buffer.content.iter().map(|c| c.symbol()).collect();
                let result = rendered.find("last-result").unwrap();
                let status = rendered.find("status 0").unwrap();
                let prompt = rendered.rfind(": ").unwrap();
                assert!(result < status && status < prompt);
            }
        }
    }
    app.submit("pwd", &mut Scanner, None);
    assert!(matches!(app.mode, AppMode::Command(_)));
    assert_eq!(app.message.as_deref(), Some(dir.path().to_str().unwrap()));
    app.output.append(b"result remains");
    app.finish_captured("status 0".into(), &mut Scanner);
    assert_eq!(app.output.text(), "result remains");
    assert!(matches!(app.mode, AppMode::Command(_)));
}
