//! Glyph and layer names

use std::str::FromStr;
use std::sync::Arc;

use serde::{Deserialize, Deserializer, Serialize};

use crate::error::NamingError;

/// A name used to identify a [`Glyph`] or a [`Layer`].
///
/// Layers must be at least one character long, and cannot contain control
/// characters (`0x00..=0x1F`, `0x7F`, and `0x80..=0x9F`).
///
/// The details of how the name is stored may change, but it will always be
/// cheap to clone (at most a memcopy or a pointer clone) and it will always
/// deref to a `&str`.
///
/// [`Glyph`]: crate::Glyph
/// [`Layer`]: crate::Layer
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(transparent)]
pub struct Name(Arc<str>);

impl Name {
    /// Creates a new `Name` if the given value isn't empty and contains no control characters.
    pub fn new(name: &str) -> Result<Name, NamingError> {
        if is_valid(name) {
            Ok(Name(name.into()))
        } else {
            Err(NamingError::Invalid(name.into()))
        }
    }

    /// Creates a new `Name`, panicking if the given name is invalid.
    pub(crate) fn new_raw(name: &str) -> Name {
        assert!(is_valid(name));
        Name(name.into())
    }

    /// Returns a string slice containing the name.
    pub fn as_str(&self) -> &str {
        self.as_ref()
    }
}

fn is_valid(name: &str) -> bool {
    !(name.is_empty()
        // Important: check the chars(), not the bytes(), as UTF-8 encoding
        // bytes of course contain control characters.
        || name.chars().any(|b| {
            (0x0..=0x1f).contains(&(b as u32))
                || (0x80..=0x9f).contains(&(b as u32))
                || b as u32 == 0x7f
        }))
}

impl AsRef<str> for Name {
    fn as_ref(&self) -> &str {
        self.0.as_ref()
    }
}

impl std::ops::Deref for Name {
    type Target = str;
    fn deref(&self) -> &Self::Target {
        self.0.as_ref()
    }
}

// so that assert_eq! macros work
impl<'a> PartialEq<&'a str> for Name {
    fn eq(&self, other: &&'a str) -> bool {
        self.0.as_ref() == *other
    }
}

impl<'a> PartialEq<Name> for &'a str {
    fn eq(&self, other: &Name) -> bool {
        other == self
    }
}

impl std::fmt::Display for Name {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        std::fmt::Display::fmt(&self.0, f)
    }
}

impl std::borrow::Borrow<str> for Name {
    fn borrow(&self) -> &str {
        self.0.as_ref()
    }
}

impl FromStr for Name {
    type Err = NamingError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Name::new(s)
    }
}

impl TryFrom<String> for Name {
    type Error = NamingError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        if is_valid(&value) {
            Ok(Name(value.into()))
        } else {
            Err(NamingError::Invalid(value))
        }
    }
}

impl<'de> Deserialize<'de> for Name {
    fn deserialize<D>(deserializer: D) -> Result<Name, D::Error>
    where
        D: Deserializer<'de>,
    {
        // we go directly to Arc<str> and validate manually so we don't need
        // to allocate twice
        let s: Arc<str> = Deserialize::deserialize(deserializer)?;
        if is_valid(&s) {
            Ok(Name(s))
        } else {
            Err(serde::de::Error::custom(NamingError::Invalid(s.to_string())))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn assert_eq_str() {
        assert_eq!(Name::new_raw("hi"), "hi");
        assert_eq!("hi", Name::new_raw("hi"));
        assert_eq!("hi 💖", Name::new_raw("hi 💖"));
        assert_eq!(vec![Name::new_raw("a"), Name::new_raw("b")], vec!["a", "b"]);
        assert_eq!(vec!["a", "b"], vec![Name::new_raw("a"), Name::new_raw("b")]);
    }
}
