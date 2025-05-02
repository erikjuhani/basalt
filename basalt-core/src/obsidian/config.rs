use dirs::config_local_dir;

use serde::{Deserialize, Deserializer};
use std::path::Path;
use std::result;
use std::{collections::BTreeMap, fs, path::PathBuf};

use crate::obsidian::{Error, Result, Vault};

/// Represents the Obsidian configuration, typically loaded from an `obsidian.json` file.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ObsidianConfig {
    /// A mapping of vault (folder) names to [`Vault`] definitions.
    vaults: BTreeMap<String, Vault>,
}

impl ObsidianConfig {
    /// Attempts to locate and load the system's `obsidian.json` file as an [`ObsidianConfig`].
    ///
    /// Returns an [`Error`] if the filepath doesn't exist or JSON parsing failed.
    pub fn load() -> Result<Self> {
        obsidian_config_dir()
            .map(|path_buf| ObsidianConfig::load_from(&path_buf))
            .ok_or(Error::PathNotFound("Obsidian config directory".to_string()))?
    }

    /// Attempts to load `obsidian.json` file as an [`ObsidianConfig`] from the given directory
    /// [`Path`].
    ///
    /// Returns an [`Error`] if the filepath doesn't exist or JSON parsing failed.
    ///
    /// # Examples
    ///
    /// ```
    /// use basalt_core::obsidian::ObsidianConfig;
    /// use std::path::Path;
    ///
    /// _ = ObsidianConfig::load_from(Path::new("./dir-with-config-file"));
    /// ```
    pub fn load_from(config_path: &Path) -> Result<Self> {
        let contents = fs::read_to_string(config_path.join("obsidian.json"))?;
        Ok(serde_json::from_str(&contents)?)
    }

    /// Returns an iterator over the vaults in the configuration.
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

    /// Finds a vault by name, returning a reference if it exists.
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
    /// _ = config.get_vault_by_name("Obsidian");
    /// ```
    pub fn get_vault_by_name(&self, name: &str) -> Option<&Vault> {
        self.vaults.get(name)
    }

    /// Gets the currently opened vault marked by Obsidian.
    ///
    /// # Examples
    ///
    /// ```
    /// use basalt_core::obsidian::{ObsidianConfig, Vault};
    ///
    /// let config = ObsidianConfig::from([
    ///     (
    ///         "Obsidian",
    ///         Vault {
    ///             open: true,
    ///             ..Vault::default()
    ///         },
    ///     ),
    ///     ("Work", Vault::default()),
    /// ]);
    ///
    /// _ = config.get_open_vault();
    /// ```
    pub fn get_open_vault(&self) -> Option<&Vault> {
        self.vaults.values().find(|vault| vault.open)
    }
}

impl<const N: usize> From<[(&str, Vault); N]> for ObsidianConfig {
    /// # Examples
    ///
    /// ```
    /// use basalt_core::obsidian::{ObsidianConfig, Vault};
    ///
    /// let config_1 = ObsidianConfig::from([
    ///   ("Obsidian", Vault::default()),
    ///   ("My Vault", Vault::default()),
    /// ]);
    ///
    /// let config_2: ObsidianConfig = [
    ///   ("Obsidian", Vault::default()),
    ///   ("My Vault", Vault::default()),
    /// ].into();
    ///
    /// assert_eq!(config_1, config_2);
    /// ```
    fn from(arr: [(&str, Vault); N]) -> Self {
        Self {
            vaults: BTreeMap::from(arr.map(|(name, vault)| (name.to_owned(), vault))),
        }
    }
}

impl<const N: usize> From<[(String, Vault); N]> for ObsidianConfig {
    /// # Examples
    ///
    /// ```
    /// use basalt_core::obsidian::{ObsidianConfig, Vault};
    ///
    /// let config_1 = ObsidianConfig::from([
    ///   (String::from("Obsidian"), Vault::default()),
    ///   (String::from("My Vault"), Vault::default()),
    /// ]);
    ///
    /// let config_2: ObsidianConfig = [
    ///   (String::from("Obsidian"), Vault::default()),
    ///   (String::from("My Vault"), Vault::default()),
    /// ].into();
    ///
    /// assert_eq!(config_1, config_2);
    /// ```
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

/// Returns the system path to Obsidian's config folder, if any.
///
/// For reference:
/// - macOS:  `/Users/username/Library/Application Support/obsidian`
/// - Windows: `%APPDATA%\Obsidian\`
/// - Linux:   `$XDG_CONFIG_HOME/obsidian/` or `~/.config/obsidian/`
///
/// More info: [https://help.obsidian.md/Files+and+folders/How+Obsidian+stores+data]
fn obsidian_config_dir() -> Option<PathBuf> {
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    const OBSIDIAN_CONFIG_DIR_NAME: &str = "obsidian";

    #[cfg(target_os = "windows")]
    const OBSIDIAN_CONFIG_DIR_NAME: &str = "Obsidian";

    config_local_dir().map(|config_path| config_path.join(OBSIDIAN_CONFIG_DIR_NAME))
}
