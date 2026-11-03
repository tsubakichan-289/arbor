use std::path::Path;

use crate::{
    fs::{DirectorySource, entry::EntryKind},
    layout::{LayoutConfig, LayoutResult, column::Column, layout, viewport::Viewport},
};

#[derive(Debug)]
pub struct BrowserState {
    pub columns: Vec<Column>,
    pub preview: Option<Column>,
    pub viewport: Viewport,
    pub quit: bool,
    pub previous_directory: Option<std::path::PathBuf>,
    pub navigation_error: Option<String>,
}

impl BrowserState {
    pub fn new(directory: &Path, source: &mut impl DirectorySource) -> Self {
        let mut columns = vec![source.scan(directory)];
        for parent in directory
            .ancestors()
            .skip(1)
            .filter(|path| !path.as_os_str().is_empty())
        {
            let child = &columns.last().expect("starting column").directory;
            let mut column = source.scan(parent);
            column.selected = column
                .entries()
                .iter()
                .position(|entry| &entry.path == child);
            columns.push(column);
        }
        columns.reverse();
        let mut state = Self {
            columns,
            preview: None,
            viewport: Viewport::default(),
            quit: false,
            previous_directory: None,
            navigation_error: None,
        };
        state.refresh_preview(source);
        state
    }

    pub fn active(&self) -> &Column {
        self.columns
            .last()
            .expect("a browser always has an active column")
    }

    pub(crate) fn refresh_preview(&mut self, source: &mut impl DirectorySource) {
        self.preview = self
            .active()
            .selected_entry()
            .filter(|entry| entry.kind == EntryKind::Directory)
            .map(|entry| Self::scan_preview(&entry.path, source));
    }

    pub(crate) fn scan_preview(directory: &Path, source: &mut impl DirectorySource) -> Column {
        let mut column = source.scan(directory);
        column.select_middle();
        column
    }

    pub fn scene(&mut self, width: u16, height: u16, config: LayoutConfig) -> LayoutResult {
        self.viewport.width = width;
        self.viewport.height = height;
        let scene = layout(&self.columns, self.preview.as_ref(), self.viewport, config);
        self.viewport = scene.viewport;
        scene
    }
}
