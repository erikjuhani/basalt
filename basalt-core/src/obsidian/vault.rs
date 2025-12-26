//! This module provides functionality operating with Obsidian vaults.
use std::{
    fs,
    ops::ControlFlow,
    path::{self, Path, PathBuf},
    result,
};

use serde::{Deserialize, Deserializer};

use crate::obsidian::{directory::Directory, vault_entry::VaultEntry, Error, Note};

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

fn basename(path: &Path, extension: Option<&str>) -> result::Result<String, Error> {
    match extension {
        Some(_) => path.file_stem(),
        None => path.file_name(),
    }
    .and_then(|os_str| os_str.to_str().map(|str| str.to_string()))
    .ok_or_else(|| Error::InvalidPathName(path.to_path_buf()))
}

/// Rename note with the given name.
///
/// # Examples
///
/// ```
/// # use std::fs;
/// # use tempfile::tempdir;
/// # use basalt_core::obsidian::{self, Vault, Note, Error};
/// #
/// # let tmp_dir = tempdir()?;
/// # let tmp_path = tmp_dir.path();
/// #
/// let vault = Vault { path: tmp_path.to_path_buf(), ..Default::default() };
/// let note = obsidian::vault::create_note(&vault, "Arbitrary Name")?;
///
/// let note = obsidian::vault::rename_note(note, "New Name.md")?;
/// assert_eq!(note.name(), "New Name");
/// assert_eq!(note.path(), tmp_path.join("New Name.md"));
/// assert_eq!(fs::exists(note.path())?, true);
///
/// let note = obsidian::vault::rename_note(note, "Renamed")?;
/// assert_eq!(note.name(), "Renamed");
/// assert_eq!(note.path(), tmp_path.join("Renamed.md"));
/// assert_eq!(fs::exists(note.path())?, true);
/// # Ok::<(), Error>(())
/// ```
pub fn rename_note(note: Note, new_name: &str) -> result::Result<Note, Error> {
    if new_name.is_empty() {
        return Err(Error::EmptyFileName(PathBuf::default()));
    }

    let path = note.path();
    let parent = path
        .parent()
        .ok_or(Error::EmptyFileName(path.to_path_buf()))?;

    let new_name = new_name.trim_end_matches(".md");
    let new_path = parent.join(new_name).with_extension("md");

    if fs::exists(&new_path)? {
        return Err(Error::Io(std::io::ErrorKind::AlreadyExists.into()));
    }

    // FIXME: After checking for invalid filenames
    if let Some(path) = new_path.parent() {
        fs::create_dir_all(path)?
    }

    fs::rename(path, &new_path)?;

    Note::try_from((new_name, new_path))
}

/// Moves the note to the given directory.
///
/// # Examples
///
/// ```
/// # use std::fs;
/// # use tempfile::tempdir;
/// # use basalt_core::obsidian::{self, Vault, Note, Error};
/// #
/// # let tmp_dir = tempdir()?;
/// # let tmp_path = tmp_dir.path();
/// #
/// let vault = Vault { path: tmp_path.to_path_buf(), ..Default::default() };
/// let note = obsidian::vault::create_note(&vault, "/notes/Arbitrary Name")?;
/// let dir = obsidian::vault::create_dir(&vault, "/archive")?;
/// let note = obsidian::vault::move_note_to(note, dir)?;
///
/// assert_eq!(note.name(), "Arbitrary Name");
/// assert_eq!(note.path(), tmp_path.join("archive/Arbitrary Name.md"));
/// assert_eq!(fs::exists(note.path())?, true);
/// # Ok::<(), Error>(())
/// ```
pub fn move_note_to(note: Note, directory: Directory) -> result::Result<Note, Error> {
    let name = basename(note.path(), None)?;

    let new_path = directory.path().join(name);
    if fs::exists(&new_path)? {
        return Err(Error::Io(std::io::ErrorKind::AlreadyExists.into()));
    }

    fs::rename(note.path(), &new_path)?;

    Note::try_from((note.name(), new_path))
}

/// Moves directory to the given directory.
///
/// # Examples
///
/// ```
/// # use std::fs;
/// # use tempfile::tempdir;
/// # use basalt_core::obsidian::{self, Vault, Note, Error};
/// #
/// # let tmp_dir = tempdir()?;
/// # let tmp_path = tmp_dir.path();
/// #
/// let vault = Vault { path: tmp_path.to_path_buf(), ..Default::default() };
/// let dir_a = obsidian::vault::create_dir(&vault, "/notes")?;
/// let dir_b = obsidian::vault::create_dir(&vault, "/archive")?;
/// let dir = obsidian::vault::move_dir_to(dir_a, dir_b)?;
///
/// assert_eq!(dir.name(), "notes");
/// assert_eq!(dir.path(), tmp_path.join("archive/notes"));
/// assert_eq!(fs::exists(dir.path())?, true);
/// # Ok::<(), Error>(())
/// ```
pub fn move_dir_to(from: Directory, to: Directory) -> result::Result<Directory, Error> {
    let name = basename(from.path(), None)?;

    let new_path = to.path().join(&name);
    if fs::exists(&new_path)? {
        return Err(Error::Io(std::io::ErrorKind::AlreadyExists.into()));
    }

    fs::rename(from.path(), &new_path)?;

    Directory::try_from((from.name(), new_path))
}

