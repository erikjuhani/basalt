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
pub struct Keystroke {
    pub code: KeyCode,
    pub modifiers: KeyModifiers,
}

impl Keystroke {
    pub const fn new(code: KeyCode, modifiers: KeyModifiers) -> Self {
        Self { code, modifiers }
    }
}

impl From<KeyEvent> for Keystroke {
    fn from(value: KeyEvent) -> Self {
        Self::from((value.code, value.modifiers))
    }
}

impl From<KeyCode> for Keystroke {
    fn from(code: KeyCode) -> Self {
        Keystroke::from((code, KeyModifiers::NONE))
    }
}

impl From<(KeyCode, KeyModifiers)> for Keystroke {
    fn from((code, mut modifiers): (KeyCode, KeyModifiers)) -> Self {
        if let KeyCode::Char(ch) = code {
            if ch.is_uppercase() {
                modifiers.insert(KeyModifiers::SHIFT);
            }
        }
        Self { code, modifiers }
    }
}

impl From<(char, KeyModifiers)> for Keystroke {
    fn from((c, modifiers): (char, KeyModifiers)) -> Self {
        Keystroke::from((KeyCode::Char(c), modifiers))
    }
}

impl From<&KeyEvent> for Keystroke {
    fn from(event: &KeyEvent) -> Self {
        Keystroke::new(event.code, event.modifiers)
    }
}

impl fmt::Display for Keystroke {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let code = self.code.to_string().replace(" ", "_");

        // Uppercase chars carry SHIFT implicitly — strip it from the display
        // so the string representation stays canonical (e.g. "G" not "shift-G")
        let modifiers = match self.code {
            KeyCode::Char(ch) if ch.is_uppercase() => self.modifiers - KeyModifiers::SHIFT,
            _ => self.modifiers,
        };

        if modifiers.is_empty() {
            write!(f, "{code}")
        } else {
            write!(f, "{}-{code}", modifiers.to_string().to_ascii_lowercase())
        }
    }
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum Key {
    Single(Keystroke),
    Chord(Vec<Keystroke>),
}

impl fmt::Display for Key {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Key::Single(key) => key.fmt(f),
            Key::Chord(keys) => keys.iter().try_for_each(|key| key.fmt(f)),
        }
    }
}

impl Key {
    pub const CTRL_C: Key = Key::new(KeyCode::Char('c'), KeyModifiers::CONTROL);

    pub const fn new(code: KeyCode, modifiers: KeyModifiers) -> Self {
        Key::Single(Keystroke::new(code, modifiers))
    }

    pub fn chord(iter: impl IntoIterator<Item = Keystroke>) -> Self {
        Key::Chord(iter.into_iter().collect())
    }
}

impl From<KeyEvent> for Key {
    fn from(value: KeyEvent) -> Self {
        Self::Single(Keystroke::from(value))
    }
}

impl From<KeyCode> for Key {
    fn from(value: KeyCode) -> Self {
        Self::Single(Keystroke::from(value))
    }
}

impl From<(KeyCode, KeyModifiers)> for Key {
    fn from(value: (KeyCode, KeyModifiers)) -> Self {
        Self::Single(Keystroke::from(value))
    }
}

impl From<char> for Key {
    fn from(value: char) -> Self {
        Self::from(KeyCode::Char(value))
    }
}

impl From<(char, KeyModifiers)> for Key {
    fn from(value: (char, KeyModifiers)) -> Self {
        Self::Single(Keystroke::from(value))
    }
}

impl From<Keystroke> for Key {
    fn from(value: Keystroke) -> Self {
        Self::Single(value)
    }
}

impl FromIterator<Keystroke> for Key {
    fn from_iter<T: IntoIterator<Item = Keystroke>>(iter: T) -> Self {
        Key::chord(iter)
    }
}

impl From<Vec<Keystroke>> for Key {
    fn from(value: Vec<Keystroke>) -> Self {
        Key::from_iter(value)
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
        formatter.write_str("a single key (\"a\"), named key (\"esc\"), modified key (\"ctrl+x\"), or key sequence (\"gg\")")
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        let mut parts = value.split('+');
        let code = parts
            .next_back()
            .ok_or(ConfigError::UnknownKeyCode(value.to_string()))
            .map_err(de::Error::custom)?;

        let mut modifiers = KeyModifiers::NONE;
        for part in parts {
            modifiers |= parse_modifiers(&part.to_lowercase()).map_err(de::Error::custom)?;
        }

        parse_key(code, modifiers).map_err(de::Error::custom)
    }
}

