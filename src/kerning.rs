use std::collections::{BTreeMap, HashMap};

use serde::ser::{SerializeMap, Serializer};
use serde::Serialize;

use crate::{Groups, Name};

pub const FIRST_KERNING_GROUP_PREFIX: &str = "public.kern1.";
pub const SECOND_KERNING_GROUP_PREFIX: &str = "public.kern2.";

/// A map of kerning pairs.
///
/// This is represented as a map of first half of a kerning pair (glyph name or group name)
/// to the second half of a pair (glyph name or group name), which maps to the kerning value
/// (high-level view: (first, second) => value).
///
/// We use a [`BTreeMap`] because we need sorting for serialization.
pub type Kerning = BTreeMap<Name, BTreeMap<Name, f64>>;

#[derive(Debug)]
pub struct ReverseGroupsLookup {
    first: HashMap<Name, Name>,
    second: HashMap<Name, Name>,
}

impl ReverseGroupsLookup {
    #[inline]
    pub fn get_first(&self, glyph_name: &str) -> Option<Name> {
        self.first.get(glyph_name).cloned()
    }

    #[inline]
    pub fn get_second(&self, glyph_name: &str) -> Option<Name> {
        self.second.get(glyph_name).cloned()
    }
}

impl From<&Groups> for ReverseGroupsLookup {
    fn from(groups: &Groups) -> Self {
        let first = groups
            .iter()
            .filter(|(group_name, _)| group_name.starts_with(FIRST_KERNING_GROUP_PREFIX))
            .flat_map(|(group_name, members)| {
                members.iter().map(|member| (member.clone(), group_name.clone()))
            })
            .collect();
        let second = groups
            .iter()
            .filter(|(group_name, _)| group_name.starts_with(SECOND_KERNING_GROUP_PREFIX))
            .flat_map(|(group_name, members)| {
                members.iter().map(|member| (member.clone(), group_name.clone()))
            })
            .collect();
        Self { first, second }
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
}
