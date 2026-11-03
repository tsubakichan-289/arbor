use std::path::PathBuf;

use crate::fs::entry::Entry;

#[derive(Clone, Debug)]
pub struct Column {
    pub directory: PathBuf,
    entries: Vec<Entry>,
    pub selected: Option<usize>,
    pub error: Option<String>,
    max_display_width: usize,
}

impl Column {
    /// Align a real middle item with its parent; choose the upper one for even lists.
    pub fn select_middle(&mut self) {
        self.selected = self.entries.len().checked_sub(1).map(|last| last / 2);
    }

    pub fn new(directory: PathBuf, entries: Vec<Entry>, error: Option<String>) -> Self {
        let max_display_width = entries.iter().map(Entry::display_width).max().unwrap_or(7);
        let selected = (!entries.is_empty()).then_some(0);
        Self {
            directory,
            entries,
            selected,
            error,
            max_display_width,
        }
    }

    pub fn entries(&self) -> &[Entry] {
        &self.entries
    }
    pub fn selected_entry(&self) -> Option<&Entry> {
        self.selected.and_then(|i| self.entries.get(i))
    }
    pub fn max_display_width(&self) -> usize {
        self.max_display_width
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ColumnRole {
    Ancestor,
    Active,
    Preview,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlacedColumn {
    pub index: usize,
    pub role: ColumnRole,
    pub x: i64,
    pub width: i64,
}
