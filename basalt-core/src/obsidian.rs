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
//!
//! _ = config.get_vault_by_name("Obsidian");
//! ```
use std::{io, result};

mod config;
mod note;
mod vault;

pub use config::ObsidianConfig;
pub use note::Note;
pub use vault::Vault;

/// A [`std::result::Result`] type for fallible operations in [`crate::obsidian`].
///
/// For convenience of use and to avoid writing [`Error`] directly.
/// All fallible operations return [`Error`] as the error variant.
///
/// # Examples
///
/// ```
/// use basalt_core::obsidian::{ObsidianConfig, Result};
///
/// fn get_vault_names() -> Result<Vec<String>> {
///     let config = ObsidianConfig::load()?;
///     Ok(config.vaults().map(|(name,_)| name).collect())
/// }
///
/// fn main() -> Result<()> {
///     let vaults = get_vault_names()?;
///     println!("Found vaults: {:?}", vaults);
///     Ok(())
/// }
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

    /// JSON (de)serialization error, from [`serde_json::Error`].
    #[error("JSON (de)serialization error: {0}")]
    Json(#[from] serde_json::Error),

    /// I/O error, from [`std::io::Error`].
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),
}
