use std::ops::Range;

use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

use super::{
    column::{Column, ColumnRole, PlacedColumn},
    connector::{ConnectorKind, LineGrid, Segment},
    geometry::Point,
    viewport::Viewport,
};
use crate::fs::entry::EntryKind;

#[derive(Clone, Copy, Debug)]
pub struct LayoutConfig {
    pub row_height: u16,
    pub column_gap: u16,
    pub min_column_width: u16,
    pub max_column_width: u16,
    pub rounded_corners: bool,
}

impl Default for LayoutConfig {
    fn default() -> Self {
        Self {
            row_height: 2,
            column_gap: 6,
            min_column_width: 12,
            max_column_width: 32,
            rounded_corners: true,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Node {
    pub column: usize,
    pub entry: Option<usize>,
    pub position: Point,
    pub text_position: Point,
    pub width: i64,
    pub label: String,
    pub kind: Option<EntryKind>,
    pub role: ColumnRole,
    pub selected: bool,
    pub error: bool,
}

#[derive(Clone, Debug)]
pub struct LayoutResult {
    pub viewport: Viewport,
    pub columns: Vec<PlacedColumn>,
    /// Only nodes intersecting the viewport are materialized.
    pub nodes: Vec<Node>,
    pub segments: Vec<Segment>,
    pub lines: LineGrid,
    pub rounded_corners: bool,
    pub header_position: Option<Point>,
    pub footer_position: Option<Point>,
}

pub fn selected_y(center: i64, index: usize, selected: usize, h: u16) -> i64 {
    center + (index as i64 - selected as i64) * i64::from(h.max(1))
}

/// Solve top <= first + i*h < bottom without visiting offscreen entries.
pub fn visible_indices(first: i64, h: i64, count: usize, viewport: Viewport) -> Range<usize> {
    let start = (viewport.top() - first + h - 1)
        .div_euclid(h)
        .clamp(0, count as i64);
    let end = (viewport.bottom() - first + h - 1)
        .div_euclid(h)
        .clamp(start, count as i64);
    start as usize..end as usize
}

pub fn ellipsize(text: &str, width: usize) -> String {
    if UnicodeWidthStr::width(text) <= width {
        return text.to_owned();
    }
    if width == 0 {
        return String::new();
    }
    let mut result = String::new();
    let mut used = 0;
    for grapheme in text.graphemes(true) {
        let w = UnicodeWidthStr::width(grapheme);
        if used + w > width - 1 {
            break;
        }
        result.push_str(grapheme);
        used += w;
    }
    result.push('…');
    result
}

pub fn layout(
    columns: &[Column],
    preview: Option<&Column>,
    mut viewport: Viewport,
    config: LayoutConfig,
) -> LayoutResult {
    let h = i64::from(config.row_height.max(1));
    let gap = i64::from(config.column_gap.max(4));
    let min_width = usize::from(config.min_column_width.max(3));
    let max_width = usize::from(config.max_column_width).max(min_width);
    let mut placed = Vec::new();
    let mut x = 0;
    for (index, column) in columns.iter().chain(preview).enumerate() {
        let role = if index == columns.len() {
            ColumnRole::Preview
        } else if index + 1 == columns.len() {
            ColumnRole::Active
        } else {
            ColumnRole::Ancestor
        };
        let width = (column.max_display_width() + 2).clamp(min_width, max_width) as i64;
        placed.push(PlacedColumn {
            index,
            role,
            x,
            width,
        });
        x += width + gap;
    }
    if let Some(active) = placed.iter().find(|c| c.role == ColumnRole::Active) {
        viewport.focus(active.x, active.width);
    }
    let center = viewport.center_y();
    let mut nodes = Vec::new();
    for (column, position) in columns.iter().chain(preview).zip(&placed) {
        if position.x >= viewport.offset_x + i64::from(viewport.width)
            || position.x + position.width <= viewport.offset_x
        {
            continue;
        }
        // A preview retains its pending selection when promoted to active, so
        // entering or returning to a directory cannot shift its rows vertically.
        let first = selected_y(center, 0, column.selected.unwrap_or(0), config.row_height);
        let mut add_node =
            |entry: Option<usize>, y: i64, label: &str, kind: Option<EntryKind>, error: bool| {
                if y < viewport.top() || y >= viewport.bottom() {
                    return;
                }
                nodes.push(Node {
                    column: position.index,
                    entry,
                    position: Point::new(position.x, y),
                    text_position: Point::new(position.x + 1, y),
                    width: position.width,
                    label: ellipsize(label, (position.width - 2) as usize),
                    kind,
                    role: position.role,
                    selected: position.role != ColumnRole::Preview
                        && entry.is_some()
                        && entry == column.selected,
                    error,
                });
            };
        if column.entries().is_empty() {
            add_node(
                None,
                center,
                if column.error.is_some() {
                    "(error)"
                } else {
                    "(empty)"
                },
                None,
                column.error.is_some(),
            );
        } else {
            for i in visible_indices(first, h, column.entries().len(), viewport) {
                let entry = &column.entries()[i];
                add_node(
                    Some(i),
                    first + i as i64 * h,
                    entry.label(),
                    Some(entry.kind),
                    false,
                );
            }
        }
    }
    let mut segments = Vec::new();
    for (index, pair) in placed.windows(2).enumerate() {
        let parent = &pair[0];
        let child = &pair[1];
        let is_preview = child.role == ColumnRole::Preview;
        let column = if is_preview {
            preview.expect("placed preview column")
        } else {
            &columns[index + 1]
        };
        let kind = if is_preview {
            ConnectorKind::PreviewBranch
        } else {
            // Keep the selected route distinct from the surrounding fork.
            segments.push(Segment::horizontal(
                parent.x + parent.width,
                child.x - 1,
                center,
                ConnectorKind::Path,
            ));
            ConnectorKind::DirectoryBranch
        };
        let trunk = child.x - gap / 2;
        let count = column.entries().len();
        let first = selected_y(center, 0, column.selected.unwrap_or(0), config.row_height);
        let last = first + count.saturating_sub(1) as i64 * h;
        if is_preview {
            segments.push(Segment::horizontal(
                parent.x + parent.width,
                trunk,
                center,
                kind,
            ));
        }
        if first != last {
            segments.push(Segment::vertical(trunk, first, last, kind));
        }
        if count == 0 {
            segments.push(Segment::horizontal(trunk, child.x - 1, center, kind));
        } else {
            for i in visible_indices(first, h, count, viewport) {
                segments.push(Segment::horizontal(
                    trunk,
                    child.x - 1,
                    first + i as i64 * h,
                    kind,
                ));
            }
        }
    }
    let mut lines = LineGrid::default();
    for segment in &segments {
        lines.rasterize(*segment, viewport);
    }
    LayoutResult {
        viewport,
        columns: placed,
        nodes,
        segments,
        lines,
        rounded_corners: config.rounded_corners,
        header_position: (viewport.height >= 3).then_some(Point::new(viewport.offset_x, 0)),
        footer_position: (viewport.height > 0).then_some(Point::new(
            viewport.offset_x,
            i64::from(viewport.height) - 1,
        )),
    }
}
