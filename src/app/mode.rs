use crate::command::input::CommandLine;

#[derive(Debug, Default)]
pub enum AppMode {
    #[default]
    Normal,
    Command(CommandLine),
    ExternalProcess,
}
