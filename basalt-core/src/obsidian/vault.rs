use std::{
    cmp::Ordering,
    path::{Path, PathBuf},
    result,
};

use serde::{Deserialize, Deserializer};
use tokio::fs;

use crate::obsidian::Note;

/// Represents a single Obsidian vault.
///
/// A vault is a folder containing notes and other metadata.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Vault {
    /// The name of the vault, inferred from its directory name.
    pub name: String,

    /// Filesystem path to the vault's directory.
    pub path: PathBuf,

    /// Whether the vault is marked 'open' by Obsidian.
    pub open: bool,

    /// Timestamp of last update or creation.
    pub ts: u64,
}

impl Vault {
    /// Returns an iterator over Markdown (`.md`) files in this vault as [`Note`] structs.
    ///
    /// # Examples
    ///
    /// ```
    /// use basalt_core::obsidian::{Vault, Note};
    ///
    /// let vault = Vault {
    ///     name: "MyVault".into(),
    ///     path: "path/to/my_vault".into(),
    ///     ..Default::default()
    /// };
    ///
    /// assert_eq!(vault.notes().await.unwrap(), vec![]);
    /// ```
    pub async fn notes(&self) -> Result<Vec<Note>, std::io::Error> {
        let mut notes = Vec::new();
        let mut dir = fs::read_dir(&self.path).await?;

        while let Some(entry) = dir.next_entry().await? {
            let file_name = entry.file_name().to_string_lossy().into_owned();
            if let Some((name, extension)) = file_name.split_once('.') {
                if extension != "md" {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        "Invalid extension",
                    ));
                }
                let path = entry.path();
                let created = entry.metadata().await?.created()?;

                notes.push(Note {
                    name: name.to_string(),
                    path,
                    created,
                });
            }
        }

        Ok(notes)
    }

    /// Returns a sorted vector [`Vec<Note>`] of all notes in the vault, sorted according to the
    /// provided comparison function.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::cmp::Ordering;
    /// use basalt_core::obsidian::{Vault, Note};
    ///
    /// let vault = Vault {
    ///     name: "MyVault".to_string(),
    ///     path: "path/to/my_vault".into(),
    ///     ..Default::default()
    /// };
    ///
    /// let alphabetically = |a: &Note, b: &Note| a.name.to_lowercase().cmp(&b.name.to_lowercase());
    ///
    /// _ = vault.notes_sorted_by(alphabetically).await;
    /// ```
    pub async fn notes_sorted_by(&self, compare: impl Fn(&Note, &Note) -> Ordering) -> Vec<Note> {
        let mut notes: Vec<Note> = self.notes().await.unwrap_or_default();

        notes.sort_by(compare);

        notes
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
                    path: value.path,
                    open: value.open.unwrap_or_default(),
                    ts: value.ts,
                })
            }
        }

        let deserialized: Json = Deserialize::deserialize(deserializer)?;
        deserialized.try_into().map_err(serde::de::Error::custom)
    }
}
