use super::geometry::Point;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Viewport {
    pub offset_x: i64,
    pub width: u16,
    pub height: u16,
}

impl Viewport {
    pub fn new(width: u16, height: u16) -> Self {
        Self {
            width,
            height,
            offset_x: 0,
        }
    }
    pub fn center_y(self) -> i64 {
        (self.top() + self.bottom()) / 2
    }
    pub fn top(self) -> i64 {
        i64::from(self.height >= 3)
    }
    pub fn bottom(self) -> i64 {
        i64::from(self.height.saturating_sub(1))
    }
    pub fn contains(self, p: Point) -> bool {
        p.x >= self.offset_x
            && p.x < self.offset_x + i64::from(self.width)
            && p.y >= self.top()
            && p.y < self.bottom()
    }
    pub fn screen(self, p: Point) -> Option<(u16, u16)> {
        self.contains(p)
            .then(|| ((p.x - self.offset_x) as u16, p.y as u16))
    }
    pub fn focus(&mut self, x: i64, width: i64) {
        // Anchor the middle cell of the active highlight to the screen center.
        // Negative offsets provide blank space to the left at the starting column.
        self.offset_x = x + width / 2 - i64::from(self.width / 2);
    }
}
