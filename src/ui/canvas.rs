use ratatui::{buffer::Buffer, layout::Rect, style::Style};
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

use crate::layout::{geometry::Point, viewport::Viewport};

/// Only transforms/clips already placed cells; it never chooses node positions.
pub struct Canvas<'a> {
    pub buffer: &'a mut Buffer,
    pub area: Rect,
    pub viewport: Viewport,
}

impl Canvas<'_> {
    fn screen(&self, p: Point) -> Option<(u16, u16)> {
        let x = p.x - self.viewport.offset_x;
        if x < 0
            || p.y < 0
            || x >= i64::from(self.area.width.min(self.viewport.width))
            || p.y >= i64::from(self.area.height.min(self.viewport.height))
        {
            return None;
        }
        let x = self.area.x.checked_add(x as u16)?;
        let y = self.area.y.checked_add(p.y as u16)?;
        self.buffer.cell((x, y)).map(|_| (x, y))
    }

    pub fn clear(&mut self, style: Style) {
        let area = self.area.intersection(self.buffer.area);
        for y in area.top()..area.bottom() {
            for x in area.left()..area.right() {
                self.buffer[(x, y)].reset();
                self.buffer[(x, y)].set_style(style);
            }
        }
    }

    pub fn fill_row(&mut self, point: Point, width: i64, style: Style) {
        let first = point.x.max(self.viewport.offset_x);
        let end = (point.x + width).min(self.viewport.offset_x + i64::from(self.viewport.width));
        for x in first..end {
            if let Some(position) = self.screen(Point::new(x, point.y)) {
                self.buffer[position].set_style(style);
            }
        }
    }

    pub fn text(&mut self, mut point: Point, text: &str, style: Style) {
        for grapheme in text.graphemes(true) {
            let width = UnicodeWidthStr::width(grapheme) as i64;
            if width == 0 {
                continue;
            }
            // A wide glyph clipped on either edge is omitted as a whole.
            if let (Some((x, y)), Some(_)) = (
                self.screen(point),
                self.screen(Point::new(point.x + width - 1, point.y)),
            ) {
                self.buffer
                    .set_stringn(x, y, grapheme, width as usize, style);
            }
            point.x += width;
            if point.x >= self.viewport.offset_x + i64::from(self.viewport.width) {
                break;
            }
        }
    }
}
