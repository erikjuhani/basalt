mod env;
mod key_binding;
pub mod symbol;
pub mod theme;

use core::fmt;
use std::{collections::BTreeMap, fs::read_to_string};

use etcetera::{choose_base_strategy, home_dir, BaseStrategy};
use key_binding::{KeyBinding, KeySpec, Leader};
use serde::Deserialize;

use crate::{app::Message, command::Command};

pub(crate) use key_binding::{Key, Keystroke};
pub(crate) use symbol::Symbols;
pub(crate) use theme::Theme;

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
    #[error("Invalid config: {0}")]
    InvalidConfig(String),
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

    /// Replaces this section's key_bindings entirely with those from another config.
    pub(crate) fn replace_key_bindings(&mut self, config: Self) {
        if !config.key_bindings.is_empty() {
            self.key_bindings = config.key_bindings;
        }
    }

    pub fn sequence_to_message(&self, keys: &[Keystroke]) -> Option<Message<'_>> {
        let s: String = keys.iter().map(|k| k.to_string()).collect();
        self.key_bindings.get(&s).cloned()
    }

    pub fn is_sequence_prefix(&self, keys: &[Keystroke]) -> bool {
        let s: String = keys.iter().map(|k| k.to_string()).collect();

        self.key_bindings
            .keys()
            .any(|k| k.starts_with(&s) && k.len() > s.len())
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

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LineNumbers {
    #[default]
    Absolute,
    Relative,
    Off,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Config<'a> {
    pub experimental_editor: bool,
    pub vim_mode: bool,
    pub wrap: bool,
    pub line_numbers: LineNumbers,
    pub symbols: Symbols,
    pub theme: Theme,
    pub global: ConfigSection<'a>,
    pub splash: ConfigSection<'a>,
    pub explorer: ConfigSection<'a>,
    pub outline: ConfigSection<'a>,
    pub input_modal: ConfigSection<'a>,
    pub help_modal: ConfigSection<'a>,
    pub note_editor: ConfigSection<'a>,
    pub vault_selector_modal: ConfigSection<'a>,
    pub debug_log_modal: ConfigSection<'a>,
    pub theme_selector_modal: ConfigSection<'a>,
}

impl Default for Config<'_> {
    fn default() -> Self {
        Self::from(TomlConfig::default())
    }
}

/// Resolves `<leader>` with the config's own leader. Layered sources are
/// converted with [`Config::from_toml`] instead, so that the leader chosen by
/// the user also applies to the bundled presets.
impl From<TomlConfig> for Config<'_> {
    fn from(value: TomlConfig) -> Self {
        let leader = value.leader.clone();
        Config::from_toml(value, &leader)
    }
}

impl ConfigSection<'_> {
    fn from_toml(TomlConfigSection { key_bindings }: TomlConfigSection, leader: &Leader) -> Self {
        Self {
            key_bindings: key_bindings
                .into_iter()
                .map(|KeyBinding { key, command }| {
                    (key.resolve(leader).to_string(), command.into())
                })
                .collect(),
        }
    }
}

