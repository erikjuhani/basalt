mod key_binding;

use std::{collections::BTreeMap, fs::read_to_string};

use etcetera::{choose_base_strategy, home_dir, BaseStrategy};
use key_binding::KeyBinding;
use serde::Deserialize;

use crate::app::ScrollAmount;
pub(crate) use key_binding::Key;

#[derive(Clone, Debug, PartialEq)]
pub enum Action {
    Select,
    Next,
    Prev,
    Insert,
    ScrollUp(ScrollAmount),
    ScrollDown(ScrollAmount),
    ToggleMode,
    ToggleHelp,
    ToggleVaultSelector,
    Quit,
}

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
pub struct Config {
    pub key_bindings: BTreeMap<String, Action>,
}

impl Default for Config {
    fn default() -> Self {
        Self::from(TomlConfig::default())
    }
}

impl From<TomlConfig> for Config {
    fn from(TomlConfig { key_bindings }: TomlConfig) -> Self {
        Self {
            key_bindings: key_bindings
                .into_iter()
                .map(|KeyBinding { key, command }| (key.to_string(), command.into()))
                .collect(),
        }
    }
}

impl Config {
    /// Takes self and another config and merges the `key_bindings` together overwriting the
    /// existing entries with the value from another config.
    pub(crate) fn merge(&self, config: Config) -> Config {
        config
            .key_bindings
            .into_iter()
            .fold(self.key_bindings.clone(), |mut acc, (key, value)| {
                acc.entry(key)
                    .and_modify(|v| *v = value.clone())
                    .or_insert(value);
                acc
            })
            .into()
    }

    pub fn key_to_action(&self, key: Key) -> Option<Action> {
        self.key_bindings
            .get(&key.to_string())
            .cloned()
            .or_else(|| key.eq(&Key::CTRLC).then_some(Action::Quit))
    }
}

impl From<BTreeMap<String, Action>> for Config {
    fn from(value: BTreeMap<String, Action>) -> Self {
        Self {
            key_bindings: value,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Deserialize, Default)]
struct TomlConfig {
    #[serde(default)]
    key_bindings: Vec<KeyBinding>,
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
fn read_user_config() -> Result<Config, ConfigError> {
    let home_dir_path = home_dir().map(|home_dir| home_dir.join(".basalt.toml"));
    let config_dir_path =
        choose_base_strategy().map(|strategy| strategy.config_dir().join("basalt/config.toml"));

    let config_path = [home_dir_path, config_dir_path]
        .into_iter()
        .flatten()
        .next()
        .ok_or(ConfigError::UserConfigNotFound(
            "Could not find user config".to_string(),
        ));

    config_path.and_then(|path_buf| {
        read_to_string(path_buf)
            .map_err(ConfigError::from)
            .and_then(|content| {
                toml::from_str::<TomlConfig>(&content)
                    .map(Config::from)
                    .map_err(ConfigError::from)
            })
    })
}

/// Loads and merges configuration from multiple sources in priority order.
///
/// The configuration is built by layering sources with increasing precedence:
/// 1. Base configuration from embedded config.toml (lowest priority)
/// 2. User-specific configuration from user's config directory
/// 3. System overrides (Ctrl+C) that cannot be changed by users (highest priority)
///
/// # Configuration Precedence
/// System overrides > User config > Base config
pub fn load() -> Result<Config, ConfigError> {
    // TODO: Use static-toml instead to check the build error during compile time
    let base_config: Config =
        toml::from_str::<TomlConfig>(include_str!("../../config.toml"))?.into();

    let system_overrides: Config = BTreeMap::from([(Key::CTRLC.to_string(), Action::Quit)]).into();

    // TODO: Parsing errors related to the configuration file should ideally be surfaced as warnings.
    // This is pending a solution for toast notifications and proper warning/error logging.
    let user_config = read_user_config().unwrap_or_default();

    Ok(base_config.merge(user_config).merge(system_overrides))
}

#[test]
fn test_config() {
    use crossterm::event::{KeyCode, KeyModifiers};

    use key_binding::{Command, Key, KeyBinding};

    let dummy_toml = r#"
        [[key_bindings]]
        key = "page_down"
        command = "page_down"

        [[key_bindings]]
        key = "page_up"
        command = "page_up"
    "#;
    let dummy_toml_config: TomlConfig = toml::from_str::<TomlConfig>(dummy_toml).unwrap();

    let expected_toml_config = TomlConfig {
        key_bindings: Vec::from([
            KeyBinding {
                key: Key {
                    code: KeyCode::PageDown,
                    modifiers: KeyModifiers::NONE,
                },
                command: Command::PageDown,
            },
            KeyBinding {
                key: Key {
                    code: KeyCode::PageUp,
                    modifiers: KeyModifiers::NONE,
                },
                command: Command::PageUp,
            },
        ]),
    };

    assert_eq!(dummy_toml_config, expected_toml_config);

    let expected_config = Config::default().merge(
        TomlConfig {
            key_bindings: Vec::from([
                KeyBinding {
                    key: Key {
                        code: KeyCode::PageUp,
                        modifiers: KeyModifiers::NONE,
                    },
                    command: Command::PageUp,
                },
                KeyBinding {
                    key: Key {
                        code: KeyCode::PageDown,
                        modifiers: KeyModifiers::NONE,
                    },
                    command: Command::PageDown,
                },
            ]),
        }
        .into(),
    );

    assert_eq!(
        Config::default().merge(Config::from(dummy_toml_config)),
        expected_config
    );
}
