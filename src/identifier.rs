use std::hash::Hash;
use std::str::FromStr;
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
    /// Create a new [`Identifier`] from a [`String`], if it is valid.
    ///
    /// A valid identifier must have between 0 and 100 characters, and each
    /// character must be in the printable ASCII range, 0x20 to 0x7E.
    pub fn new(s: impl Into<Arc<str>>) -> Result<Self, ErrorKind> {
        let string = s.into();
        if is_valid_identifier(&string) {
            Ok(Identifier(string))
        } else {
            Err(ErrorKind::BadIdentifier)
        }
    }

    /// Create a new [`Identifier`] from a UUID v4 identifier.
    pub fn from_uuidv4() -> Self {
        Self::new(uuid::Uuid::new_v4().to_string()).unwrap()
    }

    /// Return the raw identifier, as a `&str`.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl PartialEq<String> for Identifier {
    fn eq(&self, other: &String) -> bool {
        *self.0 == *other
    }
}

impl FromStr for Identifier {
    type Err = ErrorKind;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Identifier::new(s)
    }
}

fn is_valid_identifier(s: &Arc<str>) -> bool {
    s.len() <= 100 && s.bytes().all(|b| (0x20..=0x7E).contains(&b))
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
        Identifier::new(string).map_err(|_| de::Error::custom("Identifier must be at most 100 characters long and contain only ASCII characters in the range 0x20 to 0x7E."))
    }
}
