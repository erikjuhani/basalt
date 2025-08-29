use std::fmt;

use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use serde::{
    de::{self, Visitor},
    Deserialize, Deserializer,
};

use crate::{command::Command, config::ConfigError};

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub(crate) struct KeyBinding {
    pub key: Key,
    pub command: Command,
}

impl From<(Key, Command)> for KeyBinding {
    fn from((key, command): (Key, Command)) -> Self {
        Self::new(key, command)
    }
}

impl KeyBinding {
    pub const fn new(key: Key, command: Command) -> Self {
        Self { key, command }
    }
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
            write!(f, "{code}")
        } else {
            let modifiers = self.modifiers.to_string().to_ascii_lowercase();
            write!(f, "{modifiers}-{code}")
        }
    }
}

impl Key {
    pub const CTRL_C: Key = Key::new(KeyCode::Char('c'), KeyModifiers::CONTROL);

    pub const fn new(code: KeyCode, modifiers: KeyModifiers) -> Self {
        Self { code, modifiers }
    }
}

impl From<char> for Key {
    fn from(value: char) -> Self {
        Self::new(KeyCode::Char(value), KeyModifiers::NONE)
    }
}

impl From<(KeyCode, KeyModifiers)> for Key {
    fn from((code, modifiers): (KeyCode, KeyModifiers)) -> Self {
        Self::new(code, modifiers)
    }
}

impl From<(char, KeyModifiers)> for Key {
    fn from((char, modifiers): (char, KeyModifiers)) -> Self {
        Self::new(KeyCode::Char(char), modifiers)
    }
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
                "esc" => Some(KeyCode::Esc),
                "space" => Some(KeyCode::Char(' ')),
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
