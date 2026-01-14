//! This module provides functionality operating with Obsidian vault folders.
use std::path::{Path, PathBuf};

use crate::obsidian::Error;

/// Represents a directory within the vault.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Directory {
    name: String,
    path: PathBuf,
}

impl Directory {
    /// Create a new directory structure
    pub fn new(name: &str, path: &Path) -> Self {
        Self {
            name: name.to_string(),
            path: path.to_path_buf(),
        }
    }

    /// The complete filesystem path to the directory.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The directory name without the parent path.
    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl TryFrom<(&str, PathBuf)> for Directory {
    type Error = Error;
    fn try_from((name, path): (&str, PathBuf)) -> Result<Self, Self::Error> {
        match path.is_dir() {
            true => Ok(Self {
                name: name.to_string(),
                path,
            }),
            false => Err(Error::Io(std::io::ErrorKind::NotADirectory.into())),
        }
    }
}

impl TryFrom<(String, PathBuf)> for Directory {
    type Error = Error;
    fn try_from((name, path): (String, PathBuf)) -> Result<Self, Self::Error> {
        Self::try_from((name.as_str(), path))
    }
}
