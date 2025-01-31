use std::hash::Hash;
use std::sync::Arc;

use serde::{de, Deserialize, Deserializer, Serialize, Serializer};

use crate::error::ErrorKind;

/// A [UFO Object Identifier][identifier].
///
/// Identifiers are optional attributes of several objects in the UFO.
/// These identifiers are required to be unique within certain contexts
/// as defined on a per object basis throughout this specification.
/// Identifiers are specified as a string between one and 100 characters long.
/// All characters must be in the printable ASCII range, 0x20 to 0x7E.
///
/// [identifier]: https://unifiedfontobject.org/versions/ufo3/conventions/#identifiers
#[derive(Debug, Clone, Eq, Hash, PartialEq)]
pub struct Identifier(Arc<str>);

impl Identifier {
    /// Create a new [`Identifier`] from a string, if it is valid.
    ///
    /// A valid identifier must have between 0 and 100 characters, and each
    /// character must be in the printable ASCII range, 0x20 to 0x7E.
    pub fn new(string: &str) -> Result<Self, ErrorKind> {
        if is_valid_identifier(string) {
            Ok(Identifier(string.into()))
        } else {
            Err(ErrorKind::BadIdentifier)
        }
    }

    /// Creates a new `Identifier`, panicking if the given identifier is invalid.
    #[cfg(test)]
    pub(crate) fn new_raw(string: &str) -> Self {
        assert!(is_valid_identifier(string));
        Self(string.into())
    }

    /// Return the raw identifier, as a `&str`.
    pub fn as_str(&self) -> &str {
        self.as_ref()
    }
}

fn is_valid_identifier(s: &str) -> bool {
    s.len() <= 100 && s.bytes().all(|b| (0x20..=0x7E).contains(&b))
}

impl AsRef<str> for Identifier {
    fn as_ref(&self) -> &str {
        self.0.as_ref()
    }
}

impl std::ops::Deref for Identifier {
    type Target = str;
    fn deref(&self) -> &Self::Target {
        self.0.as_ref()
    }
}

// so that assert_eq! macros work
impl<'a> PartialEq<&'a str> for Identifier {
    fn eq(&self, other: &&'a str) -> bool {
        self.0.as_ref() == *other
    }
}

impl PartialEq<Identifier> for &str {
    fn eq(&self, other: &Identifier) -> bool {
        other == self
    }
}

impl std::fmt::Display for Identifier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        std::fmt::Display::fmt(&self.0, f)
    }
}

impl std::borrow::Borrow<str> for Identifier {
    fn borrow(&self) -> &str {
        self.0.as_ref()
    }
}

impl Serialize for Identifier {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        debug_assert!(
            is_valid_identifier(&self.0),
            "all identifiers are validated on construction"
        );
        serializer.serialize_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for Identifier {
    fn deserialize<D>(deserializer: D) -> Result<Identifier, D::Error>
    where
        D: Deserializer<'de>,
    {
        let string = String::deserialize(deserializer)?;
        Identifier::new(string.as_str()).map_err(de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identifier_parsing() {
        let valid_chars = " !\"#$%&'()*+,-./0123456789:;<=>?@ABCDEFGHIJKLMNOPQRSTUVWXYZ[\\]^_`abcdefghijklmnopqrstuvwxyz{|}~";
        assert!(Identifier::new(valid_chars).is_ok());

        let i2 = Identifier::new("0aAä");
        assert!(i2.is_err());
        let i3 = Identifier::new("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");
        assert!(i3.is_err());
    }
}
