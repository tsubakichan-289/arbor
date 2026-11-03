use super::style;
use crate::{
    command::input::{CommandFocus, CommandLine},
    fs::entry::safe_text,
};
use ratatui::{buffer::Buffer, layout::Rect, widgets::Widget};
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

/// A display-only projection: byte cursor -> grapheme-safe, horizontally scrolled cells.
pub struct InputView {
    pub text: String,
    pub cursor_x: u16,
}

impl InputView {
    pub fn new(line: &CommandLine, width: u16) -> Self {
        if width == 0 {
            return Self {
                text: String::new(),
                cursor_x: 0,
            };
        }
        let prefix = if width > 2 { ": " } else { "" };
        let available = usize::from(width) - prefix.len();
        let before = &line.text()[..line.cursor()];
        let mut start = 0;
        let mut cursor_width = UnicodeWidthStr::width(safe_text(before).as_str());
        for (index, grapheme) in before.grapheme_indices(true) {
            if cursor_width < available {
                break;
            }
            cursor_width =
                cursor_width.saturating_sub(UnicodeWidthStr::width(safe_text(grapheme).as_str()));
            start = index + grapheme.len();
        }
        let mut text = prefix.to_owned();
        let mut used = 0;
        for grapheme in line.text()[start..].graphemes(true) {
            let displayed = safe_text(grapheme);
            let size = UnicodeWidthStr::width(displayed.as_str());
            if used + size > available {
                break;
            }
            text.push_str(&displayed);
            used += size;
        }
        Self {
            text,
            cursor_x: (prefix.len() + cursor_width) as u16,
        }
    }
}

pub struct CommandLineWidget<'a> {
    pub line: &'a CommandLine,
}
impl Widget for CommandLineWidget<'_> {
    fn render(self, area: Rect, buffer: &mut Buffer) {
        let area = area.intersection(buffer.area);
        if area.is_empty() {
            return;
        }
        let style = if self.line.focus == CommandFocus::Input {
            style::header()
        } else {
            style::status(false)
        };
        for x in area.left()..area.right() {
            buffer[(x, area.y)].reset();
            buffer[(x, area.y)].set_style(style);
        }
        buffer.set_stringn(
            area.x,
            area.y,
            InputView::new(self.line, area.width).text,
            area.width as usize,
            style,
        );
    }
}
