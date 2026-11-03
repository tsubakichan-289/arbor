use ratatui::layout::Rect;

/// Reserve the same result/input space in every mode so selections never jump.
/// Browser's own footer row is reused as the beginning of the output region.
pub struct ScreenAreas {
    pub browser: Rect,
    pub output: Rect,
    pub input: Rect,
}
impl ScreenAreas {
    pub fn new(area: Rect) -> Self {
        let output_height = (area.height / 3).min(8);
        let browser_height = area.height.saturating_sub(output_height);
        let input_height = u16::from(area.height > 0);
        let input_y = area.bottom().saturating_sub(input_height);
        Self {
            browser: Rect::new(area.x, area.y, area.width, browser_height),
            output: Rect::new(
                area.x,
                input_y.saturating_sub(output_height),
                area.width,
                output_height,
            ),
            input: Rect::new(area.x, input_y, area.width, input_height),
        }
    }
}
