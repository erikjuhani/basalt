use std::{fmt, slice};

use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use serde::{
    de::{self, Visitor},
    Deserialize, Deserializer,
};

use crate::{command::Command, config::ConfigError};

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub(crate) struct KeyBinding {
    pub key: KeySpec,
    pub command: Command,
}

impl KeyBinding {
    pub const fn new(key: KeySpec, command: Command) -> Self {
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
        let code = match code {
            KeyCode::Char(ch) if ch.is_uppercase() => {
                modifiers.insert(KeyModifiers::SHIFT);
                code
            }
            KeyCode::Char(ch)
                if modifiers.contains(KeyModifiers::SHIFT) && ch.is_ascii_lowercase() =>
            {
                // Normalize lowercase+SHIFT to uppercase
                KeyCode::Char(ch.to_ascii_uppercase())
            }
            _ => code,
        };
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
        Self::from((event.code, event.modifiers))
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
        let mut keystrokes: Vec<Keystroke> = iter.into_iter().collect();
        match keystrokes.len() {
            1 => Key::Single(keystrokes.remove(0)),
            _ => Key::Chord(keystrokes),
        }
    }

    fn keystrokes(&self) -> &[Keystroke] {
        match self {
            Key::Single(keystroke) => slice::from_ref(keystroke),
            Key::Chord(keystrokes) => keystrokes,
        }
    }
}

/// The prefix key that `<leader>` stands for in key bindings.
#[derive(Clone, Debug, Eq, Hash, PartialEq, Deserialize)]
#[serde(transparent)]
pub struct Leader(Key);

impl Default for Leader {
    fn default() -> Self {
        Self(Key::from(' '))
    }
}

impl From<Key> for Leader {
    fn from(key: Key) -> Self {
        Self(key)
    }
}

/// One element of a binding as written in config. `Leader` stands for however
/// many keystrokes the configured leader holds, so an element is not
/// necessarily a single key.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
enum Keys {
    Keystroke(Keystroke),
    Leader,
}

/// A binding as written in config, where `<leader>` still stands for the
/// configured [`Leader`]. Resolving it against one yields a concrete [`Key`].
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct KeySpec(Vec<Keys>);

impl KeySpec {
    /// Expands every `<leader>` into the keystrokes of the configured leader.
    pub fn resolve(&self, leader: &Leader) -> Key {
        self.0
            .iter()
            .flat_map(|keys| match keys {
                Keys::Keystroke(keystroke) => slice::from_ref(keystroke),
                Keys::Leader => leader.0.keystrokes(),
            })
            .cloned()
            .collect()
    }

    /// The concrete key, or `None` when the spec still contains `<leader>`.
    fn literal(&self) -> Option<Key> {
        self.0
            .iter()
            .map(|keys| match keys {
                Keys::Keystroke(keystroke) => Some(keystroke.clone()),
                Keys::Leader => None,
            })
            .collect()
    }
}

impl From<Key> for KeySpec {
    fn from(key: Key) -> Self {
        Self(
            key.keystrokes()
                .iter()
                .cloned()
                .map(Keys::Keystroke)
                .collect(),
        )
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

impl<'de> Deserialize<'de> for KeySpec {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_str(KeySpecVisitor)
    }
}

/// Only a literal key is accepted — `<leader>` is what a [`Leader`] defines,
/// so it cannot stand for itself.
impl<'de> Deserialize<'de> for Key {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        KeySpec::deserialize(deserializer)?
            .literal()
            .ok_or_else(|| de::Error::custom("`<leader>` is not allowed here"))
    }
}

struct KeySpecVisitor;

impl Visitor<'_> for KeySpecVisitor {
    type Value = KeySpec;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a single key (\"a\"), named key (\"esc\"), modified key (\"ctrl+x\"), or key sequence (\"gg\", \"<leader>f\")")
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        parse_keys(value).map(KeySpec).map_err(de::Error::custom)
    }
}

/// Tokenizes a binding string into its elements, treating `<...>` as a grouping
/// delimiter. The inside of a group is parsed exactly like a bare standalone
/// key, so `<space>f` isolates `space` as a named key instead of splitting
/// `spacef` into per-character keystrokes.
fn parse_keys(input: &str) -> Result<Vec<Keys>, ConfigError> {
    let mut keys = Vec::new();
    let mut rest = input;

    while !rest.is_empty() {
        match rest.strip_prefix('<') {
            // A `<...>` group denotes exactly one named/modified key.
            Some(after) => {
                let (name, tail) = after.split_once('>').ok_or_else(|| {
                    ConfigError::InvalidKeybinding(format!("unterminated `<` in {input:?}"))
                })?;
                match name {
                    "leader" => keys.push(Keys::Leader),
                    _ => match parse_segment(name)?.as_slice() {
                        [single] if !name.is_empty() => keys.push(Keys::Keystroke(single.clone())),
                        _ => {
                            return Err(ConfigError::InvalidKeybinding(format!(
                                "`<{name}>` is not a single key"
                            )))
                        }
                    },
                }
                rest = tail;
            }
            None => {
                let bare = rest.split('<').next().unwrap_or(rest);
                keys.extend(parse_segment(bare)?.into_iter().map(Keys::Keystroke));
                rest = &rest[bare.len()..];
            }
        }
    }

    Ok(keys)
}

