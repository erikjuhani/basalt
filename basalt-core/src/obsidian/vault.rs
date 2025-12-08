use std::{fs, ops::ControlFlow, path::PathBuf, result};

use serde::{Deserialize, Deserializer};

use crate::obsidian::{vault_entry::VaultEntry, Error, Note};

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
    /// Returns a [`Vec`] of Markdown vault entries in this vault as [`VaultEntry`] structs.
    /// Entries can be either directories or files (notes). If the directory is marked hidden with
    /// a dot (`.`) prefix it will be filtered out from the resulting [`Vec`].
    ///
    /// The returned entries are not sorted.
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
    /// assert_eq!(vault.entries(), vec![]);
    /// ```
    pub fn entries(&self) -> Vec<VaultEntry> {
        match self.path.as_path().try_into() {
            Ok(VaultEntry::Directory { entries, .. }) => entries
                .into_iter()
                .filter(|entry| !entry.name().starts_with('.'))
                .collect(),
            _ => vec![],
        }
    }

    /// Creates a new empty note with the provided name.
    ///
    /// If a note with the given name already exists, a numbered suffix will be appended
    /// (e.g., "Note 1", "Note 2", etc.) to find an available name.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - I/O operations fail (directory creation, file writing, or path checks)
    /// - No available name is found after 999 attempts ([`Error::MaxAttemptsExceeded`])
    ///
    /// # Examples
    ///
    /// ```
    /// use std::fs;
    /// use tempfile::tempdir;
    /// use basalt_core::obsidian::{Vault, Note, Error};
    ///
    /// let tmp_dir = tempdir()?;
    ///
    /// let vault = Vault {
    ///   path: tmp_dir.path().to_path_buf(),
    ///   ..Default::default()
    /// };
    ///
    /// let note = vault.create_note("Arbitrary Name")?;
    /// assert_eq!(fs::exists(&note.path)?, true);
    ///
    /// # Ok::<(), Error>(())
    /// ```
    pub fn create_note(&self, name: &str) -> result::Result<Note, Error> {
        let base_path = self.path.join(name).with_extension("md");
        if let Some(parent_dir) = base_path.parent() {
            // Create necessary directory structures if we pass dir separated name like
            // /vault/notes/sub-notes/name.md
            fs::create_dir_all(parent_dir)?;
        }

        let (name, path) = self.find_available_note_name(name)?;

        fs::write(&path, "")?;

        Ok(Note {
            name: name.to_string(),
            path,
        })
    }

    /// Find available note name by incrementing number suffix at the end.
    ///
    /// Increments until we find a 'free' name e.g. if "Untitled 1" exists we will
    /// try next "Untitled 2", and then "Untitled 3" and so on.
    ///
    /// # Errors
    ///
    /// Returns [`Error::MaxAttemptsExceeded`] if no available name is found after 999 attempts.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::fs;
    /// use tempfile::tempdir;
    /// use basalt_core::obsidian::{Vault, Note, Error};
    ///
    /// let tmp_dir = tempdir()?;
    /// let tmp_path = tmp_dir.path();
    ///
    /// let vault = Vault {
    ///   path: tmp_path.to_path_buf(),
    ///   ..Default::default()
    /// };
    ///
    /// let note_name = "Arbitrary Name";
    /// fs::write(tmp_path.join(note_name).with_extension("md"), "")?;
    ///
    /// let (name, path) = vault.find_available_note_name(note_name)?;
    /// assert_eq!(&name, "Arbitrary Name 1");
    /// assert_eq!(fs::exists(&path)?, false);
    ///
    /// # Ok::<(), Error>(())
    /// ```
    pub fn find_available_note_name(&self, name: &str) -> result::Result<(String, PathBuf), Error> {
        let path = self.path.join(name).with_extension("md");
        if !fs::exists(&path)? {
            return Ok((name.to_string(), path));
        }

        // Maximum number of iterations
        const MAX: usize = 999;

        let candidate = (1..=MAX)
            .map(|n| format!("{name} {n}"))
            .try_fold((), |_, name| {
                let path = self.path.join(&name).with_extension("md");
                match fs::exists(&path).map_err(Error::from) {
                    Ok(false) => ControlFlow::Break(Ok((name, path))),
                    Err(e) => ControlFlow::Break(Err(e)),
                    _ => ControlFlow::Continue(()),
                }
            });

        match candidate {
            ControlFlow::Break(r) => r,
            ControlFlow::Continue(..) => Err(Error::MaxAttemptsExceeded {
                name: name.to_string(),
                max_attempts: MAX,
            }),
        }
    }

    /// Creates a new empty note with name "Untitled" or "Untitled {n}".
    ///
    /// This is a convenience method that calls [`Vault::create_note`] with "Untitled" as the name.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - I/O operations fail (file writing or path checks)
    /// - No available name is found after 999 attempts ([`Error::MaxAttemptsExceeded`])
    ///
    /// # Examples
    ///
    /// ```
    /// use std::{fs, result};
    /// use tempfile::tempdir;
    /// use basalt_core::obsidian::{Vault, Note, Error};
    ///
    /// let tmp_dir = tempdir()?;
    ///
    /// let vault = Vault {
    ///   path: tmp_dir.path().to_path_buf(),
    ///   ..Default::default()
    /// };
    ///
    /// let note = vault.create_untitled_note()?;
    /// assert_eq!(&note.name, "Untitled");
    /// assert_eq!(fs::exists(&note.path)?, true);
    ///
    /// (1..=100).try_for_each(|n| -> result::Result<(), Error> {
    ///   let note = vault.create_untitled_note()?;
    ///   assert_eq!(note.name, format!("Untitled {n}"));
    ///   assert_eq!(fs::exists(&note.path)?, true);
    ///   Ok(())
    /// })?;
    ///
    /// # Ok::<(), Error>(())
    /// ```
    pub fn create_untitled_note(&self) -> result::Result<Note, Error> {
        self.create_note("Untitled")
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
            ts: Option<u64>,
        }

        impl TryFrom<Json> for Vault {
            type Error = String;
            fn try_from(Json { path, open, ts }: Json) -> result::Result<Self, Self::Error> {
                let name = path
                    .file_name()
                    .map(|file_name| file_name.to_string_lossy().to_string())
                    .ok_or("unable to retrieve vault name")?;

                Ok(Vault {
                    name,
                    path,
                    open: open.unwrap_or(false),
                    ts: ts.unwrap_or(0),
                })
            }
        }

        let deserialized: Json = Deserialize::deserialize(deserializer)?;
        deserialized.try_into().map_err(serde::de::Error::custom)
    }
}
