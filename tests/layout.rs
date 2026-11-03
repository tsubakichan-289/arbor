use arbor::{
    fs::entry::{Entry, EntryKind},
    layout::{
        column::{Column, ColumnRole},
        connector::ConnectorKind,
        layout,
        placement::{LayoutConfig, ellipsize, selected_y},
        viewport::Viewport,
    },
};

fn column(count: usize, selected: usize) -> Column {
    let mut column = Column::new(
        "/virtual".into(),
        (0..count)
            .map(|i| {
                Entry::new(
                    format!("item{i}").into(),
                    format!("/virtual/item{i}").into(),
                    EntryKind::Directory,
                )
            })
            .collect(),
        None,
    );
    column.selected = (count > 0).then_some(selected);
    column
}

#[test]
fn selection_is_centered_at_both_ends_and_between() {
    for selected in [0, 1, 40, 99] {
        let scene = layout(
            &[column(100, selected)],
            None,
            Viewport::new(80, 24),
            LayoutConfig::default(),
        );
        assert_eq!(
            scene.nodes.iter().find(|n| n.selected).unwrap().position.y,
            12
        );
        assert_eq!(selected_y(12, selected, selected, 2), 12);
    }
}

#[test]
fn ancestors_and_path_do_not_move_on_selection_or_descent() {
    let mut columns = vec![column(9, 3), column(10, 7), column(8, 0)];
    let render = |columns: &[Column]| {
        layout(
            columns,
            None,
            Viewport::new(160, 40),
            LayoutConfig::default(),
        )
    };
    let before = render(&columns);
    columns[2].selected = Some(7);
    let after = render(&columns);
    assert_eq!(
        before
            .nodes
            .iter()
            .filter(|n| n.column < 2)
            .collect::<Vec<_>>(),
        after
            .nodes
            .iter()
            .filter(|n| n.column < 2)
            .collect::<Vec<_>>()
    );
    assert_eq!(
        before
            .segments
            .iter()
            .filter(|segment| segment.kind == ConnectorKind::Path)
            .collect::<Vec<_>>(),
        after
            .segments
            .iter()
            .filter(|segment| segment.kind == ConnectorKind::Path)
            .collect::<Vec<_>>()
    );
    // Only the fork entering the active column may scroll; older forks stay fixed.
    let ancestor_gap_end = before.columns[1].x;
    assert_eq!(
        before
            .segments
            .iter()
            .filter(|segment| segment.to.x < ancestor_gap_end)
            .collect::<Vec<_>>(),
        after
            .segments
            .iter()
            .filter(|segment| segment.to.x < ancestor_gap_end)
            .collect::<Vec<_>>()
    );
    columns.push(column(6, 0));
    let deeper = render(&columns);
    for node in after.nodes {
        let new = deeper
            .nodes
            .iter()
            .find(|n| n.column == node.column && n.entry == node.entry)
            .unwrap();
        assert_eq!(node.position, new.position);
    }
    assert!(
        deeper
            .segments
            .iter()
            .filter(|s| s.kind == ConnectorKind::Path)
            .all(|s| s.from.y == 20 && s.to.y == 20)
    );
}

#[test]
fn preview_aligns_a_real_middle_item_and_joins_the_parent_on_its_row() {
    use arbor::layout::{connector::glyph, geometry::Point};
    for count in 0..=6 {
        let mut preview = column(count, 0);
        preview.select_middle();
        let scene = layout(
            &[column(1, 0)],
            Some(&preview),
            Viewport::new(100, 40),
            LayoutConfig::default(),
        );
        let children: Vec<_> = scene
            .nodes
            .iter()
            .filter(|n| n.role == ColumnRole::Preview)
            .collect();
        if count == 0 {
            assert_eq!(children[0].label, "(empty)");
            assert_eq!(children[0].entry, None);
        } else {
            assert_eq!(children.len(), count);
            assert_eq!(children[0].position.y, [20, 20, 20, 18, 18, 16, 16][count]);
            assert_eq!(children[(count - 1) / 2].position.y, 20);
        }
        let trunk = scene.columns[1].x - 3;
        assert_eq!(
            glyph(scene.lines.0[&Point::new(trunk, 20)], false),
            ['─', '─', '┬', '┼', '┼', '┼', '┼'][count]
        );
    }
}

#[test]
fn lines_stay_outside_node_regions() {
    let scene = layout(
        &[column(5, 2), column(4, 1)],
        Some(&column(4, 0)),
        Viewport::new(120, 30),
        LayoutConfig::default(),
    );
    for p in scene.lines.0.keys() {
        assert!(
            !scene.nodes.iter().any(|n| p.y == n.position.y
                && p.x >= n.position.x
                && p.x < n.position.x + n.width)
        );
    }
}

