mod key_binding;

use core::fmt;
use std::{collections::BTreeMap, fs::read_to_string};

use etcetera::{choose_base_strategy, home_dir, BaseStrategy};
use key_binding::KeyBinding;
use serde::Deserialize;

use crate::{app::Message, command::Command};
pub(crate) use key_binding::Key;

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    // Standard IO error, from [`std::io::Error`].
    #[error(transparent)]
    Io(#[from] std::io::Error),
    // Occurs when the home directory cannot be located, from [`etcetera::HomeDirError`].
    #[error(transparent)]
    HomeDir(#[from] etcetera::HomeDirError),
    /// TOML (De)serialization error, from [`toml::de::Error`].
    #[error(transparent)]
    Toml(#[from] toml::de::Error),
    #[error("Invalid keybinding: {0}")]
    InvalidKeybinding(String),
    #[error("Unknown code: {0}")]
    UnknownKeyCode(String),
    #[error("Unknown modifiers: {0}")]
    UnknownKeyModifiers(String),
    #[error("User config not found: {0}")]
    UserConfigNotFound(String),
}

#[derive(Clone, Debug, PartialEq)]
pub struct ConfigSection<'a> {
    pub key_bindings: BTreeMap<String, Message<'a>>,
}

impl ConfigSection<'_> {
    /// Takes self and another config and merges the `key_bindings` together overwriting the
    /// existing entries with the value from another config.
    pub(crate) fn merge_key_bindings(&mut self, config: Self) {
        config.key_bindings.into_iter().for_each(|(key, message)| {
            self.key_bindings.insert(key, message);
        });
    }

    pub fn key_to_message(&self, key: Key) -> Option<Message<'_>> {
        self.key_bindings.get(&key.to_string()).cloned()
    }
}

impl fmt::Display for ConfigSection<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.key_bindings
            .iter()
            .try_for_each(|(key, message)| -> fmt::Result { writeln!(f, "{key}: {message:?}") })?;

        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Config<'a> {
    pub experimental_editor: bool,
    pub global: ConfigSection<'a>,
    pub splash: ConfigSection<'a>,
    pub explorer: ConfigSection<'a>,
    pub outline: ConfigSection<'a>,
    pub help_modal: ConfigSection<'a>,
    pub note_editor: ConfigSection<'a>,
    pub vault_selector_modal: ConfigSection<'a>,
}

impl Default for Config<'_> {
    fn default() -> Self {
        Self::from(TomlConfig::default())
    }
}

impl From<TomlConfig> for Config<'_> {
    fn from(value: TomlConfig) -> Self {
        Self {
            experimental_editor: value.experimental_editor,
            global: value.global.into(),
            splash: value.splash.into(),
            explorer: value.explorer.into(),
            outline: value.outline.into(),
            help_modal: value.help_modal.into(),
            note_editor: value.note_editor.into(),
            vault_selector_modal: value.vault_selector_modal.into(),
        }
    }
}

impl From<TomlConfigSection> for ConfigSection<'_> {
    fn from(TomlConfigSection { key_bindings }: TomlConfigSection) -> Self {
        Self {
            key_bindings: key_bindings
                .into_iter()
                .map(|KeyBinding { key, command }| (key.to_string(), command.into()))
                .collect(),
        }
    }
}

impl Config<'_> {
    /// Takes self and another config and merges the `key_bindings` together overwriting the
    /// existing entries with the value from another config.
    pub(crate) fn merge(&mut self, config: Self) -> Self {
        self.experimental_editor = config.experimental_editor;
        self.global.merge_key_bindings(config.global);
        self.explorer.merge_key_bindings(config.explorer);
        self.splash.merge_key_bindings(config.splash);
        self.note_editor.merge_key_bindings(config.note_editor);
        self.help_modal.merge_key_bindings(config.help_modal);
        self.vault_selector_modal
            .merge_key_bindings(config.vault_selector_modal);
        self.clone()
    }
}

impl fmt::Display for Config<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "[global]\n{}", self.global)?;
        writeln!(f, "[splash]\n{}", self.splash)?;
        writeln!(f, "[explorer]\n{}", self.explorer)?;
        writeln!(f, "[note_editor]\n{}", self.note_editor)?;
        writeln!(f, "[help_modal]\n{}", self.help_modal)?;
        writeln!(f, "[vault_selector_modal]\n{}", self.vault_selector_modal)?;

        Ok(())
    }
}

impl<'a> From<BTreeMap<String, Message<'a>>> for ConfigSection<'a> {
    fn from(value: BTreeMap<String, Message<'a>>) -> Self {
        Self {
            key_bindings: value,
        }
    }
}

