use super::style;
use crate::{command::output::CommandOutput, fs::entry::safe_text};
use ratatui::{buffer::Buffer, layout::Rect, widgets::Widget};
use std::collections::VecDeque;
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

pub struct CommandOutputWidget<'a> {
    pub output: &'a CommandOutput,
    pub message: Option<&'a str>,
}
impl Widget for CommandOutputWidget<'_> {
    fn render(self, area: Rect, buffer: &mut Buffer) {
        let area = area.intersection(buffer.area);
        if area.is_empty() {
            return;
        }
        for y in area.top()..area.bottom() {
            for x in area.left()..area.right() {
                buffer[(x, y)].reset();
                buffer[(x, y)].set_style(style::background());
            }
        }
        let rows = usize::from(area.height.saturating_sub(1));
        let mut lines = VecDeque::new();
        for line in self.output.text().trim_end_matches('\n').split('\n') {
            let mut row = String::new();
            let mut width = 0;
            for grapheme in line.graphemes(true) {
                let size = grapheme.width();
                if width + size > usize::from(area.width) {
                    lines.push_back(std::mem::take(&mut row));
                    if lines.len() > rows {
                        lines.pop_front();
                    }
                    width = 0;
                }
                if size <= usize::from(area.width) {
                    row.push_str(grapheme);
                    width += size;
                }
            }
            lines.push_back(row);
            if lines.len() > rows {
                lines.pop_front();
            }
        }
        for (i, line) in lines.iter().enumerate() {
            buffer.set_stringn(
                area.x,
                area.y + i as u16,
                line,
                usize::from(area.width),
                style::background(),
            );
        }
        let message = self
            .message
            .unwrap_or("Command output  |  : input  ! interactive shell");
        let message = if self.output.truncated {
            format!("[tail 64 KiB] {message}")
        } else {
            message.to_owned()
        };
        buffer.set_stringn(
            area.x,
            area.bottom() - 1,
            safe_text(&message),
            usize::from(area.width),
            style::status(false),
        );
    }
}
