//! Helper types for working with kerning.
//!
//! To find the kerning value for a glyph/group pair, see
//! [`Font::kerning_resolver`](crate::Font::kerning_resolver) and then
//! [`KerningResolver::get`].

use serde::ser::{SerializeMap, Serializer};
use serde::Serialize;
use std::collections::{BTreeMap, HashMap};

use crate::Name;

/// A map of kerning pairs.
///
/// This is represented as a map of first half of a kerning pair (glyph name or group name)
/// to the second half of a pair (glyph name or group name), which maps to the kerning value
/// (high-level view: (first, second) => value).
///
/// We use a [`BTreeMap`] because we need sorting for serialization.
pub type Kerning = BTreeMap<Name, BTreeMap<Name, f64>>;

/// A utility struct to facilitate kerning lookups, including resolving group membership.
///
/// Created by calling [`Font::kerning_resolver`](crate::Font::kerning_resolver).
///
/// ```
/// # use norad::{Font, Name};
/// use maplit::btreemap;
///
/// let mut font = Font::new();
/// font.groups = btreemap! {
///     Name::new("public.kern1.A").unwrap() => vec![
///         Name::new("A").unwrap(),
///     ],
/// };
/// font.kerning = btreemap! {
///     Name::new("public.kern1.A").unwrap() => btreemap! {
///         Name::new("V").unwrap() => -15.0,
///     },
/// };
/// let resolver = font.kerning_resolver();
/// assert_eq!(
///     resolver.get("A", "V"),
///     Some(-15.0),
/// );
/// ```
#[derive(Debug)]
pub struct KerningResolver<'font> {
    pub(crate) kerning: &'font Kerning,
    pub(crate) first: HashMap<Name, Name>,
    pub(crate) second: HashMap<Name, Name>,
}

impl KerningResolver<'_> {
    /// Get the group (if any) for the glyph name when it's first in a kerning
    /// pair.
    #[inline]
    pub fn get_first_group(&self, glyph_name: &str) -> Option<Name> {
        self.first.get(glyph_name).cloned()
    }

    /// Get the group (if any) for the glyph name when it's second in a
    /// kerning pair.
    #[inline]
    pub fn get_second_group(&self, glyph_name: &str) -> Option<Name> {
        self.second.get(glyph_name).cloned()
    }

    /// Retrieve the kerning value (if any) between a pair of elements.
    ///
    /// The elements can be either individual glyphs (by name) or kerning groups
    /// (by name), or any combination of the two.
    //  ^ note: this works without any special consideration in the code
    //          because glyph names are forbidden from using the group prefix,
    //          thus meaning the group name lookup will always fail if a group
    //          was passed in
    pub fn get(&self, first: &str, second: &str) -> Option<f64> {
        let kerning_lookup = |first: &str, second: &str| {
            self.kerning.get(first).and_then(|first| first.get(second)).copied()
        };

        // glyph name glyph name
        if let Some(kern) = kerning_lookup(first, second) {
            return Some(kern);
        }

        // glyph name group name
        let second_group = self.get_second_group(second);
        if let Some(second_group) = &second_group {
            if let Some(kern) = kerning_lookup(first, second_group.as_str()) {
                return Some(kern);
            }
        }

        // group name glyph name
        let first_group = self.get_first_group(first);
        if let Some(first_group) = &first_group {
            if let Some(kern) = kerning_lookup(first_group.as_str(), second) {
                return Some(kern);
            }
        }

        // group name group name
        if let Some((first_group, second_group)) = first_group.zip(second_group) {
            if let Some(kern) = kerning_lookup(first_group.as_str(), second_group.as_str()) {
                return Some(kern);
            }
        }

        None
    }
}

/// A helper for serializing kerning values.
///
/// `KerningSerializer` is a crutch to serialize kerning values as integers if they are
/// integers rather than floats. This spares us having to use a wrapper type like
/// `IntegerOrFloat` for kerning values.
pub(crate) struct KerningSerializer<'a> {
    pub(crate) kerning: &'a Kerning,
}

struct KerningInnerSerializer<'a> {
    inner_kerning: &'a BTreeMap<Name, f64>,
}

impl Serialize for KerningSerializer<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut map = serializer.serialize_map(Some(self.kerning.len()))?;
        for (k, v) in self.kerning {
            let inner_v = KerningInnerSerializer { inner_kerning: v };
            map.serialize_entry(k, &inner_v)?;
        }
        map.end()
    }
}

impl Serialize for KerningInnerSerializer<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut map = serializer.serialize_map(Some(self.inner_kerning.len()))?;
        for (k, v) in self.inner_kerning {
            if (v - v.round()).abs() < f64::EPSILON {
                map.serialize_entry(k, &(*v as i32))?;
            } else {
                map.serialize_entry(k, v)?;
            }
        }
        map.end()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Font;
    use maplit::btreemap;
    use serde_test::{assert_ser_tokens, Token};

    #[test]
    fn serialize_kerning() {
        let kerning: Kerning = btreemap! {
            "A".into() => btreemap!{
                "A".into() => 1.0,
            },
            "B".into() => btreemap!{
                "A".into() => 5.4,
            },
        };

        let kerning_serializer = KerningSerializer { kerning: &kerning };

        assert_ser_tokens(
            &kerning_serializer,
            &[
                Token::Map { len: Some(2) },
                Token::Str("A"),
                Token::Map { len: Some(1) },
                Token::Str("A"),
                Token::I32(1),
                Token::MapEnd,
                Token::Str("B"),
                Token::Map { len: Some(1) },
                Token::Str("A"),
                Token::F64(5.4),
                Token::MapEnd,
                Token::MapEnd,
            ],
        );
    }

    #[test]
    fn test_kerning_resolution() {
        // Test data taken from https://unifiedfontobject.org/versions/ufo3/kerning.plist/#exceptions
        let font = Font {
            groups: btreemap! {
                Name::new_raw("public.kern1.O") => vec![
                    Name::new_raw("O"),
                    Name::new_raw("D"),
                    Name::new_raw("Q"),
                ],
                Name::new_raw("public.kern2.E") => vec![
                    Name::new_raw("E"),
                    Name::new_raw("F"),
                ],
            },
            kerning: btreemap! {
                Name::new_raw("public.kern1.O") => btreemap! {
                    Name::new_raw("public.kern2.E") => -100f64,
                    Name::new_raw("F") => -200f64,
                },
                Name::new_raw("Q") => btreemap! {
                    Name::new_raw("public.kern2.E") => -250f64,
                },
                Name::new_raw("D") => btreemap! {
                    Name::new_raw("F") => -300f64,
                },
            },
            ..Default::default()
        };

        let resolver = font.kerning_resolver();
        for (left, right, expected) in [
            ("O", "E", -100f64),
            ("O", "F", -200f64),
            ("D", "E", -100f64),
            ("D", "F", -300f64),
            ("Q", "E", -250f64),
            ("Q", "F", -250f64),
        ] {
            assert_eq!(
                resolver.get(left, right),
                Some(expected),
                "kerning_lookup incorrect for /{left}/{right}"
            );
        }
    }
}