impl Config<'_> {
    fn from_toml(value: TomlConfig, leader: &Leader) -> Self {
        Self {
            symbols: value.symbols.into(),
            theme: theme::theme_by_name(value.theme.as_deref().unwrap_or("default")),
            experimental_editor: value.experimental_editor,
            vim_mode: value.vim_mode,
            wrap: value.wrap.unwrap_or(true),
            line_numbers: value.line_numbers,
            global: ConfigSection::from_toml(value.global, leader),
            splash: ConfigSection::from_toml(value.splash, leader),
            explorer: ConfigSection::from_toml(value.explorer, leader),
            outline: ConfigSection::from_toml(value.outline, leader),
            input_modal: ConfigSection::from_toml(value.input_modal, leader),
            help_modal: ConfigSection::from_toml(value.help_modal, leader),
            note_editor: ConfigSection::from_toml(value.note_editor, leader),
            vault_selector_modal: ConfigSection::from_toml(value.vault_selector_modal, leader),
            debug_log_modal: ConfigSection::from_toml(value.debug_log_modal, leader),
            theme_selector_modal: ConfigSection::from_toml(value.theme_selector_modal, leader),
        }
    }

    /// Takes self and another config and merges the `key_bindings` together overwriting the
    /// existing entries with the value from another config.
    pub(crate) fn merge(&mut self, config: Self) -> Self {
        self.symbols = config.symbols;
        self.theme = config.theme;
        self.experimental_editor = config.experimental_editor;
        self.vim_mode = config.vim_mode;
        self.wrap = config.wrap;
        self.line_numbers = config.line_numbers;
        self.global.merge_key_bindings(config.global);
        self.explorer.merge_key_bindings(config.explorer);
        self.splash.merge_key_bindings(config.splash);
        self.outline.merge_key_bindings(config.outline);
        self.input_modal.merge_key_bindings(config.input_modal);
        self.note_editor.merge_key_bindings(config.note_editor);
        self.help_modal.merge_key_bindings(config.help_modal);
        self.vault_selector_modal
            .merge_key_bindings(config.vault_selector_modal);
        self.debug_log_modal
            .merge_key_bindings(config.debug_log_modal);
        self.theme_selector_modal
            .merge_key_bindings(config.theme_selector_modal);
        self.clone()
    }

    /// Replaces key_bindings for each section that has bindings defined in the given config.
    /// Sections with no bindings in the given config are left unchanged.
    pub(crate) fn replace(&mut self, config: Self) -> Self {
        self.global.replace_key_bindings(config.global);
        self.explorer.replace_key_bindings(config.explorer);
        self.splash.replace_key_bindings(config.splash);
        self.outline.replace_key_bindings(config.outline);
        self.input_modal.replace_key_bindings(config.input_modal);
        self.note_editor.replace_key_bindings(config.note_editor);
        self.help_modal.replace_key_bindings(config.help_modal);
        self.vault_selector_modal
            .replace_key_bindings(config.vault_selector_modal);
        self.debug_log_modal
            .replace_key_bindings(config.debug_log_modal);
        self.theme_selector_modal
            .replace_key_bindings(config.theme_selector_modal);
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
        writeln!(f, "[debug_log_modal]\n{}", self.debug_log_modal)?;

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
        Self(
            value
                .into_iter()
                .map(|(key, command)| KeyBinding::new(KeySpec::from(key), command))
                .collect(),
        )
    }
}