fn parse_key(code: &str, modifiers: KeyModifiers) -> Result<Key, ConfigError> {
    if code.is_empty() {
        return Ok(Key::from((KeyCode::Null, modifiers)));
    }

    let key_code = match code {
        "esc" => KeyCode::Esc,
        "space" => KeyCode::Char(' '),
        "backspace" => KeyCode::Backspace,
        "backtab" => KeyCode::BackTab,
        "delete" => KeyCode::Delete,
        "down" => KeyCode::Down,
        "end" => KeyCode::End,
        "enter" => KeyCode::Enter,
        "home" => KeyCode::Home,
        "insert" => KeyCode::Insert,
        "left" => KeyCode::Left,
        "page_down" => KeyCode::PageDown,
        "page_up" => KeyCode::PageUp,
        "right" => KeyCode::Right,
        "tab" => KeyCode::Tab,
        "up" => KeyCode::Up,
        // Single char — uppercase SHIFT is handled by Keystroke::from
        c if c.chars().count() == 1 => c
            .chars()
            .next()
            .map(KeyCode::Char)
            .ok_or_else(|| ConfigError::UnknownKeyCode(c.to_string()))?,
        // F-n keys
        c if c.starts_with('f') => c[1..]
            .parse::<u8>()
            .map(KeyCode::F)
            .map_err(|_| ConfigError::UnknownKeyCode(c.to_string()))?,
        // Multi-char sequence like "gG" or "ciw" — uppercase SHIFT via Keystroke::from
        c => {
            return Ok(Key::chord(
                c.chars().map(KeyCode::Char).map(Keystroke::from),
            ))
        }
    };

    Ok(Key::from((key_code, modifiers)))
}

fn parse_modifiers(modifiers: &str) -> Result<KeyModifiers, ConfigError> {
    if modifiers.is_empty() {
        return Ok(KeyModifiers::NONE);
    }

    match modifiers {
        "alt" => Ok(KeyModifiers::ALT),
        "ctrl" | "control" => Ok(KeyModifiers::CONTROL),
        "hyper" => Ok(KeyModifiers::HYPER),
        "meta" => Ok(KeyModifiers::META),
        "shift" => Ok(KeyModifiers::SHIFT),
        "super" => Ok(KeyModifiers::SUPER),
        _ => Err(ConfigError::UnknownKeyModifiers(modifiers.to_string())),
    }
}

impl de::Error for ConfigError {
    fn custom<T>(msg: T) -> Self
    where
        T: fmt::Display,
    {
        ConfigError::InvalidKeybinding(msg.to_string())
    }
}

#[cfg(test)]
mod tests {
    use ratatui::crossterm::event::{KeyCode, KeyModifiers};
    use serde::de::IntoDeserializer;

    use super::*;

    fn key_from_str(s: &str) -> Result<Key, ConfigError> {
        Key::deserialize(s.into_deserializer())
    }

    #[test]
    fn test_named_keys() {
        let cases = [
            ("esc", Key::from(KeyCode::Esc)),
            ("enter", Key::from(KeyCode::Enter)),
            ("space", Key::from(KeyCode::Char(' '))),
            ("backspace", Key::from(KeyCode::Backspace)),
            ("backtab", Key::from(KeyCode::BackTab)),
            ("delete", Key::from(KeyCode::Delete)),
            ("tab", Key::from(KeyCode::Tab)),
            ("up", Key::from(KeyCode::Up)),
            ("down", Key::from(KeyCode::Down)),
            ("left", Key::from(KeyCode::Left)),
            ("right", Key::from(KeyCode::Right)),
            ("home", Key::from(KeyCode::Home)),
            ("end", Key::from(KeyCode::End)),
            ("page_up", Key::from(KeyCode::PageUp)),
            ("page_down", Key::from(KeyCode::PageDown)),
            ("insert", Key::from(KeyCode::Insert)),
        ];

        cases.into_iter().for_each(|(input, expected)| {
            assert_eq!(key_from_str(input).unwrap(), expected, "input: {input:?}");
        });
    }

    #[test]
    fn test_single_char_keys() {
        let cases = [
            ("a", Key::from('a')),
            ("z", Key::from('z')),
            ("A", Key::from('A')),
            ("0", Key::from('0')),
            ("?", Key::from('?')),
            ("/", Key::from('/')),
            (":", Key::from(':')),
        ];

        cases.into_iter().for_each(|(input, expected)| {
            assert_eq!(key_from_str(input).unwrap(), expected, "input: {input:?}");
        });
    }

