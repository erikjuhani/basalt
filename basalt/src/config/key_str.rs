use std::ops::Deref;

use crossterm::event::KeyCode;

use super::ConfigError;

#[derive(Clone, Debug, PartialEq, Default)]
pub struct KeyStr<'a>(&'a str);

impl Deref for KeyStr<'_> {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.0
    }
}

impl<'a> From<&'a str> for KeyStr<'a> {
    fn from(value: &'a str) -> Self {
        Self(value)
    }
}

impl<'a> TryFrom<KeyStr<'a>> for KeyCode {
    type Error = ConfigError;

    fn try_from(value: KeyStr<'a>) -> Result<Self, Self::Error> {
        match value.deref() {
            "backspace" => Ok(KeyCode::Backspace),
            "backtab" => Ok(KeyCode::BackTab),
            "delete" => Ok(KeyCode::Delete),
            "up" => Ok(KeyCode::Up),
            "down" => Ok(KeyCode::Down),
            "left" => Ok(KeyCode::Left),
            "right" => Ok(KeyCode::Right),
            "end" => Ok(KeyCode::End),
            "enter" => Ok(KeyCode::Enter),
            "home" => Ok(KeyCode::Home),
            "insert" => Ok(KeyCode::Insert),
            "page_down" => Ok(KeyCode::PageDown),
            "page_up" => Ok(KeyCode::PageUp),
            "tab" => Ok(KeyCode::Tab),
            "esc" => Ok(KeyCode::Esc),
            "space" => Ok(KeyCode::Char(' ')),
            key if key.len() == 1 => key
                .chars()
                .next()
                .map(KeyCode::Char)
                .ok_or(ConfigError::InvalidKeyCode(key.to_string())),
            key if key.strip_prefix('f').is_some() => key
                .parse::<u8>()
                .map(KeyCode::F)
                .map_err(|_| ConfigError::InvalidKeyCode(key.to_string())),
            key => Err(ConfigError::InvalidKeyCode(key.to_string())),
        }
    }
}