#[derive(Clone, Debug, PartialEq, Deserialize, Default)]
struct TomlConfig {
    #[serde(default)]
    symbols: symbol::TomlSymbols,
    #[serde(default)]
    theme: Option<String>,
    #[serde(default)]
    experimental_editor: bool,
    #[serde(default)]
    vim_mode: bool,
    #[serde(default)]
    wrap: Option<bool>,
    #[serde(default)]
    line_numbers: LineNumbers,
    #[serde(default)]
    leader: Leader,
    #[serde(default)]
    global: TomlConfigSection,
    #[serde(default)]
    splash: TomlConfigSection,
    #[serde(default)]
    explorer: TomlConfigSection,
    #[serde(default)]
    outline: TomlConfigSection,
    #[serde(default)]
    input_modal: TomlConfigSection,
    #[serde(default)]
    help_modal: TomlConfigSection,
    #[serde(default)]
    note_editor: TomlConfigSection,
    #[serde(default)]
    vault_selector_modal: TomlConfigSection,
    #[serde(default)]
    debug_log_modal: TomlConfigSection,
    #[serde(default)]
    theme_selector_modal: TomlConfigSection,
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
fn read_user_config() -> Result<TomlConfig, ConfigError> {
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
        .map_err(|err| ConfigError::InvalidConfig(err.message().to_string()))
}

/// The path the user config should be written to: an existing config if there
/// is one, otherwise `$config/basalt/config.toml`.
fn user_config_write_path() -> Option<std::path::PathBuf> {
    let home = home_dir().ok().map(|home| home.join(".basalt.toml"));
    let config = choose_base_strategy()
        .ok()
        .map(|strategy| strategy.config_dir().join("basalt/config.toml"));

    [home.clone(), config.clone()]
        .into_iter()
        .flatten()
        .find(|path| path.exists())
        .or(config)
        .or(home)
}

/// Sets the top-level `theme` key, preserving the rest of the config (comments,
/// formatting and ordering) by editing the TOML document in place.
fn upsert_theme(content: &str, name: &str) -> Result<String, ConfigError> {
    let mut config = content
        .parse::<toml_edit::DocumentMut>()
        .map_err(|error| ConfigError::InvalidConfig(error.to_string()))?;
    config["theme"] = toml_edit::value(name);
    Ok(config.to_string())
}

/// Persists the chosen theme to the user config so it loads on the next run.
pub fn save_theme(name: &str) -> Result<std::path::PathBuf, ConfigError> {
    let path = user_config_write_path().ok_or(ConfigError::UserConfigNotFound(
        "Could not determine a config location".to_string(),
    ))?;

    // Only a missing file is empty; a read that fails for any other reason must
    // abort rather than clobber the user's existing config with just the theme.
    let existing = match read_to_string(&path) {
        Ok(content) => content,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(error) => return Err(error.into()),
    };
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&path, upsert_theme(&existing, name)?)?;
    Ok(path)
}

const BASE_CONFIGURATION_STR: &str =
    include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/config.toml"));

const VIM_CONFIGURATION_STR: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/vim.toml"));

/// Loads and merges configuration from multiple sources in priority order.
///
/// The configuration is built by layering sources with increasing precedence:
/// 1. Base configuration from embedded config.toml (lowest priority)
/// 2. User-specific configuration from user's config directory
/// 3. System overrides (Ctrl+C) that cannot be changed by users (highest priority)
///
/// # Configuration Precedence
/// System overrides > User config > Base config
///
/// The leader key is taken from the user config alone and applied to every
/// layer, so `<leader>` means the same key in the bundled presets as it does in
/// the user's own bindings.
pub fn load<'a>() -> Result<(Config<'a>, Vec<String>), ConfigError> {
    let (user_config, warnings) = match read_user_config() {
        Ok(config) => (Some(config), vec![]),
        Err(ConfigError::UserConfigNotFound(_)) => (None, vec![]),
        Err(err) => (None, vec![err.to_string()]),
    };

    let leader = user_config
        .as_ref()
        .map(|user| user.leader.clone())
        .unwrap_or_default();

    // TODO: Use compile time toml parsing instead to check the build error during compile time
    // Requires a custom proc-macro workspace crate
    let mut config = Config::from_toml(
        toml::from_str::<TomlConfig>(BASE_CONFIGURATION_STR)?,
        &leader,
    );

    if config.symbols.preset == symbol::Preset::Auto {
        config.symbols.preset = symbol::detect_preset(env::SystemEnv)
    }

    if user_config.as_ref().is_some_and(|user| user.vim_mode) {
        let vim_config = toml::from_str::<TomlConfig>(VIM_CONFIGURATION_STR)
            .map_err(ConfigError::from)
            .map(|vim| Config::from_toml(vim, &leader))?;
        config.replace(vim_config);
    }

    if let Some(user) = user_config {
        config.merge(Config::from_toml(user, &leader));
    }

    let system_key_binding_overrides: ConfigSection =
        [(Key::CTRL_C.to_string(), Message::Quit)].into();

    config
        .global
        .merge_key_bindings(system_key_binding_overrides);

    Ok((config, warnings))
}

#[cfg(test)]
mod tests {
    use std::slice;

    use ratatui::crossterm::event::{KeyCode, KeyModifiers};
    use similar_asserts::assert_eq;

    use super::*;
    // use insta::assert_snapshot;

