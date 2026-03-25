//! Helper types for working with kerning.

use std::collections::{BTreeMap, HashMap};

use serde::ser::{SerializeMap, Serializer};
use serde::Serialize;

use crate::groups::{FIRST_KERNING_GROUP_PREFIX, SECOND_KERNING_GROUP_PREFIX};
use crate::{Groups, Name};

/// A map of kerning pairs.
///
/// This is represented as a map of first half of a kerning pair (glyph name or group name)
/// to the second half of a pair (glyph name or group name), which maps to the kerning value
/// (high-level view: (first, second) => value).
///
/// We use a [`BTreeMap`] because we need sorting for serialization.
pub type Kerning = BTreeMap<Name, BTreeMap<Name, f64>>;

/// Maps glyph names to group names; the inverse of a `groups.plist` file.
#[derive(Debug)]
pub struct ReverseGroupsLookup {
    first: HashMap<Name, Name>,
    second: HashMap<Name, Name>,
}

impl ReverseGroupsLookup {
    /// Get the group (if any) for the glyph name when it's first in a kerning
    /// pair.
    #[inline]
    pub fn get_first(&self, glyph_name: &str) -> Option<Name> {
        self.first.get(glyph_name).cloned()
    }

    /// Get the group (if any) for the glyph name when it's second in a
    /// kerning pair.
    #[inline]
    pub fn get_second(&self, glyph_name: &str) -> Option<Name> {
        self.second.get(glyph_name).cloned()
    }
}

impl From<&Groups> for ReverseGroupsLookup {
    fn from(groups: &Groups) -> Self {
        groups.iter().fold(
            ReverseGroupsLookup { first: HashMap::new(), second: HashMap::new() },
            |mut rgl, (group_name, members)| {
                let inverted = members.iter().map(|member| (member.clone(), group_name.clone()));
                if group_name.starts_with(FIRST_KERNING_GROUP_PREFIX) {
                    rgl.first.extend(inverted);
                } else if group_name.starts_with(SECOND_KERNING_GROUP_PREFIX) {
                    rgl.second.extend(inverted);
                }
                rgl
            },
        )
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

        let lookup = font.get_reverse_groups_lookup();
        for (left, right, expected) in [
            ("O", "E", -100f64),
            ("O", "F", -200f64),
            ("D", "E", -100f64),
            ("D", "F", -300f64),
            ("Q", "E", -250f64),
            ("Q", "F", -250f64),
        ] {
            assert_eq!(
                font.kerning_lookup(&lookup, left, right),
                Some(expected),
                "kerning_lookup incorrect for /{left}/{right}"
            );
            assert_eq!(
                font.kerning_lookup_slow(left, right),
                Some(expected),
                "kerning_lookup_slow incorrect for /{left}/{right}"
            );
        }
    }
}
