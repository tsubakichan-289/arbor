use ratatui::{buffer::Buffer, layout::Rect, widgets::Widget};

use super::{canvas::Canvas, status::Status, style};
use crate::layout::{LayoutResult, connector::glyph};

pub struct BrowserWidget<'a> {
    pub scene: &'a LayoutResult,
    pub status: Option<&'a Status>,
}

impl Widget for BrowserWidget<'_> {
    fn render(self, area: Rect, buffer: &mut Buffer) {
        let mut canvas = Canvas {
            buffer,
            area,
            viewport: self.scene.viewport,
        };
        canvas.clear(style::background());
        for (point, bits) in &self.scene.lines.0 {
            let mut bytes = [0; 4];
            canvas.text(
                *point,
                glyph(*bits, self.scene.rounded_corners).encode_utf8(&mut bytes),
                style::lines(),
            );
        }
        for node in &self.scene.nodes {
            if node.selected {
                canvas.fill_row(node.position, node.width, style::node(node));
            }
        }
        for node in &self.scene.nodes {
            canvas.text(node.text_position, &node.label, style::node(node));
        }
        if let Some(status) = self.status {
            status.render(self.scene, &mut canvas);
        }
    }
}
