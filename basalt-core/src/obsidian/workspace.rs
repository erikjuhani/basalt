//! This module provides functionality operating with Obsidian workspace state.
use serde::Deserialize;
use std::{fs, path::Path, path::PathBuf};

use crate::obsidian::{Error, Result};

/// Represents the Obsidian workspace state, typically loaded from a `workspace.json` file
/// inside a vault's `.obsidian` directory.
///
/// More info: [https://help.obsidian.md/data-storage#Vault+settings]
#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Workspace {
    active: Option<String>,
    last_open_files: Vec<PathBuf>,
}

impl Workspace {
    /// Returns the identifier of the active workspace leaf, if any.
    ///
    /// The identifier refers to a leaf in the workspace layout, not a file path.
    pub fn active(&self) -> Option<&str> {
        self.active.as_deref()
    }

    /// Returns the vault-relative paths of the most recently open files, most recent first.
    pub fn last_open_files(&self) -> &[PathBuf] {
        &self.last_open_files
    }
}

/// Attempts to load `workspace.json` from the given `.obsidian` directory path.
///
/// Returns an [`Error`] if the file doesn't exist or JSON parsing failed.
///
/// # Examples
///
/// ```
/// use std::path::Path;
/// use basalt_core::obsidian;
///
/// _ = obsidian::workspace::load_from(Path::new("./dir-with-workspace-file"));
/// ```
pub fn load_from(config_dir: &Path) -> Result<Workspace> {
    let workspace_json_path = config_dir.join("workspace.json");
    if workspace_json_path.try_exists()? {
        let contents = fs::read_to_string(workspace_json_path)?;
        serde_json::from_str(&contents).map_err(Error::Json)
    } else {
        Err(Error::PathNotFound(
            workspace_json_path.to_string_lossy().to_string(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn parses_active_and_last_open_files() {
        let dir = tempdir().unwrap();
        fs::write(
            dir.path().join("workspace.json"),
            r#"{"active":"29458dac1350a42f","lastOpenFiles":["index.md"]}"#,
        )
        .unwrap();

        let workspace = load_from(dir.path()).unwrap();

        assert_eq!(workspace.active(), Some("29458dac1350a42f"));
        assert_eq!(workspace.last_open_files(), [PathBuf::from("index.md")]);
    }

    #[test]
    fn ignores_unknown_fields() {
        let dir = tempdir().unwrap();
        fs::write(
            dir.path().join("workspace.json"),
            r#"{"main":{},"left":{},"active":"leaf-id","lastOpenFiles":[]}"#,
        )
        .unwrap();

        let workspace = load_from(dir.path()).unwrap();

        assert_eq!(workspace.active(), Some("leaf-id"));
        assert!(workspace.last_open_files().is_empty());
    }

    #[test]
    fn defaults_missing_properties() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("workspace.json"), "{}").unwrap();

        let workspace = load_from(dir.path()).unwrap();

        assert_eq!(workspace.active(), None);
        assert!(workspace.last_open_files().is_empty());
    }

    #[test]
    fn errors_when_file_is_missing() {
        let dir = tempdir().unwrap();

        assert!(matches!(load_from(dir.path()), Err(Error::PathNotFound(_))));
    }

    #[test]
    fn errors_when_file_is_malformed() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("workspace.json"), "not json").unwrap();

        assert!(matches!(load_from(dir.path()), Err(Error::Json(_))));
    }
}
