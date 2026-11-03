use arbor::{
    app::{Action, BrowserState, event},
    fs::{
        DirectorySource,
        entry::{Entry, EntryKind},
    },
    layout::{
        LayoutConfig,
        column::{Column, ColumnRole},
        geometry::Point,
    },
};
use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

struct Tree(BTreeMap<PathBuf, Column>, usize);
impl DirectorySource for Tree {
    fn scan(&mut self, path: &Path) -> Column {
        self.1 += 1;
        self.0.get(path).unwrap().clone()
    }
}
fn tree() -> Tree {
    let mut tree = Tree(BTreeMap::new(), 0);
    for (path, children) in [
        (
            "/",
            vec![
                ("a", EntryKind::Directory),
                ("b", EntryKind::Directory),
                ("file", EntryKind::File),
                ("link", EntryKind::Symlink),
            ],
        ),
        (
            "/a",
            vec![("nested", EntryKind::Directory), ("text", EntryKind::File)],
        ),
        ("/a/nested", vec![]),
        ("/b", vec![]),
    ] {
        let dir = PathBuf::from(path);
        let entries = children
            .into_iter()
            .map(|(name, kind)| Entry::new(name.into(), dir.join(name), kind))
            .collect();
        tree.0.insert(dir.clone(), Column::new(dir, entries, None));
    }
    tree
}

#[test]
fn arrows_change_only_active_selection_and_keep_center() {
    let mut source = tree();
    let mut state = BrowserState::new(Path::new("/a"), &mut source);
    let before = state.scene(100, 24, LayoutConfig::default());
    for (action, expected) in [
        (Action::Down, 1),
        (Action::Down, 1),
        (Action::Up, 0),
        (Action::Up, 0),
    ] {
        state.apply(action, &mut source);
        assert_eq!(state.active().selected, Some(expected));
        let after = state.scene(100, 24, LayoutConfig::default());
        assert_eq!(before.viewport.offset_x, after.viewport.offset_x);
        assert!(
            after
                .nodes
                .iter()
                .filter(|n| n.selected)
                .all(|n| n.position.y == 12)
        );
        assert_eq!(
            before
                .nodes
                .iter()
                .filter(|n| n.column == 0)
                .collect::<Vec<_>>(),
            after
                .nodes
                .iter()
                .filter(|n| n.column == 0)
                .collect::<Vec<_>>()
        );
    }
}

#[test]
fn descent_promotes_preview_and_return_restores_selection() {
    let mut source = tree();
    let mut state = BrowserState::new(Path::new("/"), &mut source);
    state.apply(Action::Down, &mut source);
    let scanned = source.1;
    state.apply(Action::Right, &mut source);
    assert_eq!(source.1, scanned, "preview reused, not read again");
    assert_eq!(state.columns.len(), 2);
    assert_eq!(state.active().directory, Path::new("/b"));
    assert_eq!(state.active().selected, None);
    state.apply(Action::Up, &mut source);
    state.apply(Action::Down, &mut source);
    state.apply(Action::Right, &mut source);
    assert_eq!(state.columns.len(), 2);
    state.apply(Action::Left, &mut source);
    assert_eq!(
        state.active().selected_entry().unwrap().path,
        Path::new("/b")
    );
    assert_eq!(state.preview.as_ref().unwrap().directory, Path::new("/b"));
    state.apply(Action::Left, &mut source);
    assert_eq!(state.active().directory, Path::new("/"));
}

#[test]
fn files_and_symlinks_cannot_be_entered() {
    let mut source = tree();
    let mut state = BrowserState::new(Path::new("/"), &mut source);
    for _ in 0..2 {
        state.apply(Action::Down, &mut source);
    }
    for _ in 0..2 {
        assert!(state.preview.is_none());
        state.apply(Action::Right, &mut source);
        assert_eq!(state.columns.len(), 1);
        state.apply(Action::Down, &mut source);
    }
}

#[test]
fn left_can_go_above_initial_directory_and_back() {
    let mut source = tree();
    let mut state = BrowserState::new(Path::new("/a/nested"), &mut source);
    state.apply(Action::Left, &mut source);
    assert_eq!(state.active().directory, Path::new("/a"));
    assert_eq!(
        state.active().selected_entry().unwrap().path,
        Path::new("/a/nested")
    );
    state.apply(Action::Left, &mut source);
    assert_eq!(state.active().directory, Path::new("/"));
    state.apply(Action::Right, &mut source);
    state.apply(Action::Right, &mut source);
    assert_eq!(state.columns.len(), 3);
    assert_eq!(state.active().directory, Path::new("/a/nested"));
}

