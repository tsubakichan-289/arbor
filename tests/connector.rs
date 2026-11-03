use arbor::layout::{
    connector::{ConnectorKind, DOWN, LEFT, LineGrid, RIGHT, Segment, UP, glyph},
    geometry::Point,
    viewport::Viewport,
};

#[test]
fn every_junction_and_rounded_corner_has_correct_glyph() {
    for (bits, expected) in [
        (UP | DOWN, '│'),
        (LEFT | RIGHT, '─'),
        (DOWN | RIGHT, '┌'),
        (DOWN | LEFT, '┐'),
        (UP | RIGHT, '└'),
        (UP | LEFT, '┘'),
        (UP | DOWN | RIGHT, '├'),
        (UP | DOWN | LEFT, '┤'),
        (LEFT | RIGHT | DOWN, '┬'),
        (LEFT | RIGHT | UP, '┴'),
        (15, '┼'),
    ] {
        assert_eq!(glyph(bits, false), expected);
    }
    assert_eq!(glyph(DOWN | RIGHT, true), '╭');
    assert_eq!(glyph(UP | RIGHT, true), '╰');
    assert_eq!(glyph(DOWN | LEFT, true), '╮');
    assert_eq!(glyph(UP | LEFT, true), '╯');
}

#[test]
fn rasterization_is_order_independent_and_clipped() {
    let viewport = Viewport::new(10, 10);
    let segments = [
        Segment::horizontal(-1_000_000, 1_000_000, 5, ConnectorKind::Path),
        Segment::vertical(5, -1_000_000, 1_000_000, ConnectorKind::PreviewBranch),
    ];
    let mut a = LineGrid::default();
    let mut b = LineGrid::default();
    for segment in segments {
        a.rasterize(segment, viewport);
    }
    for segment in segments.into_iter().rev() {
        b.rasterize(segment, viewport);
    }
    assert_eq!(a.0, b.0);
    assert_eq!(a.0[&Point::new(5, 5)], 15);
    assert_eq!(a.0.len(), 17);
}

#[test]
fn branch_endpoints_join_into_corners_and_tee() {
    let mut grid = LineGrid::default();
    let vp = Viewport::new(20, 20);
    let kind = ConnectorKind::PreviewBranch;
    for segment in [
        Segment::vertical(8, 4, 8, kind),
        Segment::horizontal(8, 12, 4, kind),
        Segment::horizontal(8, 12, 8, kind),
        Segment::horizontal(2, 8, 6, kind),
        Segment::horizontal(8, 12, 6, kind),
    ] {
        grid.rasterize(segment, vp);
    }
    assert_eq!(glyph(grid.0[&Point::new(8, 4)], false), '┌');
    assert_eq!(glyph(grid.0[&Point::new(8, 6)], false), '┼');
    assert_eq!(glyph(grid.0[&Point::new(8, 8)], false), '└');
}
