use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
    result,
};

use ki::fs::SortablePath;
use serde::{Deserialize, Deserializer};

use crate::obsidian::Note;

/// Represents a single Obsidian vault.
///
/// A vault is a folder containing notes and other metadata.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Vault {
    /// The name of the vault, inferred from its directory name.
    pub name: String,

    /// Filesystem path to the vault's directory.
    pub path: SortablePath,

    /// Whether the vault is marked 'open' by Obsidian.
    pub open: bool,

    /// Timestamp of last update or creation.
    pub ts: u64,
}

impl Vault {
    /// Returns a `BTreeSet<SortablePath>` containing all the filesystem entries inside the vault,
    /// sorted in alphabetical order, directories first.
    /// TODO: Example snippet
    pub fn load(&self) -> std::io::Result<BTreeSet<SortablePath>> {
        fn load_recursive(path: &SortablePath) -> std::io::Result<BTreeSet<SortablePath>> {
            let mut entries = BTreeSet::new();
            let sub_entries: Vec<_> = fs::read_dir(path)?.collect();

            for entry in sub_entries {
                let entry = entry?;
                let path = entry.path();
                entries.insert(SortablePath(path.clone()));

                if entry.metadata()?.is_dir() {
                    entries.extend(load_recursive(&SortablePath(path))?);
                }
            }

            Ok(entries)
        }

        load_recursive(&self.path)
    }

    /// Returns a `BTreeSet<SortablePath>` containing the first-level filesystem entries in the vault,
    /// sorted in alphabetical order, directories first.|
    /// TODO: Example snippet
    pub fn load_first_level(&self) -> std::io::Result<BTreeSet<SortablePath>> {
        let mut entries = BTreeSet::new();

        for entry in std::fs::read_dir(&self.path)? {
            let entry = entry?;
            entries.insert(SortablePath(entry.path()));
        }

        Ok(entries)
    }
}

impl<'de> Deserialize<'de> for Vault {
    fn deserialize<D>(deserializer: D) -> result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct Json {
            path: PathBuf,
            open: Option<bool>,
            ts: u64,
        }

        impl TryFrom<Json> for Vault {
            type Error = String;
            fn try_from(value: Json) -> Result<Self, Self::Error> {
                let path = Path::new(&value.path);
                let name = path
                    .file_name()
                    .ok_or_else(|| String::from("unable to retrieve vault name"))?
                    .to_string_lossy()
                    .to_string();
                Ok(Vault {
                    name,
                    path: SortablePath(value.path),
                    open: value.open.unwrap_or_default(),
                    ts: value.ts,
                })
            }
        }

        let deserialized: Json = Deserialize::deserialize(deserializer)?;
        deserialized.try_into().map_err(serde::de::Error::custom)
    }
}

/// Internal wrapper for directory entries to implement custom conversion between [`fs::DirEntry`]
/// and [`Option<Note>`].
#[derive(Debug)]
struct DirEntry(fs::DirEntry);

impl From<fs::DirEntry> for DirEntry {
    fn from(value: fs::DirEntry) -> Self {
        DirEntry(value)
    }
}

impl From<DirEntry> for Option<Note> {
    /// Transforms path with extension `.md` into [`Option<Note>`].
    fn from(value: DirEntry) -> Option<Note> {
        let dir = value.0;
        let created = dir.metadata().ok()?.created().ok()?;
        let path = dir.path();

        if path.extension()? != "md" {
            return None;
        }

        let name = path
            .with_extension("")
            .file_name()
            .map(|file_name| file_name.to_string_lossy().into_owned())?;

        Some(Note {
            name,
            path,
            created,
        })
    }
}
