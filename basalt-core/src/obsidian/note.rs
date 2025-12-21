//! This module provides functionality operating with Obsidian notes.
use std::path::PathBuf;

/// Represents a single note (Markdown file) within a vault.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Note {
    /// The base filename without `.md` extension.
    pub name: String,

    /// Filesystem path to the `.md` file.
    pub path: PathBuf,
}
