use std::{env, io, process::ExitCode};

use arbor::{
    app::{
        BrowserState,
        cli::{self, Command},
        controller::{Application, Effect},
        mode::AppMode,
    },
    command::{
        captured::CapturedProcess,
        executor::{self, ExternalRequest},
        input::CommandFocus,
        shell::Shell,
    },
    fs::scanner::Scanner,
    layout::{LayoutConfig, screen::ScreenAreas},
    ui::{
        browser::BrowserWidget,
        command_line::{CommandLineWidget, InputView},
        command_output::CommandOutputWidget,
        command_status,
        status::Status,
        terminal,
    },
};

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("arbor: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let command =
        cli::parse(env::args_os().skip(1), &env::current_dir()?).map_err(io::Error::other)?;
    let options = match command {
        Command::Help => {
            print!("{}", cli::HELP);
            return Ok(());
        }
        Command::Version => {
            println!("arbor {}", env!("CARGO_PKG_VERSION"));
            return Ok(());
        }
        Command::Browse(options) => options,
    };
    let mut scanner = Scanner;
    let mut cwd_file = options
        .cwd_file
        .as_deref()
        .map(arbor::command::cwd_file::CwdFile::open)
        .transpose()?;
    let mut app = Application::new(BrowserState::new(&options.directory, &mut scanner));
    terminal::run(|terminal| {
        let mut running: Option<CapturedProcess> = None;
        while !app.browser.quit {
            if let Some(process) = &mut running {
                let result = process.poll(&mut app.output);
                if !matches!(result, Ok(None)) {
                    let result = result.map(|status| status.expect("completed process"));
                    running = None;
                    app.finish_captured(command_status::result_message(&result), &mut scanner);
                }
            }
            terminal.draw(|frame| {
                let area = frame.area();
                let areas = ScreenAreas::new(area);
                let scene = app.browser.scene(
                    areas.browser.width,
                    areas.browser.height,
                    LayoutConfig::default(),
                );
                let mut status = Status::from_state(&app.browser);
                let footer = std::mem::take(&mut status.footer);
                if let AppMode::Command(line) = &app.mode {
                    status
                        .header
                        .push_str(if line.focus == CommandFocus::Input {
                            "  [command: Tab → files]"
                        } else {
                            "  [files: Tab → command]"
                        });
                }
                frame.render_widget(
                    BrowserWidget {
                        scene: &scene,
                        status: Some(&status),
                    },
                    areas.browser,
                );
                frame.render_widget(
                    CommandOutputWidget {
                        output: &app.output,
                        message: app.message.as_deref(),
                    },
                    areas.output,
                );
                if let AppMode::Command(line) = &app.mode
                    && !areas.input.is_empty()
                {
                    let row = areas.input;
                    frame.render_widget(CommandLineWidget { line }, row);
                    if line.focus == CommandFocus::Input {
                        frame.set_cursor_position((
                            row.x + InputView::new(line, row.width).cursor_x,
                            row.y,
                        ));
                    }
                } else {
                    let text = if running.is_some() {
                        "Running…  Ctrl+C interrupt"
                    } else {
                        &footer
                    };
                    frame.render_widget(
                        ratatui::widgets::Paragraph::new(text)
                            .style(arbor::ui::style::status(false)),
                        areas.input,
                    );
                }
            })?;
            if running.is_some() && !crossterm::event::poll(std::time::Duration::from_millis(40))? {
                continue;
            }
            let input = crossterm::event::read()?;
            if let Some(process) = &running {
                if let crossterm::event::Event::Key(key) = input
                    && key.kind != crossterm::event::KeyEventKind::Release
                    && key
                        .modifiers
                        .contains(crossterm::event::KeyModifiers::CONTROL)
                    && key.code == crossterm::event::KeyCode::Char('c')
                    && let Err(error) = process.interrupt()
                {
                    app.message = Some(format!("Cannot interrupt: {error}"));
                }
                continue;
            }
            if let Some(effect) = app.handle_event(&input, &mut scanner) {
                let external = match effect {
                    Effect::Submit(text) => app.submit(
                        &text,
                        &mut scanner,
                        env::var_os("HOME").as_deref().map(std::path::Path::new),
                    ),
                    Effect::Shell => Some(Effect::Shell),
                };
                if let Some(effect) = external {
                    let request = ExternalRequest {
                        shell: Shell::current(),
                        directory: app.browser.active().directory.clone(),
                        command: match effect {
                            Effect::Submit(text) => Some(text),
                            Effect::Shell => None,
                        },
                    };
                    app.mode = AppMode::ExternalProcess;
                    if request.command.is_some() {
                        match CapturedProcess::spawn(&request) {
                            Ok(process) => running = Some(process),
                            Err(error) => app.finish_captured(
                                command_status::result_message(&Err(error)),
                                &mut scanner,
                            ),
                        }
                        continue;
                    }
                    let result = terminal::suspend(terminal, false, || {
                        let result = executor::execute(&request, true);
                        if let Err(error) = &result
                            && error.kind() == io::ErrorKind::BrokenPipe
                        {
                            return Err(io::Error::other(error.to_string()));
                        }
                        Ok(result)
                    })?;
                    app.finish_external(command_status::result_message(&result), &mut scanner);
                }
            }
        }
        Ok(())
    })?;
    if let Some(file) = &mut cwd_file {
        file.write_directory(&app.browser.active().directory)?;
    }
    Ok(())
}