    #[test]
    fn test_function_keys() {
        let cases = [
            ("f1", Key::from(KeyCode::F(1))),
            ("f5", Key::from(KeyCode::F(5))),
            ("f12", Key::from(KeyCode::F(12))),
        ];

        cases.into_iter().for_each(|(input, expected)| {
            assert_eq!(key_from_str(input).unwrap(), expected, "input: {input:?}");
        });
    }

    #[test]
    fn test_modified_keys() {
        let cases = [
            ("ctrl+c", Key::from(('c', KeyModifiers::CONTROL))),
            ("control+c", Key::from(('c', KeyModifiers::CONTROL))),
            ("alt+x", Key::from(('x', KeyModifiers::ALT))),
            ("shift+a", Key::from(('a', KeyModifiers::SHIFT))),
            (
                "ctrl+shift+k",
                Key::from((
                    KeyCode::Char('k'),
                    KeyModifiers::CONTROL | KeyModifiers::SHIFT,
                )),
            ),
            (
                "ctrl+enter",
                Key::from((KeyCode::Enter, KeyModifiers::CONTROL)),
            ),
            ("alt+esc", Key::from((KeyCode::Esc, KeyModifiers::ALT))),
            ("ctrl+f5", Key::from((KeyCode::F(5), KeyModifiers::CONTROL))),
        ];

        cases.into_iter().for_each(|(input, expected)| {
            assert_eq!(key_from_str(input).unwrap(), expected, "input: {input:?}");
        });
    }

    #[test]
    fn test_key_sequences() {
        let cases: &[(&str, &[Keystroke])] = &[
            (
                "gg",
                &[
                    Keystroke::from(KeyCode::Char('g')),
                    Keystroke::from(KeyCode::Char('g')),
                ],
            ),
            (
                "gG",
                &[
                    Keystroke::from(KeyCode::Char('g')),
                    Keystroke::from(KeyCode::Char('G')),
                ],
            ),
            (
                "crn",
                &[
                    Keystroke::from(KeyCode::Char('c')),
                    Keystroke::from(KeyCode::Char('r')),
                    Keystroke::from(KeyCode::Char('n')),
                ],
            ),
        ];

        cases.iter().for_each(|(input, expected_keys)| {
            let key = key_from_str(input).unwrap();
            match key {
                Key::Chord(keys) => assert_eq!(keys, *expected_keys, "input: {input:?}"),
                Key::Single(_) => panic!("Expected sequence for {input:?}, got plain key"),
            }
        });
    }

    #[test]
    fn test_invalid_keys() {
        let cases = ["unknown_modifier+c", "badmod+x", "f999"];

        cases.into_iter().for_each(|input| {
            assert!(key_from_str(input).is_err(), "Expected error for {input:?}");
        });
    }

    #[test]
    fn test_keystroke_display() {
        let cases = [
            (Keystroke::new(KeyCode::Char('a'), KeyModifiers::NONE), "a"),
            (
                Keystroke::new(KeyCode::Char('c'), KeyModifiers::CONTROL),
                "control-c",
            ),
            // Uppercase char: SHIFT is implicit, not shown in display
            (Keystroke::from(KeyCode::Char('G')), "G"),
            // Uppercase char with additional modifier
            (
                Keystroke::from((KeyCode::Char('G'), KeyModifiers::CONTROL)),
                "control-G",
            ),
        ];

        cases.into_iter().for_each(|(key, expected)| {
            assert_eq!(key.to_string(), expected, "key: {key:?}");
        });
    }

    #[test]
    fn test_key_sequence_display() {
        let keys = [
            Keystroke::from(KeyCode::Char('g')),
            Keystroke::from(KeyCode::Char('G')),
        ];

        assert_eq!(Key::chord(keys).to_string(), "gG");
    }

    #[test]
    fn test_uppercase_implies_shift() {
        // Parsing "G" should give the same result as "shift+g" would — SHIFT in modifiers
        let upper = key_from_str("G").unwrap();
        assert_eq!(
            upper,
            Key::Single(Keystroke::new(KeyCode::Char('G'), KeyModifiers::SHIFT))
        );

        // Sequence "gG" — second key carries SHIFT
        let seq = key_from_str("gG").unwrap();
        assert_eq!(
            seq,
            Key::chord([
                Keystroke::new(KeyCode::Char('g'), KeyModifiers::NONE),
                Keystroke::new(KeyCode::Char('G'), KeyModifiers::SHIFT),
            ])
        );
    }
}
