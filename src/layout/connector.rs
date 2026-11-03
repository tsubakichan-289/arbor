use std::collections::BTreeMap;

use super::{geometry::Point, viewport::Viewport};

pub const UP: u8 = 0b0001;
pub const DOWN: u8 = 0b0010;
pub const LEFT: u8 = 0b0100;
pub const RIGHT: u8 = 0b1000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConnectorKind {
    Path,
    /// Branches into a confirmed column, alongside the horizontal selected path.
    DirectoryBranch,
    PreviewBranch,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Segment {
    pub from: Point,
    pub to: Point,
    pub kind: ConnectorKind,
}

impl Segment {
    pub fn horizontal(x0: i64, x1: i64, y: i64, kind: ConnectorKind) -> Self {
        Self {
            from: Point::new(x0, y),
            to: Point::new(x1, y),
            kind,
        }
    }
    pub fn vertical(x: i64, y0: i64, y1: i64, kind: ConnectorKind) -> Self {
        Self {
            from: Point::new(x, y0),
            to: Point::new(x, y1),
            kind,
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct LineGrid(pub BTreeMap<Point, u8>);

impl LineGrid {
    pub fn add(&mut self, p: Point, directions: u8) {
        *self.0.entry(p).or_default() |= directions;
    }

    /// Clip before iterating. A million-row branch costs at most screen height.
    pub fn rasterize(&mut self, segment: Segment, viewport: Viewport) {
        let Segment { from, to, .. } = segment;
        if from.y == to.y {
            let lo = from.x.min(to.x);
            let hi = from.x.max(to.x);
            if from.y < viewport.top() || from.y >= viewport.bottom() {
                return;
            }
            for x in lo.max(viewport.offset_x)
                ..=hi.min(viewport.offset_x + i64::from(viewport.width) - 1)
            {
                let bits = if lo == hi {
                    LEFT | RIGHT
                } else {
                    (u8::from(x > lo) * LEFT) | (u8::from(x < hi) * RIGHT)
                };
                self.add(Point::new(x, from.y), bits);
            }
        } else if from.x == to.x {
            if from.x < viewport.offset_x || from.x >= viewport.offset_x + i64::from(viewport.width)
            {
                return;
            }
            let lo = from.y.min(to.y);
            let hi = from.y.max(to.y);
            for y in lo.max(viewport.top())..=hi.min(viewport.bottom() - 1) {
                self.add(
                    Point::new(from.x, y),
                    (u8::from(y > lo) * UP) | (u8::from(y < hi) * DOWN),
                );
            }
        }
    }
}

pub fn glyph(bits: u8, rounded: bool) -> char {
    match bits {
        1..=3 => '│',
        4 | 8 | 12 => '─',
        5 => {
            if rounded {
                '╯'
            } else {
                '┘'
            }
        }
        6 => {
            if rounded {
                '╮'
            } else {
                '┐'
            }
        }
        7 => '┤',
        9 => {
            if rounded {
                '╰'
            } else {
                '└'
            }
        }
        10 => {
            if rounded {
                '╭'
            } else {
                '┌'
            }
        }
        11 => '├',
        13 => '┴',
        14 => '┬',
        15 => '┼',
        _ => ' ',
    }
}
