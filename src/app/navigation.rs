use super::state::BrowserState;
use crate::{
    fs::{DirectorySource, entry::EntryKind},
    layout::column::Column,
};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    Up,
    Down,
    Left,
    Right,
    Quit,
}

impl BrowserState {
    /// All directory changes commit only after destination validation succeeds.
    pub fn navigate_to(
        &mut self,
        destination: &Path,
        source: &mut impl DirectorySource,
    ) -> Result<(), String> {
        let destination = source.resolve(destination)?;
        let previous = self.active().directory.clone();
        if destination == previous {
            return Ok(());
        }
        if let Some(index) = self
            .columns
            .iter()
            .position(|column| column.directory == destination)
        {
            let removed = self.columns.split_off(index + 1);
            self.preview = removed.into_iter().next();
        } else {
            let target = if self
                .preview
                .as_ref()
                .is_some_and(|column| column.directory == destination && column.error.is_none())
            {
                self.preview.take().expect("validated preview")
            } else {
                let column = Self::scan_preview(&destination, source);
                if let Some(error) = &column.error {
                    return Err(error.clone());
                }
                column
            };
            let paths: Vec<_> = destination
                .ancestors()
                .filter(|p| !p.as_os_str().is_empty())
                .map(Path::to_path_buf)
                .collect::<Vec<_>>()
                .into_iter()
                .rev()
                .collect();
            let mut cached: BTreeMap<_, _> = std::mem::take(&mut self.columns)
                .into_iter()
                .map(|column| (column.directory.clone(), column))
                .collect();
            for pair in paths.windows(2) {
                let mut column = cached
                    .remove(&pair[0])
                    .unwrap_or_else(|| source.scan(&pair[0]));
                column.selected = column
                    .entries()
                    .iter()
                    .position(|entry| entry.path == pair[1]);
                self.columns.push(column);
            }
            self.columns.push(target);
            self.refresh_preview(source);
        }
        self.previous_directory = Some(previous);
        self.navigation_error = None;
        Ok(())
    }

    pub fn apply(&mut self, action: Action, source: &mut impl DirectorySource) {
        self.navigation_error = None;
        match action {
            Action::Up | Action::Down => {
                let active = self.columns.last_mut().expect("active column");
                if let Some(old) = active.selected {
                    let next = if action == Action::Up {
                        old.saturating_sub(1)
                    } else {
                        (old + 1).min(active.entries().len().saturating_sub(1))
                    };
                    if old != next {
                        active.selected = Some(next);
                        self.refresh_preview(source);
                    }
                } else if !active.entries().is_empty() {
                    active.selected = Some(0);
                    self.refresh_preview(source);
                }
            }
            Action::Right => {
                if let Some(path) = self
                    .active()
                    .selected_entry()
                    .filter(|entry| entry.kind == EntryKind::Directory)
                    .map(|entry| entry.path.clone())
                {
                    self.navigation_error = self.navigate_to(&path, source).err();
                }
            }
            Action::Left => {
                if let Some(parent) = self.active().directory.parent().map(Path::to_path_buf) {
                    self.navigation_error = self.navigate_to(&parent, source).err();
                }
            }
            Action::Quit => self.quit = true,
        }
    }

    /// Refresh snapshots, retaining identities rather than old sorted indices.
    pub fn reload(&mut self, source: &mut impl DirectorySource) -> Result<(), String> {
        let directory = self.active().directory.clone();
        let mut notice = None;
        if let Err(error) = source.resolve(&directory) {
            let fallback = directory
                .ancestors()
                .skip(1)
                .find_map(|path| source.resolve(path).ok());
            if let Some(path) = fallback {
                self.navigate_to(&path, source)?;
                notice = Some(format!(
                    "Current directory unavailable ({error}); returned to {}",
                    crate::fs::entry::safe_text(&path.to_string_lossy())
                ));
            } else {
                notice = Some(error);
            }
        }
        for column in &mut self.columns {
            let mut updated = source.scan(&column.directory);
            restore_selection(column, &mut updated);
            *column = updated;
        }
        for index in 0..self.columns.len().saturating_sub(1) {
            let child: PathBuf = self.columns[index + 1].directory.clone();
            self.columns[index].selected = self.columns[index]
                .entries()
                .iter()
                .position(|entry| entry.path == child);
        }
        let old_preview = self.preview.take();
        self.refresh_preview(source);
        if let (Some(old), Some(new)) = (old_preview, &mut self.preview)
            && old.directory == new.directory
        {
            restore_selection(&old, new);
        }
        notice.map_or(Ok(()), Err)
    }
}

fn restore_selection(old: &Column, new: &mut Column) {
    new.selected = old
        .selected_entry()
        .and_then(|selected| {
            new.entries()
                .iter()
                .position(|entry| entry.path == selected.path)
        })
        .or_else(|| {
            (!new.entries().is_empty())
                .then(|| old.selected.unwrap_or(0).min(new.entries().len() - 1))
        });
}
