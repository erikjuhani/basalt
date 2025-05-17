use std::{error::Error, fmt};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use serde::{
    de::{self, Visitor},
    Deserialize, Deserializer,
};

use crate::app::{Action, ScrollAmount};

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub(crate) struct KeyBinding {
    pub key: Key,
    pub command: Command,
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct Key {
    pub modifiers: KeyModifiers,
    pub code: KeyCode,
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
        let (modifiers, code) = value.split_once('+').unwrap_or(("", value.as_str()));

        Ok(Key {
            modifiers: parse_modifiers(modifiers).map_err(de::Error::custom)?,
            code: parse_code(code).map_err(de::Error::custom)?,
        })
    }
}

fn parse_modifiers(modifiers: &str) -> Result<KeyModifiers, KeyParseError> {
    match modifiers {
        "" => Ok(KeyModifiers::NONE),
        "alt" => Ok(KeyModifiers::ALT),
        "ctrl" | "control" => Ok(KeyModifiers::CONTROL),
        "hyper" => Ok(KeyModifiers::HYPER),
        "meta" => Ok(KeyModifiers::META),
        "shift" => Ok(KeyModifiers::SHIFT),
        "super" => Ok(KeyModifiers::SUPER),
        _ => Err(KeyParseError::UnknownModifiers(modifiers.to_string())),
    }
}
fn parse_code(code: &str) -> Result<KeyCode, KeyParseError> {
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
    .ok_or(KeyParseError::UnknownCode(code.to_string()))
}

#[derive(Debug)]
enum KeyParseError {
    InvalidKeybinding(String),
    UnknownCode(String),
    UnknownModifiers(String),
}

impl fmt::Display for KeyParseError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            KeyParseError::InvalidKeybinding(error) => {
                write!(f, "Invalid keybinding: {error}")
            }
            KeyParseError::UnknownModifiers(modifier) => {
                write!(f, "Unknown modifiers: {}", modifier)
            }
            KeyParseError::UnknownCode(code) => write!(f, "Unknown code: {}", code),
        }
    }
}

impl Error for KeyParseError {}

impl de::Error for KeyParseError {
    fn custom<T>(msg: T) -> Self
    where
        T: fmt::Display,
    {
        KeyParseError::InvalidKeybinding(msg.to_string())
    }
}

impl From<&KeyEvent> for Key {
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
