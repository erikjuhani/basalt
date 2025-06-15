mod key_binding;

use std::{collections::HashMap, fs::read_to_string};

use etcetera::{choose_base_strategy, home_dir, BaseStrategy};
use key_binding::KeyBinding;
use serde::Deserialize;

pub(crate) use key_binding::Key;

use crate::app::ScrollAmount;

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
    Etcetera(#[from] etcetera::HomeDirError),
    /// TOML (De)serialization error, from [`toml::de::Error`].
    #[error(transparent)]
    Toml(#[from] toml::de::Error),
    #[error("Invalid keybinding: {0}")]
    InvalidKeybinding(String),
    #[error("Unknown code: {0}")]
    UnknownKeyCode(String),
    #[error("Unknown modifiers: {0}")]
    UnknownKeyModifiers(String),
}

#[derive(Clone, Debug, PartialEq)]
pub struct Config {
    pub keymap: HashMap<Key, Action>,
}

impl Default for Config {
    fn default() -> Self {
        Self::from(TomlConfig::default())
    }
}

impl From<TomlConfig> for Config {
    fn from(TomlConfig { key_bindings }: TomlConfig) -> Self {
        Self {
            keymap: key_bindings
                .into_iter()
                .map(|KeyBinding { key, command }| (key, command.into()))
                .collect(),
        }
    }
}

impl Config {
    pub fn build() -> Self {
        let mut config = Self::default();
        if let Ok(Some(user_config)) = TomlConfig::read_user_config() {
            config = config.merge(user_config.into());
        };
        config.keymap.insert(Key::CTRLC, Action::Quit);

        config
    }
    fn merge(self, Self { keymap }: Self) -> Self {
        self.merge_keymap(keymap)
    }
    fn merge_keymap(mut self, key_bindings: HashMap<Key, Action>) -> Self {
        key_bindings.into_iter().for_each(|(key, action)| {
            self.keymap
                .entry(key)
                .and_modify(|old_action| *old_action = action.clone())
                .or_insert(action);
        });

        self
    }
    pub fn key_to_action(&self, key: Key) -> Option<Action> {
        self.keymap
            .get(&key)
            .cloned()
            .or_else(|| key.eq(&Key::CTRLC).then_some(Action::Quit))
    }
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
struct TomlConfig {
    #[serde(default)]
    key_bindings: Vec<KeyBinding>,
}

impl Default for TomlConfig {
    fn default() -> Self {
        let default_config = include_str!("../../config.toml");
        toml::from_str(default_config)
            // I REALLY think this particular Default implementation should panic
            // in the event the 'config.toml' has been modified and cannot be properly parsed
            // so Basalt cannot build, should the default configuration be faulty
            .expect("Could not parse built-in config.toml into valid toml")
    }
}

impl TomlConfig {
    /// Finds and reads the user configuration file in order of priority.
    ///
    /// The function checks two standard locations:
    ///
    /// 1. Directly under the user's home directory: `$HOME/.basalt.toml`
    /// 2. Under the user's config directory: `$HOME/.config/basalt/config.toml`
    ///
    /// It first attempts to find the config file in the home directory. If not found, it then checks
    /// the config directory.
    ///
    // TODO: Parsing errors related to the configuration file should ideally be surfaced as warnings.
    // This is pending a solution for toast notifications and proper warning/error logging.
    fn read_user_config() -> Result<Option<TomlConfig>, ConfigError> {
        [
            home_dir()?.join(".basalt.toml"),
            choose_base_strategy()?
                .config_dir()
                .join("basalt/config.toml"),
        ]
        .into_iter()
        .find_map(|file| {
            file.exists().then(|| {
                read_to_string(file)
                    .map_err(ConfigError::from)
                    .and_then(|content| {
                        toml::from_str::<TomlConfig>(content.as_str()).map_err(ConfigError::from)
                    })
            })
        })
        .transpose()
    }
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
