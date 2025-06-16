use std::fmt;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use serde::{
    de::{self, Visitor},
    Deserialize, Deserializer,
};

use super::Action;
use crate::app::ScrollAmount;
use crate::config::ConfigError;

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub(crate) struct KeyBinding {
    pub key: Key,
    pub command: Command,
}

impl KeyBinding {
    #[expect(dead_code)]
    pub const CTRLC: KeyBinding = KeyBinding {
        key: Key::CTRLC,
        command: Command::Quit,
    };
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct Key {
    pub modifiers: KeyModifiers,
    pub code: KeyCode,
}

impl fmt::Display for Key {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let code = self.code.to_string().to_lowercase().replace(" ", "_");

        if self.modifiers.is_empty() {
            write!(f, "{}", code)
        } else {
            let modifiers = self
                .modifiers
                .iter_names()
                .map(|(name, _)| name.to_lowercase())
                .collect::<Vec<_>>()
                .join("+");

            write!(f, "{}+{}", code, modifiers)
        }
    }
}

impl Key {
    pub const CTRLC: Key = Key {
        modifiers: KeyModifiers::CONTROL,
        code: KeyCode::Char('c'),
    };
}

impl<'de> Deserialize<'de> for Key {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_str(KeyVisitor)
    }
}

struct KeyVisitor;

impl Visitor<'_> for KeyVisitor {
    type Value = Key;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a string whose format is either 'key' or 'modifier+key'")
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        let value = value.to_lowercase();
        let mut parts = value.split('+');
        // Does not panic if the str is empty
        let code = parts.by_ref().next_back().unwrap();
        let modifiers = parts
            .map(parse_modifiers)
            .collect::<Result<Vec<KeyModifiers>, ConfigError>>()
            .map_err(de::Error::custom)?
            .into_iter()
            .reduce(|acc, modifiers| acc.union(modifiers))
            .unwrap_or(KeyModifiers::NONE);

        Ok(Key {
            modifiers,
            code: parse_code(code).map_err(de::Error::custom)?,
        })
    }
}

fn parse_modifiers(modifiers: &str) -> Result<KeyModifiers, ConfigError> {
    match modifiers {
        "" => Ok(KeyModifiers::NONE),
        "alt" => Ok(KeyModifiers::ALT),
        "ctrl" | "control" => Ok(KeyModifiers::CONTROL),
        "hyper" => Ok(KeyModifiers::HYPER),
        "meta" => Ok(KeyModifiers::META),
        "shift" => Ok(KeyModifiers::SHIFT),
        "super" => Ok(KeyModifiers::SUPER),
        _ => Err(ConfigError::UnknownKeyModifiers(modifiers.to_string())),
    }
}
fn parse_code(code: &str) -> Result<KeyCode, ConfigError> {
    match code.len() {
        0 => Some(KeyCode::Null),
        1 => Some(KeyCode::Char(code.chars().next().unwrap())),
        _ => code
            .strip_prefix('f')
            .and_then(|n| n.parse::<u8>().map(KeyCode::F).ok())
            .or(match code {
                "backspace" => Some(KeyCode::Backspace),
                "backtab" => Some(KeyCode::BackTab),
                "delete" => Some(KeyCode::Delete),
                "down" => Some(KeyCode::Down),
                "end" => Some(KeyCode::End),
                "enter" => Some(KeyCode::Enter),
                "home" => Some(KeyCode::Home),
                "insert" => Some(KeyCode::Insert),
                "left" => Some(KeyCode::Left),
                "page_down" => Some(KeyCode::PageDown),
                "page_up" => Some(KeyCode::PageUp),
                "right" => Some(KeyCode::Right),
                "tab" => Some(KeyCode::Tab),
                "up" => Some(KeyCode::Up),
                _ => None,
            }),
    }
    .ok_or(ConfigError::UnknownKeyCode(code.to_string()))
}

impl de::Error for ConfigError {
    fn custom<T>(msg: T) -> Self
    where
        T: fmt::Display,
    {
        ConfigError::InvalidKeybinding(msg.to_string())
    }
}

impl From<&KeyEvent> for Key {
    fn from(event: &KeyEvent) -> Self {
        Self {
            code: event.code,
            modifiers: event.modifiers,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Command {
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
