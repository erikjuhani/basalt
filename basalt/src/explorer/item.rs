use std::path::PathBuf;

use basalt_core::obsidian::{Note, VaultEntry};

#[derive(Debug, Clone, PartialEq)]
pub enum Item {
    File {
        note: Note,
        depth: usize,
    },
    Directory {
        name: String,
        path: PathBuf,
        expanded: bool,
        items: Vec<Item>,
        depth: usize,
    },
}

impl Item {
    pub(crate) fn depth(&self) -> usize {
        match self {
            Self::Directory { depth, .. } | Self::File { depth, .. } => *depth,
        }
    }

    pub(crate) fn name(&self) -> &str {
        match self {
            Self::Directory { name, .. } => name.as_str(),
            Self::File { note, .. } => note.name(),
        }
    }

    pub(crate) fn is_dir(&self) -> bool {
        matches!(self, Self::Directory { .. })
    }
}

impl From<VaultEntry> for Item {
    fn from(value: VaultEntry) -> Self {
        fn to_items(depth: usize, entry: VaultEntry) -> Item {
            match entry {
                VaultEntry::File(note) => Item::File { note, depth },
                VaultEntry::Directory {
                    name,
                    entries,
                    path,
                } => Item::Directory {
                    name,
                    path,
                    depth,
                    expanded: false,
                    items: entries
                        .into_iter()
                        .map(|entry| to_items(depth + 1, entry))
                        .collect(),
                },
            }
        }

        to_items(0, value)
    }
}
