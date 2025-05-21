mod key;
mod key_str;
#[cfg(test)]
mod test;

use core::fmt;
use std::{collections::BTreeMap, fs::read_to_string, result};

use crossterm::event::{KeyCode, KeyModifiers};
use etcetera::{choose_base_strategy, home_dir, BaseStrategy};
use key::Key;
use serde::Deserialize;

use crate::app::{Action, ScrollAmount};

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct KeyBinding {
    pub key: Key,
    pub command: Command,
}

impl fmt::Display for KeyBinding {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.key)
    }
}

pub const CTRL_C: KeyBinding = KeyBinding {
    key: Key {
        modifiers: KeyModifiers::CONTROL,
        code: KeyCode::Char('c'),
    },
    command: Command::Quit,
};

#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Command {
    ScrollUp,
    ScrollDown,
    PageUp,
    PageDown,
    Next,
    Previous,
    Quit,
    Select,
    ToggleHelp,
    ToggleMode,
    ToggleVaultSelector,
}

impl From<Command> for Action {
    fn from(value: Command) -> Self {
        match value {
            Command::ScrollUp => Action::ScrollUp(ScrollAmount::One),
            Command::ScrollDown => Action::ScrollDown(ScrollAmount::One),
            Command::PageUp => Action::ScrollUp(ScrollAmount::HalfPage),
            Command::PageDown => Action::ScrollDown(ScrollAmount::HalfPage),
            Command::Next => Action::Next,
            Command::Previous => Action::Prev,
            Command::Quit => Action::Quit,
            Command::Select => Action::Select,
            Command::ToggleHelp => Action::ToggleHelp,
            Command::ToggleMode => Action::ToggleMode,
            Command::ToggleVaultSelector => Action::ToggleVaultSelector,
        }
    }
}

impl From<&KeyBinding> for Action {
    fn from(value: &KeyBinding) -> Self {
        value.command.clone().into()
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Config {
    pub key_bindings: BTreeMap<String, KeyBinding>,
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

    pub fn get_key_binding(&self, key: Key) -> Option<&KeyBinding> {
        self.key_bindings.get(&key.to_string())
    }
}

impl<const N: usize> From<[KeyBinding; N]> for Config {
    fn from(value: [KeyBinding; N]) -> Self {
        Self {
            key_bindings: BTreeMap::from(
                value.map(|key_binding| (key_binding.to_string(), key_binding)),
            ),
        }
    }
}

impl From<BTreeMap<String, KeyBinding>> for Config {
    fn from(value: BTreeMap<String, KeyBinding>) -> Self {
        Self {
            key_bindings: value,
        }
    }
}

impl From<TomlConfig> for Config {
    fn from(value: TomlConfig) -> Self {
        Self {
            key_bindings: value
                .key_bindings
                .into_iter()
                .map(|key_binding| (key_binding.key.to_string(), key_binding))
                .collect(),
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
fn read_user_config() -> Option<TomlConfig> {
    let config_path = home_dir()
        .map(|home_dir| home_dir.join(".basalt.toml"))
        .or_else(|_| {
            choose_base_strategy().map(|strategy| strategy.config_dir().join("basalt/config.toml"))
        })
        .ok()?;

    // TODO: Parsing errors related to the configuration file should ideally be surfaced as warnings.
    // This is pending a solution for toast notifications and proper warning/error logging.
    toml::from_str::<TomlConfig>(read_to_string(config_path).unwrap_or_default().as_str()).ok()
}

pub fn read_config() -> Result<Config> {
    let default_config: Config =
        toml::from_str::<TomlConfig>(include_str!("../../config.toml"))?.into();

    let constant_config: Config = BTreeMap::from([(CTRL_C.to_string(), CTRL_C)]).into();

    Ok(default_config
        .merge(read_user_config().unwrap_or_default().into())
        .merge(constant_config))
}

/// A [`std::result::Result`] type for fallible operations in [`crate::config`].
///
/// For convenience of use and to avoid writing [`Error`] directly. All fallible operations return
/// [`Error`] as the error variant.
pub type Result<T> = result::Result<T, ConfigError>;

/// Error type for fallible operations in this [`crate`].
///
/// Implements [`std::error::Error`] via [thiserror](https://docs.rs/thiserror).
#[derive(thiserror::Error, Debug)]
pub enum ConfigError {
    /// TOML (De)serialization error, from [`toml::de::Error`].
    #[error("Toml (de)serialization error: {0}")]
    Toml(#[from] toml::de::Error),

    #[error("Invalid key binding: {0}")]
    InvalidKeybinding(String),

    #[error("Invalid key code: {0}")]
    InvalidKeyCode(String),
}

// let mut config = toml::toml! {
//     key_bindings = [
//       { key = "q",       command = "quit" },
//       { key = "?",       command = "toggle_help" },
//       { key = " ",       command = "toggle_vault_selector" },
//       { key = "t",       command = "toggle_mode" },
//       { key = "up",      command = "scroll_up" },
//       { key = "down",    command = "scroll_down" },
//       { key = "ctrl+u",  command = "page_up" },
//       { key = "ctrl+d",  command = "page_down" },
//       { key = "k",       command = "previous" },
//       { key = "j",       command = "next" },
//       { key = "enter",   command = "select" }
//     ]
// };