    fn theme_of(config: &str) -> Option<String> {
        toml::from_str::<toml::Value>(config)
            .unwrap()
            .get("theme")
            .and_then(toml::Value::as_str)
            .map(str::to_string)
    }

    #[test]
    fn upsert_theme_replaces_existing_key_and_keeps_comments() {
        let updated =
            upsert_theme("# keep me\ntheme = \"default\"\nvim_mode = true\n", "nord").unwrap();
        assert_eq!(theme_of(&updated).as_deref(), Some("nord"));
        assert!(updated.contains("# keep me"));
        assert!(updated.contains("vim_mode = true"));
    }

    #[test]
    fn upsert_theme_adds_missing_key_at_top_level() {
        let updated = upsert_theme("vim_mode = false\n\n[global]\nkey = 1\n", "nord").unwrap();
        let document = toml::from_str::<toml::Value>(&updated).unwrap();
        assert_eq!(
            document.get("theme").and_then(toml::Value::as_str),
            Some("nord")
        );
        // The key must land top-level, not reparented under [global].
        assert!(document["global"].get("theme").is_none());
        assert_eq!(document["global"]["key"].as_integer(), Some(1));
    }

    #[test]
    fn upsert_theme_ignores_commented_key() {
        let updated = upsert_theme("# theme = \"default\"\n", "nord").unwrap();
        assert_eq!(theme_of(&updated).as_deref(), Some("nord"));
        assert!(updated.contains("# theme = \"default\""));
    }

    #[test]
    fn upsert_theme_writes_into_empty_config() {
        assert_eq!(
            theme_of(&upsert_theme("", "nord").unwrap()).as_deref(),
            Some("nord")
        );
    }

    #[test]
    fn upsert_theme_rejects_malformed_config() {
        assert!(upsert_theme("this is = = not toml", "nord").is_err());
    }

    #[test]
    fn test_base_config_parses() {
        // Guards against a binding in the bundled config.toml that the key parser
        // rejects, which would panic at startup via `load().unwrap()`.
        toml::from_str::<TomlConfig>(BASE_CONFIGURATION_STR)
            .map(Config::from)
            .expect("bundled config.toml should parse");
    }

    #[test]
    fn test_vim_config_parses() {
        // Same guard for the vim overrides, which `load()` merges when vim mode
        // is on and would otherwise only fail at startup for vim users.
        toml::from_str::<TomlConfig>(VIM_CONFIGURATION_STR)
            .map(Config::from)
            .expect("bundled vim.toml should parse");
    }

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
    fn test_leader_key_bindings() {
        let dummy_toml = r#"
        leader = ","

        [global]
        key_bindings = [
         { key = "<leader>q", command = "quit" },
        ]
    "#;
        let config = Config::from(toml::from_str::<TomlConfig>(dummy_toml).unwrap());
        let leader = Keystroke::from(KeyCode::Char(','));
        let q = Keystroke::from(KeyCode::Char('q'));

        assert!(config.global.is_sequence_prefix(slice::from_ref(&leader)));
        assert_eq!(
            config.global.sequence_to_message(&[leader, q]),
            Some(Message::Quit)
        );
    }

    #[test]
    fn test_leader_applies_to_every_layer() {
        // The user's leader has to reach the bundled presets too, otherwise a
        // `<leader>` binding shipped with basalt would answer to a different key
        // than the user's own bindings.
        let leader = Leader::from(Key::from(','));
        let preset = r#"
        [explorer]
        key_bindings = [
         { key = "<leader>s", command = "explorer_sort" },
        ]
    "#;
        let config = Config::from_toml(toml::from_str::<TomlConfig>(preset).unwrap(), &leader);
        let keys = [
            Keystroke::from(KeyCode::Char(',')),
            Keystroke::from(KeyCode::Char('s')),
        ];

        assert_eq!(
            config.explorer.sequence_to_message(&keys),
            Some(Message::Explorer(crate::explorer::Message::Sort))
        );
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
