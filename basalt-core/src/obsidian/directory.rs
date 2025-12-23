//! This module provides functionality operating with Obsidian vault folders.
use std::path::PathBuf;

/// Represents a directory within the vault.
/// TODO: Needs try_from or the like to make sure the path is a directory when creating this struct
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Directory {
    /// The directory name without the parent path.
    pub name: String,

    /// The complete filesystem path to the directory.
    pub path: PathBuf,
}
