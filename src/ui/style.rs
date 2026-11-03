use ratatui::style::{Color, Modifier, Style};

use crate::{
    fs::entry::EntryKind,
    layout::{column::ColumnRole, placement::Node},
};

pub const BACKGROUND: Color = Color::Rgb(17, 23, 31);
pub const FOREGROUND: Color = Color::Rgb(207, 216, 227);
pub const ACCENT: Color = Color::Rgb(107, 212, 184);
pub const CONNECTOR: Color = Color::Rgb(78, 112, 148);
pub const SELECTED: Color = Color::Rgb(37, 78, 105);
pub const PATH: Color = Color::Rgb(28, 43, 57);
pub const MUTED: Color = Color::Rgb(133, 150, 168);
pub const ERROR: Color = Color::Rgb(245, 152, 140);

pub fn background() -> Style {
    Style::default().bg(BACKGROUND).fg(FOREGROUND)
}
pub fn lines() -> Style {
    background().fg(CONNECTOR)
}
pub fn header() -> Style {
    background().fg(ACCENT).add_modifier(Modifier::BOLD)
}
pub fn status(error: bool) -> Style {
    background().fg(if error { ERROR } else { MUTED })
}

pub fn node(node: &Node) -> Style {
    let mut style = background().fg(if node.error {
        ERROR
    } else {
        match node.kind {
            Some(EntryKind::Directory) => ACCENT,
            Some(EntryKind::Symlink) => Color::Rgb(196, 165, 230),
            None => MUTED,
            _ => FOREGROUND,
        }
    });
    if node.selected {
        style = style
            .bg(if node.role == ColumnRole::Active {
                SELECTED
            } else {
                PATH
            })
            .add_modifier(Modifier::BOLD);
    }
    style
}
