use std::path::PathBuf;

use basalt_core::obsidian::{Note, VaultEntry};

#[derive(Debug, Clone, PartialEq)]
pub enum Item {
    File(Note),
    Directory {
        name: String,
        path: PathBuf,
        expanded: bool,
        items: Vec<Item>,
    },
}

impl Item {
    pub(crate) fn name(&self) -> &str {
        match self {
            Self::Directory { name, .. } => name.as_str(),
            Self::File(note) => note.name(),
        }
    }

    pub(crate) fn is_dir(&self) -> bool {
        matches!(self, Self::Directory { .. })
    }
}

impl From<VaultEntry> for Item {
    fn from(value: VaultEntry) -> Self {
        match value {
            VaultEntry::File(note) => Self::File(note),
            VaultEntry::Directory {
                name,
                entries,
                path,
            } => Self::Directory {
                name,
                path,
                expanded: false,
                items: entries.into_iter().map(|item| item.into()).collect(),
            },
        }
    }
}
