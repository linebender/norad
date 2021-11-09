use std::collections::BTreeMap;

use serde::ser::{SerializeMap, Serializer};
use serde::Serialize;

/// A map of kerning pairs.
///
/// This is represented as a map of first half of a kerning pair (glyph name or group name)
/// to the second half of a pair (glyph name or group name), which maps to the kerning value
/// (high-level view: (first, second) => value).
///
/// We use a [`BTreeMap`] because we need sorting for serialization.
pub type Kerning = BTreeMap<String, BTreeMap<String, f64>>;

/// A helper for serializing kerning values.
///
/// `KerningSerializer` is a crutch to serialize kerning values as integers if they are
/// integers rather than floats. This spares us having to use a wrapper type like
/// `IntegerOrFloat` for kerning values.
pub(crate) struct KerningSerializer<'a> {
    pub(crate) kerning: &'a Kerning,
}

struct KerningInnerSerializer<'a> {
    inner_kerning: &'a BTreeMap<String, f64>,
}

impl<'a> Serialize for KerningSerializer<'a> {
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

impl<'a> Serialize for KerningInnerSerializer<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut map = serializer.serialize_map(Some(self.inner_kerning.len()))?;
        for (k, v) in self.inner_kerning {
            if (v - v.round()).abs() < std::f64::EPSILON {
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
