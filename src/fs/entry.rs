use std::{ffi::OsString, path::PathBuf};

use unicode_width::UnicodeWidthStr;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EntryKind {
    Directory,
    File,
    Symlink,
    Other,
}

#[derive(Clone, Debug)]
pub struct Entry {
    pub name: OsString,
    pub path: PathBuf,
    pub kind: EntryKind,
    label: String,
    display_width: usize,
}

impl Entry {
    pub fn new(name: OsString, path: PathBuf, kind: EntryKind) -> Self {
        let mut label = safe_text(&name.to_string_lossy());
        match kind {
            EntryKind::Directory => label.push('/'),
            EntryKind::Symlink => label.push('@'),
            _ => {}
        }
        let display_width = UnicodeWidthStr::width(label.as_str());
        Self {
            name,
            path,
            kind,
            label,
            display_width,
        }
    }

    pub fn label(&self) -> &str {
        &self.label
    }
    pub fn display_width(&self) -> usize {
        self.display_width
    }
}

/// Keep control characters in filenames from becoming terminal commands or newlines.
pub fn safe_text(text: &str) -> String {
    text.chars()
        .flat_map(|c| {
            if c.is_control() || matches!(c, '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}') {
                c.escape_default().collect::<Vec<_>>()
            } else {
                vec![c]
            }
        })
        .collect()
}