#[test]
fn parent_forks_follow_active_rows_including_first_last_and_empty() {
    use arbor::layout::{connector::glyph, geometry::Point};
    for count in 0..=5 {
        for selected in 0..count.max(1) {
            let scene = layout(
                &[column(1, 0), column(count, selected)],
                None,
                Viewport::new(100, 24),
                LayoutConfig::default(),
            );
            let child = &scene.columns[1];
            let trunk = child.x - 3;
            let expected = if count <= 1 {
                '─'
            } else if selected == 0 {
                '┬'
            } else if selected + 1 == count {
                '┴'
            } else {
                '┼'
            };
            assert_eq!(
                glyph(scene.lines.0[&Point::new(trunk, 12)], false),
                expected
            );
            for node in scene.nodes.iter().filter(|node| node.column == 1) {
                assert_eq!(
                    glyph(
                        scene.lines.0[&Point::new(child.x - 1, node.position.y)],
                        false
                    ),
                    '─'
                );
                if node.position.y != 12 {
                    let expected = if node.entry == Some(0) {
                        '┌'
                    } else if node.entry == Some(count - 1) {
                        '└'
                    } else {
                        '├'
                    };
                    assert_eq!(
                        glyph(scene.lines.0[&Point::new(trunk, node.position.y)], false),
                        expected
                    );
                }
            }
            assert!(
                scene
                    .segments
                    .iter()
                    .filter(|segment| segment.kind == ConnectorKind::Path)
                    .all(|segment| segment.from.y == 12 && segment.to.y == 12)
            );
        }
    }
}

#[test]
fn unicode_truncation_respects_graphemes_and_display_width() {
    use unicode_width::UnicodeWidthStr;
    assert_eq!(ellipsize("日本語.txt", 5), "日本…");
    assert_eq!(ellipsize("e\u{301}xyz", 3), "e\u{301}x…");
    for width in 0..16 {
        assert!(
            UnicodeWidthStr::width(ellipsize("👩‍💻日本語e\u{301}abcdef", width).as_str()) <= width
        );
    }
}

#[test]
fn huge_directory_only_materializes_visible_nodes_and_lines() {
    let mut huge = column(100_000, 49_999);
    let scene = layout(
        &[column(1, 0)],
        Some(&huge),
        Viewport::new(80, 24),
        LayoutConfig::default(),
    );
    assert!(scene.nodes.len() <= 14);
    assert!(scene.segments.len() <= 15);
    assert!(scene.lines.0.len() <= 80 * 24);
    huge.selected = Some(99_999);
    let scene = layout(
        &[column(1, 0), huge],
        None,
        Viewport::new(80, 24),
        LayoutConfig::default(),
    );
    assert_eq!(
        scene.nodes.iter().find(|n| n.selected).unwrap().position.y,
        12
    );
    assert!(scene.nodes.len() <= 13);
    assert!(scene.segments.len() <= 15);
    assert!(scene.lines.0.len() <= 80 * 24);
}

#[test]
fn resize_and_horizontal_scroll_keep_active_column() {
    let columns: Vec<_> = (0..15).map(|_| column(5, 2)).collect();
    for width in [0, 1, 2, 8, 12, 80, 160] {
        for height in [0, 1, 2, 3, 24, 40] {
            let scene = layout(
                &columns,
                None,
                Viewport::new(width, height),
                LayoutConfig::default(),
            );
            let active = scene.columns.last().unwrap();
            assert_eq!(scene.columns.len(), 15);
            assert_eq!(
                active.x + active.width / 2 - scene.viewport.offset_x,
                i64::from(width / 2)
            );
            assert!(scene.lines.0.keys().all(|p| scene.viewport.contains(*p)));
            assert!(
                scene
                    .nodes
                    .iter()
                    .filter(|n| n.selected)
                    .all(|n| n.position.y == scene.viewport.center_y())
            );
        }
    }
}

#[test]
fn deep_navigation_keeps_room_for_preview_when_screen_is_wide_enough() {
    let columns: Vec<_> = (0..15).map(|_| column(5, 2)).collect();
    let scene = layout(
        &columns,
        Some(&column(3, 0)),
        Viewport::new(80, 24),
        LayoutConfig::default(),
    );
    let preview = scene.columns.last().unwrap();
    assert!(preview.x + preview.width <= scene.viewport.offset_x + 80);
    assert!(
        scene
            .nodes
            .iter()
            .any(|node| node.role == ColumnRole::Preview)
    );
}