/// Creates a new empty directory with the provided name.
///
/// If a directory with the given name already exists, a numbered suffix will be appended
/// (e.g., "Dir 1", "Dir 2", etc.) to find an available name.
///
/// # Errors
///
/// Returns an error if:
/// - I/O operations fail (directory creation, path checks)
/// - No available name is found after 999 attempts ([`Error::MaxAttemptsExceeded`])
///
/// # Examples
///
/// ```
/// # use std::fs;
/// # use tempfile::tempdir;
/// # use basalt_core::obsidian::{self, Vault, Note, Error};
/// #
/// # let tmp_dir = tempdir()?;
///
/// let vault = Vault { path: tmp_dir.path().to_path_buf(), ..Default::default() };
/// let dir = obsidian::vault::create_dir(&vault, "/sub-dir/Arbitrary.Name")?;
/// # assert_eq!(dir.name(), "Arbitrary.Name");
/// # assert_eq!(dir.path().is_dir(), true);
/// # assert_eq!(fs::exists(dir.path())?, true);
/// # Ok::<(), Error>(())
/// ```
pub fn create_dir(vault: &Vault, name: &str) -> result::Result<Directory, Error> {
    let (name, path) = find_available_path_name(vault, name, None)?;
    fs::create_dir_all(&path)?;
    Directory::try_from((name, path))
}

