use super::{Action, BrowserState, event, mode::AppMode};
use crate::{
    command::{
        input::{CommandFocus, CommandLine},
        quoting::{insert_argument, quote_argument},
        shell::Shell,
    },
    fs::DirectorySource,
};
use crossterm::event::{Event, KeyCode, KeyEventKind, KeyModifiers};

#[derive(Debug, PartialEq, Eq)]
pub enum Effect {
    Submit(String),
    Shell,
}

pub struct Application {
    pub browser: BrowserState,
    pub mode: AppMode,
    pub message: Option<String>,
    pub output: crate::command::output::CommandOutput,
    pub history: crate::command::history::History,
}

impl Application {
    pub fn new(browser: BrowserState) -> Self {
        Self {
            browser,
            mode: AppMode::Normal,
            message: None,
            output: Default::default(),
            history: crate::command::history::History::default(),
        }
    }

    pub fn handle_event(
        &mut self,
        input: &Event,
        source: &mut impl DirectorySource,
    ) -> Option<Effect> {
        let Event::Key(key) = input else {
            return None;
        };
        if key.kind == KeyEventKind::Release {
            return None;
        }
        if matches!(self.mode, AppMode::ExternalProcess) {
            return None;
        }
        if matches!(self.mode, AppMode::Normal) {
            match key.code {
                KeyCode::Char(':') => {
                    self.mode = AppMode::Command(CommandLine::default());
                }
                KeyCode::Char('!') => {
                    self.mode = AppMode::ExternalProcess;
                    return Some(Effect::Shell);
                }
                _ => {
                    if let Some(action) = event::action(input) {
                        self.browser.apply(action, source);
                        if let Some(error) = &self.browser.navigation_error {
                            self.message = Some(error.clone());
                        }
                    }
                }
            }
            return None;
        }
        if key.code == KeyCode::Esc
            || (key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c'))
        {
            self.mode = AppMode::Normal;
            return None;
        }
        if key.modifiers.contains(KeyModifiers::CONTROL)
            && matches!(key.code, KeyCode::Char('f' | 'p' | 'd'))
        {
            let result = self.insert_selected(key.code, &Shell::current());
            self.message = result.err();
            return None;
        }
        let AppMode::Command(line) = &mut self.mode else {
            return None;
        };
        if key.code == KeyCode::Tab {
            line.toggle_focus();
            return None;
        }
        if line.focus == CommandFocus::Browser {
            if let Some(action @ (Action::Up | Action::Down | Action::Left | Action::Right)) =
                event::action(input)
            {
                self.browser.apply(action, source);
                self.message = self.browser.navigation_error.clone();
            }
            return None;
        }
        match key.code {
            KeyCode::Enter => return Some(Effect::Submit(line.text().to_owned())),
            KeyCode::Backspace => line.backspace(),
            KeyCode::Delete => line.delete(),
            KeyCode::Left => line.left(),
            KeyCode::Right => line.right(),
            KeyCode::Home => line.home(),
            KeyCode::End => line.end(),
            KeyCode::Char(c)
                if !c.is_control()
                    && !key
                        .modifiers
                        .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
            {
                line.insert(&c.to_string())
            }
            _ => {}
        }
        None
    }

    pub fn insert_selected(&mut self, key: KeyCode, shell: &Shell) -> Result<(), String> {
        let AppMode::Command(line) = &mut self.mode else {
            return Ok(());
        };
        let active = self.browser.active();
        let (value, filename) = match key {
            KeyCode::Char('d') => (active.directory.as_os_str(), false),
            KeyCode::Char('f') => (
                active
                    .selected_entry()
                    .ok_or("No item is selected")?
                    .name
                    .as_os_str(),
                true,
            ),
            KeyCode::Char('p') => (
                active
                    .selected_entry()
                    .ok_or("No item is selected")?
                    .path
                    .as_os_str(),
                false,
            ),
            _ => return Ok(()),
        };
        let quoted = quote_argument(value, filename, shell)?;
        insert_argument(line, &quoted)
    }

    /// Builtins run without suspending the TUI; external work is returned to the runner.
    pub fn submit(
        &mut self,
        text: &str,
        source: &mut impl DirectorySource,
        home: Option<&std::path::Path>,
    ) -> Option<Effect> {
        use crate::command::{
            builtin,
            parser::{self, Parsed},
        };
        self.history.record(text);
        if !text.trim().is_empty() {
            self.output.clear();
        }
        match parser::parse(text) {
            Ok(Parsed::Empty) => {
                self.mode = AppMode::Normal;
                self.message = None;
            }
            Ok(Parsed::External(command)) => {
                self.mode = AppMode::ExternalProcess;
                self.message = Some("Running…  Ctrl+C interrupt".into());
                return Some(Effect::Submit(command));
            }
            Ok(command) => match builtin::execute(&command, &mut self.browser, source, home) {
                Ok(message) => {
                    self.message = Some(message);
                    self.mode = AppMode::Command(CommandLine::default());
                }
                Err(error) => self.message = Some(error),
            },
            Err(error) => self.message = Some(error),
        }
        None
    }

    pub fn finish_external(&mut self, mut message: String, source: &mut impl DirectorySource) {
        self.mode = AppMode::Normal;
        if let Err(error) = self.browser.reload(source) {
            message.push_str(&format!("; {error}"));
        }
        self.message = Some(message);
    }

    pub fn finish_captured(&mut self, message: String, source: &mut impl DirectorySource) {
        self.finish_external(message, source);
        self.mode = AppMode::Command(CommandLine::default());
    }
}