/// Parses one segment — a bare run or the contents of a `<...>` group — into
/// its keystrokes, splitting `+` into modifiers and a trailing key code.
fn parse_segment(segment: &str) -> Result<Vec<Keystroke>, ConfigError> {
    let mut parts = segment.split('+');
    let code = parts
        .next_back()
        .ok_or_else(|| ConfigError::UnknownKeyCode(segment.to_string()))?;

    let modifiers = parts.try_fold(KeyModifiers::NONE, |modifiers, part| {
        parse_modifiers(&part.to_lowercase()).map(|parsed| modifiers | parsed)
    })?;

    Ok(parse_key(code, modifiers)?.keystrokes().to_vec())
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
        // `<` and `>` are reserved for group syntax — name them to bind literals
        "lt" => KeyCode::Char('<'),
        "gt" => KeyCode::Char('>'),
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

    fn spec_from_str(s: &str) -> Result<KeySpec, ConfigError> {
        KeySpec::deserialize(s.into_deserializer())
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
    fn test_named_key_sequences() {
        let space = Keystroke::from(KeyCode::Char(' '));
        let cases: &[(&str, &[Keystroke])] = &[
            (
                "<space>f",
                &[space.clone(), Keystroke::from(KeyCode::Char('f'))],
            ),
            ("<space><space>", &[space.clone(), space.clone()]),
            (
                "<enter>x",
                &[
                    Keystroke::from(KeyCode::Enter),
                    Keystroke::from(KeyCode::Char('x')),
                ],
            ),
            (
                "g<space>",
                &[Keystroke::from(KeyCode::Char('g')), space.clone()],
            ),
            (
                "<ctrl+space>f",
                &[
                    Keystroke::from((KeyCode::Char(' '), KeyModifiers::CONTROL)),
                    Keystroke::from(KeyCode::Char('f')),
                ],
            ),
        ];

        cases.iter().for_each(
            |(input, expected_keys)| match key_from_str(input).unwrap() {
                Key::Chord(keys) => assert_eq!(keys, *expected_keys, "input: {input:?}"),
                Key::Single(_) => panic!("Expected sequence for {input:?}, got plain key"),
            },
        );
    }

    #[test]
    fn test_named_group_single_key() {
        // A lone group is equivalent to the bare named key
        assert_eq!(
            key_from_str("<space>").unwrap(),
            key_from_str("space").unwrap()
        );
        assert_eq!(key_from_str("<esc>").unwrap(), key_from_str("esc").unwrap());
        // `<` and `>` literals remain bindable via lt/gt
        assert_eq!(key_from_str("<lt>").unwrap(), Key::from('<'));
        assert_eq!(key_from_str("<gt>").unwrap(), Key::from('>'));
    }

    #[test]
    fn test_invalid_keys() {
        let cases = [
            "unknown_modifier+c",
            "badmod+x",
            "f999",
            "<space",
            "<>",
            "<unknown_key>",
        ];

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
    fn test_leader_expands_to_the_configured_key() {
        let leader = Leader::from(Key::from(','));
        let cases = [
            ("<leader>", ","),
            ("<leader>f", ",f"),
            ("<leader><leader>", ",,"),
            ("g<leader>", "g,"),
            ("<leader><space>", ",<space>"),
        ];

        cases.into_iter().for_each(|(input, expected)| {
            assert_eq!(
                spec_from_str(input).unwrap().resolve(&leader),
                key_from_str(expected).unwrap(),
                "input: {input:?}"
            );
        });
    }

    #[test]
    fn test_leader_defaults_to_space() {
        assert_eq!(
            spec_from_str("<leader>f")
                .unwrap()
                .resolve(&Leader::default()),
            key_from_str("<space>f").unwrap()
        );
    }

    #[test]
    fn test_leader_can_be_a_sequence() {
        let leader = Leader::from(key_from_str("gs").unwrap());

        assert_eq!(
            spec_from_str("<leader>f").unwrap().resolve(&leader),
            key_from_str("gsf").unwrap()
        );
    }

    #[test]
    fn test_leader_is_not_a_literal_key() {
        // The leader definition itself cannot refer to `<leader>`
        assert!(key_from_str("<leader>").is_err());
        assert!(key_from_str("<leader>f").is_err());
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