/// Creates a new empty directory with name "Untitled" or "Untitled {n}".
///
/// This is a convenience method that calls [`Vault::create_dir`] with "Untitled" as the name.
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
/// # use std::{fs, result};
/// # use tempfile::tempdir;
/// # use basalt_core::obsidian::{self, Vault, Note, Error};
/// #
/// # let tmp_dir = tempdir()?;
/// # let tmp_path = tmp_dir.path();
/// #
/// let vault = Vault { path: tmp_path.to_path_buf(), ..Default::default() };
/// let dir = obsidian::vault::create_untitled_dir(&vault)?;
///
/// assert_eq!(dir.name(), "Untitled");
/// assert_eq!(fs::exists(dir.path())?, true);
/// assert_eq!(dir.path().is_dir(), true);
/// #
/// # (1..=100).try_for_each(|n| -> result::Result<(), Error> {
/// #   let dir = obsidian::vault::create_untitled_dir(&vault)?;
/// #   assert_eq!(dir.name(), format!("Untitled {n}"));
/// #   assert_eq!(fs::exists(dir.path())?, true);
/// #   assert_eq!(dir.path().is_dir(), true);
/// #   Ok(())
/// # })?;
/// # Ok::<(), Error>(())
/// ```
pub fn create_untitled_dir(vault: &Vault) -> result::Result<Directory, Error> {
    create_dir(vault, "Untitled")
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
/// # use std::fs;
/// # use tempfile::tempdir;
/// # use basalt_core::obsidian::{self, Vault, Note, Error};
/// #
/// # let tmp_dir = tempdir()?;
/// # let tmp_path = tmp_dir.path();
/// #
/// let vault = Vault { path: tmp_path.to_path_buf(), ..Default::default() };
/// let note = obsidian::vault::create_note(&vault, "/notes/Arbitrary Name")?;
/// assert_eq!(note.name(), "Arbitrary Name");
/// assert_eq!(note.path(), tmp_path.join("notes/Arbitrary Name.md"));
/// assert_eq!(fs::exists(note.path())?, true);
/// # Ok::<(), Error>(())
/// ```
pub fn create_note(vault: &Vault, name: &str) -> result::Result<Note, Error> {
    let name = name.trim_start_matches(path::MAIN_SEPARATOR);

    let base_path = vault.path.join(name).with_extension("md");
    if let Some(parent_dir) = base_path.parent() {
        // Create necessary directory structures if we pass dir separated name like
        // /vault/notes/sub-notes/name.md
        fs::create_dir_all(parent_dir)?;
    }

    let (name, path) = find_available_path_name(vault, name, Some("md"))?;

    fs::write(&path, "")?;

    Note::try_from((name, path))
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
/// # use std::{fs, result};
/// # use tempfile::tempdir;
/// # use basalt_core::obsidian::{self, Vault, Note, Error};
/// #
/// # let tmp_dir = tempdir()?;
/// # let tmp_path = tmp_dir.path();
/// #
/// let vault = Vault { path: tmp_path.to_path_buf(), ..Default::default() };
/// let note = obsidian::vault::create_untitled_note(&vault)?;
/// assert_eq!(note.name(), "Untitled");
/// assert_eq!(fs::exists(note.path())?, true);
/// #
/// # (1..=100).try_for_each(|n| -> result::Result<(), Error> {
/// #   let note = obsidian::vault::create_untitled_note(&vault)?;
/// #   assert_eq!(note.name(), format!("Untitled {n}"));
/// #   assert_eq!(fs::exists(note.path())?, true);
/// #   Ok(())
/// # })?;
/// # Ok::<(), Error>(())
/// ```
pub fn create_untitled_note(vault: &Vault) -> result::Result<Note, Error> {
    create_note(vault, "Untitled")
}

/// Find available path name by incrementing number suffix at the end.
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
/// ## Markdown filename
/// ```
/// # use std::fs;
/// # use tempfile::tempdir;
/// # use basalt_core::obsidian::{self, Vault, Note, Error};
/// #
/// # let tmp_dir = tempdir()?;
/// # let tmp_path = tmp_dir.path();
/// #
/// let vault = Vault { path: tmp_path.to_path_buf(), ..Default::default() };
/// let note_name = "Arbitrary Name";
/// # fs::write(tmp_path.join(note_name).with_extension("md"), "")?;
///
/// let (name, path) = obsidian::vault::find_available_path_name(&vault, note_name, Some("md"))?;
/// assert_eq!(&name, "Arbitrary Name 1");
/// assert_eq!(fs::exists(&path)?, false);
/// # Ok::<(), Error>(())
/// ```
///
/// ## Directory name
/// ```
/// # use std::fs;
/// # use tempfile::tempdir;
/// # use basalt_core::obsidian::{self, Vault, Note, Error};
/// #
/// # let tmp_dir = tempdir()?;
/// # let tmp_path = tmp_dir.path();
/// #
/// let vault = Vault { path: tmp_path.to_path_buf(), ..Default::default() };
/// let dir_name = "Arbitrary.Dir";
/// # fs::create_dir_all(tmp_path.join(dir_name))?;
///
/// let (name, path) = obsidian::vault::find_available_path_name(&vault, dir_name, None)?;
/// assert_eq!(&name, "Arbitrary.Dir 1");
/// assert_eq!(fs::exists(&path)?, false);
/// # Ok::<(), Error>(())
/// ```
pub fn find_available_path_name(
    vault: &Vault,
    name: &str,
    extension: Option<&str>,
) -> result::Result<(String, PathBuf), Error> {
    let name = name.trim_start_matches(path::MAIN_SEPARATOR);

    let name_to_path = |name: &str| match extension {
        Some(ext) => vault.path.join(name).with_extension(ext),
        None => vault.path.join(name),
    };

    let path = name_to_path(name);
    if !fs::exists(&path)? {
        return Ok((basename(&path, extension)?, path));
    }

    // Maximum number of iterations
    const MAX: usize = 999;

    let candidate = (1..=MAX)
        .map(|n| format!("{name} {n}"))
        .try_fold((), |_, name| {
            let path = name_to_path(&name);
            match fs::exists(&path).map_err(Error::from) {
                Ok(false) => {
                    ControlFlow::Break(basename(&path, extension).map(|name| (name, path)))
                }
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

impl Vault {
    /// Returns a [`Vec`] of entries as [`VaultEntry`]s. Entries can be either directories or
    /// files. If the directory is marked hidden with a dot (`.`) prefix it will be filtered out
    /// from the resulting [`Vec`].
    ///
    /// The returned entries are not sorted.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::result;
    /// use tempfile::tempdir;
    /// use basalt_core::obsidian::{self, Vault, Note, Error};
    ///
    /// let tmp_dir = tempdir()?;
    ///
    /// let vault = Vault {
    ///   path: tmp_dir.path().to_path_buf(),
    ///   ..Default::default()
    /// };
    ///
    /// (1..=5).try_for_each(|n| -> result::Result<(), Error> {
    ///   _ = obsidian::vault::create_untitled_note(&vault)?;
    ///   Ok(())
    /// })?;
    ///
    /// assert_eq!(vault.entries().len(), 5);
    ///
    /// # Ok::<(), Error>(())
    /// ```
    /// TODO: Add Options struct to configure e.g. filters. Currently all hidden folders are filtered.
    pub fn entries(&self) -> Vec<VaultEntry> {
        match self.path.as_path().try_into() {
            Ok(VaultEntry::Directory { entries, .. }) => entries
                .into_iter()
                .filter(|entry| !entry.name().starts_with('.'))
                .collect(),
            _ => vec![],
        }
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
                let name = basename(&path, None).map_err(|e| e.to_string())?;

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
