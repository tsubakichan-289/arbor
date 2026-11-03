pub mod entry;
pub mod scanner;

use crate::layout::column::Column;
use std::path::Path;

/// Navigation can use a real scanner or a deterministic in-memory tree.
pub trait DirectorySource {
    fn scan(&mut self, directory: &Path) -> Column;
    fn resolve(&mut self, directory: &Path) -> Result<std::path::PathBuf, String> {
        Ok(directory.to_path_buf())
    }
}
