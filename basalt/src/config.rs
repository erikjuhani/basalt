use std::io::Read;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::app::{Action, ScrollAmount};

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Config {
    pub keymap: KeyMap,
}

impl Config {
    pub fn build() -> Self {
        TomlConfig::open().unwrap_or_default().into()
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct KeyMap(std::collections::HashMap<KeyBinding, Action>);

impl std::ops::Deref for KeyMap {
    type Target = std::collections::HashMap<KeyBinding, Action>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<KeyBindings> for KeyMap {
    fn from(
        KeyBindings {
            quit,
            help,
            vault_selector,
            up,
            down,
            page_up,
            page_down,
            toggle_mode,
            previous_entry,
            next_entry,
            enter,
        }: KeyBindings,
    ) -> Self {
        Self(std::collections::HashMap::from([
            (
                quit.as_deref().unwrap_or("q").parse().unwrap(),
                Action::Quit,
            ),
            (
                help.as_deref().unwrap_or("?").parse().unwrap(),
                Action::ToggleHelp,
            ),
            (
                vault_selector.as_deref().unwrap_or(" ").parse().unwrap(),
                Action::ToggleVaultSelector,
            ),
            (
                up.as_deref().unwrap_or("up").parse().unwrap(),
                Action::ScrollUp(ScrollAmount::One),
            ),
            (
                down.as_deref().unwrap_or("down").parse().unwrap(),
                Action::ScrollDown(ScrollAmount::One),
            ),
            (
                page_up.as_deref().unwrap_or("ctrl+u").parse().unwrap(),
                Action::ScrollUp(ScrollAmount::HalfPage),
            ),
            (
                page_down.as_deref().unwrap_or("ctrl+d").parse().unwrap(),
                Action::ScrollDown(ScrollAmount::HalfPage),
            ),
            (
                toggle_mode
                    .as_deref()
                    .unwrap_or("toggle_mode")
                    .parse()
                    .unwrap(),
                Action::ToggleMode,
            ),
            (
                previous_entry.as_deref().unwrap_or("k").parse().unwrap(),
                Action::Prev,
            ),
            (
                next_entry.as_deref().unwrap_or("j").parse().unwrap(),
                Action::Next,
            ),
            (
                enter.as_deref().unwrap_or("enter").parse().unwrap(),
                Action::Select,
            ),
        ]))
    }
}

impl From<TomlConfig> for Config {
    fn from(TomlConfig { keybindings }: TomlConfig) -> Self {
        Self {
            keymap: keybindings.into(),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, serde::Deserialize)]
pub struct TomlConfig {
    pub keybindings: KeyBindings,
}

#[derive(Clone, Debug, Default, PartialEq, serde::Deserialize)]
pub struct KeyBindings {
    quit: Option<String>,
    help: Option<String>,
    vault_selector: Option<String>,
    up: Option<String>,
    down: Option<String>,
    page_up: Option<String>,
    page_down: Option<String>,
    toggle_mode: Option<String>,
    previous_entry: Option<String>,
    next_entry: Option<String>,
    enter: Option<String>,
}

impl TomlConfig {
    pub fn open() -> std::io::Result<Self> {
        let home = dirs::home_dir().ok_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::Other, "Cannot retrieve home directory")
        })?;

        std::fs::File::open(home.join(".basalt.toml"))
            .or_else(|_| std::fs::File::open(home.join(".config/basalt/basalt.toml")))
            .and_then(|mut file| {
                let mut buffer = String::new();
                file.read_to_string(&mut buffer)?;

                Ok(buffer)
            })
            .and_then(|toml| {
                toml::from_str::<TomlConfig>(&toml)
                    .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))
            })
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct KeyBinding {
    pub code: KeyCode,
    pub modifiers: KeyModifiers,
}

impl KeyBinding {
    pub const CTRLC: KeyBinding = KeyBinding {
        code: KeyCode::Char('c'),
        modifiers: KeyModifiers::CONTROL,
    };
}

impl Default for KeyBinding {
    fn default() -> Self {
        Self {
            code: KeyCode::Null,
            modifiers: KeyModifiers::NONE,
        }
    }
}

impl From<&KeyEvent> for KeyBinding {
    fn from(
        KeyEvent {
            code,
            modifiers,
            kind: _,
            state: _,
        }: &KeyEvent,
    ) -> Self {
        Self {
            code: *code,
            modifiers: *modifiers,
        }
    }
}

impl std::str::FromStr for KeyBinding {
    type Err = std::convert::Infallible;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let value = value.to_lowercase();
        let (modifier, key) = value
            .split_once('+')
            .map(|(modifier, key)| (Some(modifier), key))
            .unwrap_or((None, value.as_str()));
        let modifiers = match modifier {
            Some(modifier) => match modifier {
                "shift" => KeyModifiers::SHIFT,
                "ctrl" => KeyModifiers::CONTROL,
                "alt" => KeyModifiers::ALT,
                "super" => KeyModifiers::SUPER,
                "hyper" => KeyModifiers::HYPER,
                "meta" => KeyModifiers::META,
                _ => KeyModifiers::NONE,
            },
            _ => KeyModifiers::NONE,
        };

        let code = match key.len() {
            0 => KeyCode::Null,
            1 => KeyCode::Char(key.chars().next().unwrap()),
            _ => key
                .strip_prefix('f')
                .and_then(|n| n.parse::<u8>().map(KeyCode::F).ok())
                .unwrap_or(match key {
                    "backspace" => KeyCode::Backspace,
                    "enter" => KeyCode::Enter,
                    "left" => KeyCode::Left,
                    "right" => KeyCode::Right,
                    "up" => KeyCode::Up,
                    "down" => KeyCode::Down,
                    "home" => KeyCode::Home,
                    "end" => KeyCode::End,
                    "page_up" => KeyCode::PageUp,
                    "page_down" => KeyCode::PageDown,
                    "tab" => KeyCode::Tab,
                    "backtab" => KeyCode::BackTab,
                    "delete" => KeyCode::Delete,
                    "insert" => KeyCode::Insert,
                    _ => KeyCode::Null,
                }),
        };

        Ok(Self { modifiers, code })
    }
}