#[test]
fn startup_loads_ancestors_and_selects_the_path_to_the_active_directory() {
    let mut source = tree();
    let mut state = BrowserState::new(Path::new("/a/nested"), &mut source);
    assert_eq!(
        state
            .columns
            .iter()
            .map(|column| column.directory.as_path())
            .collect::<Vec<_>>(),
        [Path::new("/"), Path::new("/a"), Path::new("/a/nested")]
    );
    assert_eq!(
        state.columns[0].selected_entry().unwrap().path,
        Path::new("/a")
    );
    assert_eq!(
        state.columns[1].selected_entry().unwrap().path,
        Path::new("/a/nested")
    );
    assert_eq!(state.active().selected, None);
    let scene = state.scene(100, 24, LayoutConfig::default());
    let ancestors: Vec<_> = scene
        .nodes
        .iter()
        .filter(|node| node.role == ColumnRole::Ancestor && node.selected)
        .collect();
    assert_eq!(ancestors.len(), 2);
    for node in ancestors {
        let (x, y) = scene.viewport.screen(node.text_position).unwrap();
        assert!(x < 50);
        assert_eq!(y, 12);
    }
    let active = scene
        .columns
        .iter()
        .find(|column| column.role == ColumnRole::Active)
        .unwrap();
    assert_eq!(
        scene
            .viewport
            .screen(Point::new(active.x + active.width / 2, 12)),
        Some((50, 12))
    );
    let scanned = source.1;
    state.apply(Action::Left, &mut source);
    assert_eq!(state.active().directory, Path::new("/a"));
    assert_eq!(
        state.active().selected_entry().unwrap().path,
        Path::new("/a/nested")
    );
    state.apply(Action::Right, &mut source);
    assert_eq!(state.active().directory, Path::new("/a/nested"));
    assert_eq!(
        source.1, scanned,
        "initial ancestors and preview should be reused"
    );
}

#[test]
fn unreadable_ancestor_does_not_prevent_startup_or_return_to_root() {
    let mut source = tree();
    source.0.insert(
        "/a".into(),
        Column::new("/a".into(), vec![], Some("permission denied".into())),
    );
    let mut state = BrowserState::new(Path::new("/a/nested"), &mut source);
    assert_eq!(state.columns.len(), 3);
    assert_eq!(state.active().directory, Path::new("/a/nested"));
    assert!(state.columns[1].error.is_some());
    assert_eq!(state.columns[1].selected, None);
    let scene = state.scene(100, 24, LayoutConfig::default());
    assert!(
        scene
            .nodes
            .iter()
            .any(|node| node.role == ColumnRole::Ancestor && node.error)
    );
    state.apply(Action::Left, &mut source);
    assert!(state.active().error.is_some());
    state.apply(Action::Left, &mut source);
    assert_eq!(state.active().directory, Path::new("/"));
    state.apply(Action::Left, &mut source);
    assert_eq!(state.columns.len(), 1);
}

#[test]
fn all_required_keys_and_release_filter_are_mapped() {
    for (key, expected) in [
        (KeyCode::Up, Action::Up),
        (KeyCode::Down, Action::Down),
        (KeyCode::Left, Action::Left),
        (KeyCode::Right, Action::Right),
        (KeyCode::Char('q'), Action::Quit),
        (KeyCode::Esc, Action::Quit),
    ] {
        assert_eq!(
            event::action(&Event::Key(KeyEvent::new(key, KeyModifiers::NONE))),
            Some(expected)
        );
    }
    assert_eq!(
        event::action(&Event::Key(KeyEvent::new_with_kind(
            KeyCode::Down,
            KeyModifiers::NONE,
            KeyEventKind::Release
        ))),
        None
    );
    assert_eq!(event::action(&Event::Resize(80, 24)), None);
}

