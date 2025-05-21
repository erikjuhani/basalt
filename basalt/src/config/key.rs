use std::{fmt, ops::Deref};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use serde::{
    de::{self, Visitor},
    Deserialize, Deserializer,
};

use super::{key_str::KeyStr, ConfigError};

#[derive(Clone, Debug, PartialEq)]
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

fn try_fold_into_key_str_and_modifiers<'a>(
    (key_str, modifiers): (KeyStr<'a>, KeyModifiers),
    value: &'a str,
) -> Result<(KeyStr<'a>, KeyModifiers), ConfigError> {
    match value {
        "ctrl" | "control" => Ok((key_str, modifiers.union(KeyModifiers::CONTROL))),
        "shift" => Ok((key_str, modifiers.union(KeyModifiers::SHIFT))),
        "alt" => Ok((key_str, modifiers.union(KeyModifiers::ALT))),
        "meta" => Ok((key_str, modifiers.union(KeyModifiers::META))),
        "super" => Ok((key_str, modifiers.union(KeyModifiers::SUPER))),
        "hyper" => Ok((key_str, modifiers.union(KeyModifiers::HYPER))),
        key if key_str.is_empty() => Ok((key.into(), modifiers)),
        key => Err(ConfigError::InvalidKeybinding(format!(
            "Multiple keys specified: {}, {}. There can only be one.",
            key_str.deref(),
            key
        ))),
    }
}

impl TryFrom<&str> for Key {
    type Error = ConfigError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let value = value.to_lowercase();

        let (key_str, modifiers) = value
            .split('+')
            .try_fold(
                (KeyStr::default(), KeyModifiers::empty()),
                try_fold_into_key_str_and_modifiers,
            )
            .and_then(|(key_str, modifiers)| {
                if !key_str.is_empty() {
                    Ok((key_str, modifiers))
                } else {
                    Err(ConfigError::InvalidKeybinding(format!(
                        "missing key, got: {}",
                        value
                    )))
                }
            })?;

        Ok(Key {
            code: key_str.try_into()?,
            modifiers,
        })
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
        formatter.write_str("a string that has a format of either 'key' or 'modifier+key'")
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        value.try_into().map_err(de::Error::custom)
    }
}

impl From<&KeyEvent> for Key {
    fn from(value: &KeyEvent) -> Self {
        Self {
            code: value.code,
            modifiers: value.modifiers,
        }
    }
}
