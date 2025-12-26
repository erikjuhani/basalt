//! This module provides functionality operating with Obsidian notes.
use std::path::{Path, PathBuf};

use crate::obsidian::Error;

/// Represents a single note (Markdown file) within a vault.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Note {
    name: String,
    path: PathBuf,
}

impl Note {
    /// The base filename without `.md` extension.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Filesystem path to the `.md` file.
    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl TryFrom<(&str, PathBuf)> for Note {
    type Error = Error;
    fn try_from((name, path): (&str, PathBuf)) -> Result<Self, Self::Error> {
        match path.is_file() {
            true => Ok(Self {
                name: name.to_string(),
                path,
            }),
            false => Err(Error::Io(std::io::ErrorKind::IsADirectory.into())),
        }
    }
}

impl TryFrom<(String, PathBuf)> for Note {
    type Error = Error;
    fn try_from((name, path): (String, PathBuf)) -> Result<Self, Self::Error> {
        Self::try_from((name.as_str(), path))
    }
}

impl Note {
    /// Creates a Note struct that is unchecked to be a valid file.
    /// Only used in tests.
    /// NOTE: This will be removed.
    pub fn new_unchecked(name: &str, path: &Path) -> Self {
        Self {
            name: name.to_string(),
            path: path.to_path_buf(),
        }
    }
}
