//! This module provides functionality operating with Obsidian config.
use dirs::{config_dir, home_dir};

use serde::{Deserialize, Deserializer};
use std::path::Path;
use std::{collections::BTreeMap, fs, path::PathBuf};
use std::{env, result};

use crate::obsidian::{Error, Result, Vault};

/// Represents the Obsidian configuration, typically loaded from an `obsidian.json` file.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ObsidianConfig {
    /// A mapping of vault (folder) names to [`Vault`] definitions.
    vaults: BTreeMap<String, Vault>,
}

/// Attempts to locate and load the system's `obsidian.json` file as an [`ObsidianConfig`].
///
/// Returns an [`Error`] if the file path doesn't exist or JSON parsing failed.
pub fn load() -> Result<ObsidianConfig> {
    let config_locations = obsidian_global_config_locations();
    let existing_config_locations = config_locations
        .iter()
        .filter(|path| path.is_dir())
        .collect::<Vec<_>>();

    if let Some(config_dir) = existing_config_locations.first() {
        load_from(config_dir)
    } else {
        Err(Error::PathNotFound(format!(
            "Obsidian config directory was not found from these locations: {}",
            config_locations
                .iter()
                .map(|path| path.to_string_lossy())
                .collect::<Vec<_>>()
                .join(", ")
        )))
    }
}

/// Attempts to load `obsidian.json` file as an [`ObsidianConfig`] from the given directory
/// [`Path`].
///
/// Returns an [`Error`] if the file path doesn't exist or JSON parsing failed.
///
/// # Examples
///
/// ```
/// use std::path::Path;
/// use basalt_core::obsidian;
///
/// _ = obsidian::config::load_from(Path::new("./dir-with-config-file"));
/// ```
pub fn load_from(config_path: &Path) -> Result<ObsidianConfig> {
    let obsidian_json_path = config_path.join("obsidian.json");

    if obsidian_json_path.try_exists()? {
        let contents = fs::read_to_string(obsidian_json_path)?;
        serde_json::from_str(&contents).map_err(Error::Json)
    } else {
        // TODO: Maybe a different error should be propagated in this case. E.g. 'unreadable'
        // file.
        Err(Error::PathNotFound(
            obsidian_json_path.to_string_lossy().to_string(),
        ))
    }
}

impl ObsidianConfig {
    /// Returns a vec of vaults in the configuration.
    ///
    /// # Examples
    ///
    /// ```
    /// use basalt_core::obsidian::{ObsidianConfig, Vault};
    ///
    /// let config = ObsidianConfig::from([
    ///     ("Obsidian", Vault::default()),
    ///     ("Work", Vault::default()),
    /// ]);
    ///
    /// let vaults = config.vaults();
    ///
    /// assert_eq!(vaults.len(), 2);
    /// assert_eq!(vaults.get(0), Some(&Vault::default()).as_ref());
    /// ```
    pub fn vaults(&self) -> Vec<&Vault> {
        self.vaults.values().collect()
    }
}

impl<const N: usize> From<[(&str, Vault); N]> for ObsidianConfig {
    fn from(arr: [(&str, Vault); N]) -> Self {
        Self {
            vaults: BTreeMap::from(arr.map(|(name, vault)| (name.to_owned(), vault))),
        }
    }
}

impl<const N: usize> From<[(String, Vault); N]> for ObsidianConfig {
    fn from(arr: [(String, Vault); N]) -> Self {
        Self {
            vaults: BTreeMap::from(arr),
        }
    }
}

impl<'de> Deserialize<'de> for ObsidianConfig {
    fn deserialize<D>(deserializer: D) -> result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct Json {
            vaults: BTreeMap<String, Vault>,
        }

        impl From<Json> for ObsidianConfig {
            fn from(value: Json) -> Self {
                ObsidianConfig {
                    vaults: value
                        .vaults
                        .into_values()
                        .map(|vault| (vault.name.clone(), vault))
                        .collect(),
                }
            }
        }

        let deserialized: Json = Deserialize::deserialize(deserializer)?;
        Ok(deserialized.into())
    }
}

/// Returns all existing configuration directory paths where Obsidian might store its global
/// settings.
///
/// This function determines possible configuration locations by platform-specific conventions and
/// installation methods. On all platforms, it first checks if the user has defined the
/// `OBSIDIAN_CONFIG_DIR` environment variable. If so, that path is used, and any leading tilde (~)
/// is expanded to the current user's home directory.
///
/// On Windows, it then resolves to the default Obsidian directory located under the system's
/// application data folder, typically `%APPDATA%\Obsidian`. On macOS, the function expects to find
/// the configuration under `~/Library/Application Support/obsidian`. On Linux, the standard config
/// directory is assumed to be `$XDG_CONFIG_HOME/obsidian`, or `~/.config/obsidian`.
///
/// For Linux users, the function also accounts for sandboxed installations. If Obsidian is
/// installed via Flatpak, the configuration is likely found in
/// `~/.var/app/md.obsidian.Obsidian/config/obsidian`. For Snap installations, the relevant path is
/// typically `~/snap/obsidian/common/.config/obsidian`.
///
/// For reference:
/// - macOS:     `/Users/username/Library/Application Support/obsidian`
/// - Windows:   `%APPDATA%\Obsidian\`
/// - Linux:     `$XDG_CONFIG_HOME/obsidian` or `~/.config/obsidian`
///   - flatpak: `$HOME/.var/app/md.obsidian.Obsidian/config/obsidian`
///   - snap:    `$HOME/snap/obsidian/current/.config/obsidian`
///
/// More info: [https://help.obsidian.md/Files+and+folders/How+Obsidian+stores+data]
pub fn obsidian_global_config_locations() -> Vec<PathBuf> {
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    const OBSIDIAN_CONFIG_DIR_NAME: &str = "obsidian";

    #[cfg(target_os = "windows")]
    const OBSIDIAN_CONFIG_DIR_NAME: &str = "Obsidian";

    let override_path =
        env::var("OBSIDIAN_CONFIG_DIR")
            .ok()
            .zip(home_dir())
            .map(|(path, home_dir)| {
                PathBuf::from(path.replace("~", home_dir.to_string_lossy().as_ref()))
            });

    let default_config_path =
        config_dir().map(|config_path| config_path.join(OBSIDIAN_CONFIG_DIR_NAME));

    #[cfg(any(target_os = "macos", target_os = "windows"))]
    let sandboxed_paths: [Option<PathBuf>; 0] = [];

    // In cases where user has a sandboxes instance of Obsidian installed under either flatpak or
    // snap, we must check if the configuration exists under these locations.
    #[cfg(target_os = "linux")]
    let sandboxed_paths = {
        let flatpak_path = home_dir().map(|home_dir| {
            home_dir
                .join(".var/app/md.obsidian.Obsidian/config")
                .join(OBSIDIAN_CONFIG_DIR_NAME)
        });

        let snap_path = home_dir().map(|home_dir| {
            home_dir
                .join("snap/obsidian/current/.config")
                .join(OBSIDIAN_CONFIG_DIR_NAME)
        });

        [flatpak_path, snap_path]
    };

    let base_paths = [override_path, default_config_path];

    base_paths
        .into_iter()
        .chain(sandboxed_paths)
        .flatten()
        .collect()
}
