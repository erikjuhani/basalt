//! This module provides functionality operating with Obsidian. It lets you read and manipulate
//! Obsidian's configuration, vaults, and notes.
//!
//! Currently supports reading vaults, notes, and writing to note path.
//!
//! # Example
//!
//! ```
//! use basalt_core::obsidian::{ObsidianConfig, Error, Vault};
//!
//! let config = ObsidianConfig::from([
//!   ("Obsidian", Vault::default()),
//!   ("My Vault", Vault::default()),
//! ]);
//! ```
use std::{io, path::PathBuf, result};

pub mod config;
mod note;
mod vault;
mod vault_entry;

pub use config::ObsidianConfig;
pub use note::Note;
pub use vault::*;
pub use vault_entry::FindNote;
pub use vault_entry::VaultEntry;

/// A [`std::result::Result`] type for fallible operations in [`crate::obsidian`].
///
/// For convenience of use and to avoid writing [`Error`] directly.
/// All fallible operations return [`Error`] as the error variant.
///
/// # Examples
///
/// ```
/// use std::path::Path;
/// use basalt_core::obsidian;
///
/// let config_result = obsidian::config::load_from(Path::new("./nonexistent"));
/// assert_eq!(config_result.is_err(), true);
/// ```
pub type Result<T> = result::Result<T, Error>;

/// Error type for fallible operations in this [`crate`].
///
/// Implements [`std::error::Error`] via [thiserror](https://docs.rs/thiserror).
#[derive(thiserror::Error, Debug)]
pub enum Error {
    /// Expected resource behind a path was not found.
    #[error("Path not found: {0}")]
    PathNotFound(String),

    /// Filename was empty
    #[error("Empty filename for path: {0}")]
    EmptyFileName(PathBuf),

    /// JSON (de)serialization error, from [`serde_json::Error`].
    #[error("JSON (de)serialization error: {0}")]
    Json(#[from] serde_json::Error),

    /// I/O error, from [`std::io::Error`].
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),

    /// Exceeded maximum attempts while searching for an available note name.
    ///
    /// This occurs when creating a note with a name that already exists, and all
    /// numbered variants (e.g., "Name 1", "Name 2", ..., "Name 999") also exist.
    #[error("Failed to find available name for '{name}' after {max_attempts} attempts")]
    MaxAttemptsExceeded {
        /// The base name that was attempted
        name: String,
        /// The maximum number of attempts made
        max_attempts: usize,
    },
}