impl<'a, const N: usize> From<[(String, Message<'a>); N]> for ConfigSection<'a> {
    fn from(value: [(String, Message<'a>); N]) -> Self {
        BTreeMap::from(value).into()
    }
}

#[derive(Clone, Debug, PartialEq, Deserialize, Default)]
struct TomlConfigSection {
    #[serde(default)]
    key_bindings: KeyBindings,
}

#[derive(Clone, Debug, PartialEq, Deserialize, Default)]
struct KeyBindings(Vec<KeyBinding>);

impl IntoIterator for KeyBindings {
    type Item = KeyBinding;
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl AsRef<Vec<KeyBinding>> for KeyBindings {
    fn as_ref(&self) -> &Vec<KeyBinding> {
        &self.0
    }
}

impl<const N: usize> From<[(Key, Command); N]> for KeyBindings {
    fn from(value: [(Key, Command); N]) -> Self {
        Self(value.into_iter().map(KeyBinding::from).collect())
    }
}

#[derive(Clone, Debug, PartialEq, Deserialize, Default)]
struct TomlConfig {
    #[serde(default)]
    experimental_editor: bool,
    #[serde(default)]
    global: TomlConfigSection,
    #[serde(default)]
    splash: TomlConfigSection,
    #[serde(default)]
    explorer: TomlConfigSection,
    #[serde(default)]
    outline: TomlConfigSection,
    #[serde(default)]
    help_modal: TomlConfigSection,
    #[serde(default)]
    note_editor: TomlConfigSection,
    #[serde(default)]
    vault_selector_modal: TomlConfigSection,
}

/// Finds and reads the user configuration file in order of priority.
///
/// The function checks two standard locations:
///
/// 1. Directly under the user's home directory: `$HOME/.basalt.toml`
/// 2. Under the user's config directory: `$HOME/.config/basalt/config.toml`
///
/// It first attempts to find the config file in the home directory. If not found, it then checks
/// the config directory.
fn read_user_config<'a>() -> Result<Config<'a>, ConfigError> {
    let home_dir_path = home_dir().map(|home_dir| home_dir.join(".basalt.toml"));
    let config_dir_path =
        choose_base_strategy().map(|strategy| strategy.config_dir().join("basalt/config.toml"));

    let config_path = [home_dir_path, config_dir_path]
        .into_iter()
        .flatten()
        .find(|path| path.exists())
        .ok_or(ConfigError::UserConfigNotFound(
            "Could not find user config".to_string(),
        ))?;

    toml::from_str::<TomlConfig>(&read_to_string(config_path)?)
        .map(Config::from)
        .map_err(ConfigError::from)
}

const BASE_CONFIGURATION_STR: &str =
    include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/config.toml"));

/// Loads and merges configuration from multiple sources in priority order.
///
/// The configuration is built by layering sources with increasing precedence:
/// 1. Base configuration from embedded config.toml (lowest priority)
/// 2. User-specific configuration from user's config directory
/// 3. System overrides (Ctrl+C) that cannot be changed by users (highest priority)
///
/// # Configuration Precedence
/// System overrides > User config > Base config
pub fn load<'a>() -> Result<Config<'a>, ConfigError> {
    // TODO: Use compile time toml parsing instead to check the build error during compile time
    // Requires a custom proc-macro workspace crate
    let mut base_config: Config = toml::from_str::<TomlConfig>(BASE_CONFIGURATION_STR)?.into();

    // TODO: Parsing errors related to the configuration file should ideally be surfaced as warnings.
    // This is pending a solution for toast notifications and proper warning/error logging.
    if let Ok(user_config) = read_user_config() {
        base_config.merge(user_config);
    }

    let system_key_binding_overrides: ConfigSection =
        [(Key::CTRL_C.to_string(), Message::Quit)].into();

    base_config
        .global
        .merge_key_bindings(system_key_binding_overrides);

    Ok(base_config)
}

#[cfg(test)]
mod tests {
    use ratatui::crossterm::event::KeyModifiers;

    use super::*;
    // use insta::assert_snapshot;

    #[test]
    fn test_base_config_snapshot() {
        // TODO: Does not work cross-platform as macOS has different names for the keys
        // Potentially needs two snapshots
        //
        // let config: Config = toml::from_str::<TomlConfig>(BASE_CONFIGURATION_STR)
        //     .unwrap()
        //     .into();
        //
        // assert_snapshot!(format!("{:?}", config));
    }

    #[test]
    fn test_config() {
        use key_binding::Key;

        let dummy_toml = r#"
        [global]
        key_bindings = [
         { key = "q", command = "quit" },
         { key = "ctrl+g", command = "vault_selector_modal_toggle" },
         { key = "?", command = "help_modal_toggle" },
        ]
    "#;
        let dummy_toml_config: TomlConfig = toml::from_str::<TomlConfig>(dummy_toml).unwrap();

        let expected_toml_config = TomlConfig {
            global: TomlConfigSection {
                key_bindings: [
                    (Key::from('q'), Command::Quit),
                    (
                        Key::from(('g', KeyModifiers::CONTROL)),
                        Command::VaultSelectorModalToggle,
                    ),
                    (Key::from('?'), Command::HelpModalToggle),
                ]
                .into(),
            },
            ..Default::default()
        };

        assert_eq!(dummy_toml_config, expected_toml_config);

        let expected_config = Config::default().merge(expected_toml_config.into());

        assert_eq!(
            Config::default().merge(Config::from(dummy_toml_config)),
            expected_config
        );
    }
}
