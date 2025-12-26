use std::{
    fs,
    path::{Path, PathBuf},
};

use super::{Error, Note, Result};

#[derive(Debug, Clone, PartialEq)]
#[allow(missing_docs)]
pub enum VaultEntry {
    File(Note),
    Directory {
        name: String,
        path: PathBuf,
        entries: Vec<VaultEntry>,
    },
}

impl VaultEntry {
    #[allow(missing_docs)]
    pub fn name(&self) -> &str {
        match self {
            Self::Directory { name, .. } => name,
            Self::File(note) => note.name(),
        }
    }
}

impl TryFrom<&Path> for VaultEntry {
    type Error = Error;
    fn try_from(value: &Path) -> Result<Self> {
        let name = value
            .with_extension("")
            .file_name()
            .map(|file_name| file_name.to_string_lossy().into_owned())
            .ok_or_else(|| Error::EmptyFileName(value.to_path_buf()))?;

        if value.is_file() {
            let note = Note::try_from((name, value.to_path_buf()))?;
            Ok(VaultEntry::File(note))
        } else {
            Ok(VaultEntry::Directory {
                name,
                path: value.to_path_buf(),
                entries: fs::read_dir(value)
                    .into_iter()
                    .flatten()
                    .filter_map(|entry| {
                        // NOTE: Might want to propagate the try_into errors further up
                        entry
                            .map_err(Error::from)
                            .and_then(|entry| entry.path().as_path().try_into())
                            .ok()
                    })
                    .collect(),
            })
        }
    }
}

#[allow(missing_docs)]
pub trait FindNote {
    #[allow(missing_docs)]
    fn find_note<'a>(&'a self, path: &Path) -> Option<&'a Note>;
}

impl FindNote for Vec<VaultEntry> {
    fn find_note<'a>(&'a self, path: &Path) -> Option<&'a Note> {
        self.iter().find_map(|entry| entry.find_note(path))
    }
}

impl FindNote for VaultEntry {
    fn find_note<'a>(&'a self, path: &Path) -> Option<&'a Note> {
        match self {
            VaultEntry::File(note) if note.path() == path => Some(note),
            VaultEntry::Directory {
                entries,
                path: dir_path,
                ..
            } if path.starts_with(dir_path) => entries.find_note(path),
            _ => None,
        }
    }
}