#[test]
fn left_and_right_keep_selection_at_screen_center_across_column_widths() {
    let mut source = tree();
    // Exercise odd/even column widths, capped labels, and the empty directory.
    for (path, name) in [
        ("/", "elevenchars"),
        ("/a", "a-file-name-that-is-longer-than-the-column-limit"),
    ] {
        let mut entries = source.0[Path::new(path)].entries().to_vec();
        entries.push(Entry::new(
            name.into(),
            Path::new(path).join(name),
            EntryKind::File,
        ));
        source
            .0
            .insert(path.into(), Column::new(path.into(), entries, None));
    }
    for (width, height) in [(1, 2), (5, 3), (80, 24), (81, 25), (160, 40)] {
        let mut state = BrowserState::new(Path::new("/"), &mut source);
        for action in [
            None,
            Some(Action::Right),
            Some(Action::Right),
            Some(Action::Left),
            Some(Action::Down),
            Some(Action::Right),
            Some(Action::Up),
            Some(Action::Left),
            Some(Action::Left),
        ] {
            if let Some(action) = action {
                state.apply(action, &mut source);
            }
            let scene = state.scene(width, height, LayoutConfig::default());
            let node = scene
                .nodes
                .iter()
                .find(|node| {
                    node.role == ColumnRole::Active && (node.selected || node.entry.is_none())
                })
                .unwrap();
            let anchor = Point::new(node.position.x + node.width / 2, node.position.y);
            assert_eq!(
                scene.viewport.screen(anchor),
                Some((width / 2, scene.viewport.center_y() as u16))
            );
            // Re-rendering after a resize must keep the same center invariant.
            let resized = state.scene(101, 31, LayoutConfig::default());
            let active = resized
                .columns
                .iter()
                .find(|column| column.role == ColumnRole::Active)
                .unwrap();
            assert_eq!(
                resized
                    .viewport
                    .screen(Point::new(active.x + active.width / 2, 15)),
                Some((50, 15))
            );
        }
    }
    let mut state = BrowserState::new(Path::new("/"), &mut source);
    assert!(
        state
            .scene(80, 24, LayoutConfig::default())
            .viewport
            .offset_x
            < 0
    );
}

#[test]
fn entering_preview_and_returning_preserve_every_rows_vertical_position() {
    for count in 0usize..=6 {
        for row_height in [1, 2, 3] {
            let mut source = tree();
            let entries = (0..count)
                .map(|i| {
                    Entry::new(
                        format!("file{i}").into(),
                        format!("/a/file{i}").into(),
                        EntryKind::File,
                    )
                })
                .collect();
            source
                .0
                .insert("/a".into(), Column::new("/a".into(), entries, None));
            let mut state = BrowserState::new(Path::new("/"), &mut source);
            let config = LayoutConfig {
                row_height,
                ..LayoutConfig::default()
            };
            let before = state.scene(120, 40, config);
            let preview_rows: Vec<_> = before
                .nodes
                .iter()
                .filter(|node| node.role == ColumnRole::Preview)
                .map(|node| (node.entry, node.position.y))
                .collect();
            state.apply(Action::Right, &mut source);
            assert_eq!(
                state.active().selected,
                count.checked_sub(1).map(|last| last / 2)
            );
            let after = state.scene(120, 40, config);
            let active_rows: Vec<_> = after
                .nodes
                .iter()
                .filter(|node| node.role == ColumnRole::Active)
                .map(|node| (node.entry, node.position.y))
                .collect();
            assert_eq!(preview_rows, active_rows);
            for node in before
                .nodes
                .iter()
                .filter(|node| node.role == ColumnRole::Active)
            {
                let ancestor = after
                    .nodes
                    .iter()
                    .find(|other| other.column == node.column && other.entry == node.entry)
                    .unwrap();
                assert_eq!(node.position, ancestor.position);
            }
            // After scrolling to the end, both leaving and re-entering retain
            // the actual displayed position instead of re-centering the list.
            for _ in 0..count {
                state.apply(Action::Down, &mut source);
            }
            let scrolled = state.scene(120, 40, config);
            let scrolled_rows: Vec<_> = scrolled
                .nodes
                .iter()
                .filter(|node| node.role == ColumnRole::Active)
                .map(|node| (node.entry, node.position.y))
                .collect();
            state.apply(Action::Left, &mut source);
            let returned = state.scene(120, 40, config);
            assert_eq!(
                scrolled_rows,
                returned
                    .nodes
                    .iter()
                    .filter(|node| node.role == ColumnRole::Preview)
                    .map(|node| (node.entry, node.position.y))
                    .collect::<Vec<_>>()
            );
            state.apply(Action::Right, &mut source);
            assert_eq!(state.active().selected, count.checked_sub(1));
            let reentered = state.scene(120, 40, config);
            assert_eq!(
                scrolled_rows,
                reentered
                    .nodes
                    .iter()
                    .filter(|node| node.role == ColumnRole::Active)
                    .map(|node| (node.entry, node.position.y))
                    .collect::<Vec<_>>()
            );
        }
    }
}
