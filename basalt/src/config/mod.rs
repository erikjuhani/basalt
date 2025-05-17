mod keybinding;
#[cfg(test)]
mod test;

use std::{collections::HashMap, fs::read_to_string, path::PathBuf, str::from_utf8};

use etcetera::{choose_base_strategy, home_dir, BaseStrategy};
use keybinding::KeyBinding;
use serde::Deserialize;

use crate::app::Action;

pub(crate) use keybinding::Key;

#[derive(Clone, Debug, PartialEq)]
pub struct Config {
    pub keymap: HashMap<Key, Action>,
}

impl Default for Config {
    fn default() -> Self {
        let TomlConfig { key_bindings } = TomlConfig::default();

        Self {
            keymap: key_bindings
                .expect("The built-in config.toml should contain key bindings")
                .into_iter()
                .map(|KeyBinding { key, command }| (key, command.into()))
                .collect(),
        }
    }
}

impl From<TomlConfig> for Config {
    fn from(TomlConfig { key_bindings }: TomlConfig) -> Self {
        let mut config = Self::default();
        config.merge_key_bindings(key_bindings);

        config
    }
}

impl Config {
    pub fn build() -> Self {
        TomlConfig::build().map_or_else(Self::default, Self::from)
    }
    fn merge_key_bindings(&mut self, key_bindings: Option<Vec<KeyBinding>>) {
        if let Some(key_bindings) = key_bindings {
            key_bindings
                .into_iter()
                .for_each(|KeyBinding { key, command }| {
                    let new_action: Action = command.into();
                    self.keymap
                        .entry(key)
                        .and_modify(|old_action| *old_action = new_action)
                        .or_insert(new_action);
                })
        }
    }
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
struct TomlConfig {
    key_bindings: Option<Vec<KeyBinding>>,
}

impl Default for TomlConfig {
    fn default() -> Self {
        let default_config = include_bytes!("../../../config.toml");
        toml::from_str(from_utf8(default_config).unwrap())
            .expect("Could not parse built-in config.toml into valid toml")
    }
}

impl TomlConfig {
    fn build() -> Option<Self> {
        let mut config = config_files().into_iter().filter(|file| file.exists());

        if config.clone().count() > 1 {
            panic!(
                "One config file is plenty! I suggest you get rid of the supplementary one.s in order to avoid potential conflicts:\n\t- {}",
                config.map(|path| path.to_string_lossy().into_owned()).collect::<Vec<_>>().join(",\n\t- ")
            )
        }

        config.next().map(|file| {
            let content = read_to_string(file).unwrap();
            toml::from_str(&content).unwrap()
        })
    }
}

fn config_files() -> [PathBuf; 3] {
    let home_dir = home_dir().expect("Couldn't determine the home directory!");
    [
        // $HOME_DIR/.basalt.toml
        home_dir.as_path().join(".basalt.toml"),
        // $HOME_DIR/.basalt/config.toml
        home_dir.as_path().join(".basalt/config.toml"),
        // $CONFIG_DIR/basalt/config.toml
        choose_base_strategy()
            .expect("Unable to find the config directory!")
            .config_dir()
            .join("basalt/config.toml"),
    ]
}
