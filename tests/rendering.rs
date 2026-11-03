use arbor::{
    fs::entry::{Entry, EntryKind},
    layout::{
        LayoutConfig, column::Column, connector::glyph, geometry::Point, layout, viewport::Viewport,
    },
    ui::{browser::BrowserWidget, canvas::Canvas, style},
};
use ratatui::{Terminal, backend::TestBackend, buffer::Buffer, layout::Rect, widgets::Widget};

fn column(names: &[&str], selected: usize) -> Column {
    let mut column = Column::new(
        "/virtual".into(),
        names
            .iter()
            .map(|name| {
                let kind = if name.ends_with('/') {
                    EntryKind::Directory
                } else {
                    EntryKind::File
                };
                Entry::new(
                    name.trim_end_matches('/').into(),
                    format!("/virtual/{name}").into(),
                    kind,
                )
            })
            .collect(),
        None,
    );
    column.selected = (!names.is_empty()).then_some(selected);
    column
}

#[test]
fn mind_map_buffer_has_expected_branches_names_and_highlight() {
    let scene = layout(
        &[column(&["documents/", "projects/", "readme"], 1)],
        Some(&column(&["src/", "Cargo.toml", "README.md"], 1)),
        Viewport::new(80, 13),
        LayoutConfig::default(),
    );
    let mut terminal = Terminal::new(TestBackend::new(80, 13)).unwrap();
    terminal
        .draw(|frame| {
            frame.render_widget(
                BrowserWidget {
                    scene: &scene,
                    status: None,
                },
                frame.area(),
            )
        })
        .unwrap();
    let buffer = terminal.backend().buffer();
    for (y, expected) in [
        (4, " documents/    ╭── src/"),
        (5, "               │"),
        (6, " projects/  ───┼── Cargo.toml"),
        (7, "               │"),
        (8, " readme        ╰── README.md"),
    ] {
        let actual: String = (0..80).map(|x| buffer[(x, y)].symbol()).collect();
        assert_eq!(actual.trim_end(), format!("{}{expected}", " ".repeat(34)));
    }
    assert_eq!(buffer[(40, 6)].bg, style::SELECTED);
    assert_eq!(buffer[(40, 4)].bg, style::BACKGROUND);
    for (point, bits) in &scene.lines.0 {
        let (x, y) = scene.viewport.screen(*point).unwrap();
        assert_eq!(
            buffer[(x, y)].symbol(),
            glyph(*bits, scene.rounded_corners).to_string()
        );
    }
}

#[test]
fn parent_and_preview_forks_are_both_rendered_around_active_column() {
    let scene = layout(
        &[
            column(&["home/"], 0),
            column(&["documents/", "projects/", "readme"], 1),
        ],
        Some(&column(&["src/", "Cargo.toml", "README.md"], 1)),
        Viewport::new(80, 13),
        LayoutConfig::default(),
    );
    let mut buffer = Buffer::empty(Rect::new(0, 0, 80, 13));
    BrowserWidget {
        scene: &scene,
        status: None,
    }
    .render(buffer.area, &mut buffer);
    for (y, padding, expected) in [
        (4, 31, "╭── documents/    ╭── src/"),
        (5, 31, "│                 │"),
        (6, 16, " home/      ───┼── projects/  ───┼── Cargo.toml"),
        (7, 31, "│                 │"),
        (8, 31, "╰── readme        ╰── README.md"),
    ] {
        let actual: String = (0..80).map(|x| buffer[(x, y)].symbol()).collect();
        assert_eq!(
            actual.trim_end(),
            format!("{}{expected}", " ".repeat(padding))
        );
    }
    assert_eq!(buffer[(40, 6)].bg, style::SELECTED);
}

#[test]
fn all_small_terminal_sizes_and_nonzero_widget_origins_are_safe() {
    let columns: Vec<_> = (0..8)
        .map(|_| {
            column(
                &[
                    "日本語/",
                    "e\u{301}cole",
                    "very-long-name-abcdefghijklmnopqrstuvwxyz",
                ],
                2,
            )
        })
        .collect();
    for width in 0..=40 {
        for height in 0..=25 {
            let scene = layout(
                &columns,
                Some(&column(&[], 0)),
                Viewport::new(width, height),
                LayoutConfig::default(),
            );
            let area = Rect::new(3, 2, width, height);
            let mut buffer = Buffer::empty(Rect::new(0, 0, width + 6, height + 4));
            buffer[(0, 0)].set_symbol("X");
            BrowserWidget {
                scene: &scene,
                status: None,
            }
            .render(area, &mut buffer);
            assert_eq!(buffer[(0, 0)].symbol(), "X");
        }
    }
}

#[test]
fn clipping_never_writes_half_a_wide_grapheme() {
    let mut buffer = Buffer::empty(Rect::new(0, 0, 5, 1));
    let mut canvas = Canvas {
        buffer: &mut buffer,
        area: Rect::new(0, 0, 5, 1),
        viewport: Viewport {
            offset_x: 1,
            width: 5,
            height: 1,
        },
    };
    canvas.text(Point::new(0, 0), "日e\u{301}本語", style::background());
    assert_eq!(buffer[(0, 0)].symbol(), " ");
    assert_eq!(buffer[(1, 0)].symbol(), "e\u{301}");
    assert_eq!(buffer[(2, 0)].symbol(), "本");
    assert_eq!(buffer[(4, 0)].symbol(), " ");
}

#[test]
fn resize_recomputes_center_and_clears_old_content() {
    let columns = [column(&["one", "two", "three"], 1)];
    let mut terminal = Terminal::new(TestBackend::new(40, 20)).unwrap();
    for (width, height) in [(40, 20), (5, 3), (100, 40), (5, 2), (1, 1)] {
        terminal.backend_mut().resize(width, height);
        terminal.autoresize().unwrap();
        let scene = layout(
            &columns,
            None,
            Viewport::new(width, height),
            LayoutConfig::default(),
        );
        terminal
            .draw(|frame| {
                frame.render_widget(
                    BrowserWidget {
                        scene: &scene,
                        status: None,
                    },
                    frame.area(),
                )
            })
            .unwrap();
        if height == 1 {
            assert!(scene.nodes.is_empty());
            continue;
        }
        let selected = scene.nodes.iter().find(|node| node.selected).unwrap();
        assert_eq!(selected.position.y, scene.viewport.center_y());
        assert_eq!(
            terminal.backend().buffer()[(width / 2, scene.viewport.center_y() as u16)].bg,
            style::SELECTED
        );
    }
}
