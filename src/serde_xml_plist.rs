//! Helpers for deserializing plist values from untyped XML
//!
//! This is essentially glue between the `plist` crate and the `quick_xml` crate.
//! It allows plist values, dictionaries and arrays to be used inside types that
//! derive Deserialize/Serialize.

use std::{fmt::Display, marker::PhantomData, str::FromStr};

use base64::{engine::general_purpose::STANDARD as base64_standard, Engine};
use plist::{Dictionary, Value};
use serde::{
    de::{Error as DeError, Visitor},
    Deserialize, Deserializer,
};

/// Deserialize a plist Dictionary
///
/// This relies on the specific structure presented by the quick_xml crate and
/// is likely not suited to other formats.
pub fn deserialize_dict<'de, D>(deserializer: D) -> Result<Dictionary, D::Error>
where
    D: Deserializer<'de>,
{
    deserializer.deserialize_map(ValueVisitor::DictOnly).map(|x| x.into_dictionary().unwrap())
}

struct DictWrapper(Dictionary);
struct ValueWrapper(Value);
struct ArrayWrapper(Vec<Value>);
struct IntWrapper(plist::Integer);

/// The literal keyword 'key'.
struct KeyKeywordLiteral;

/// PLIST value keywords
///
/// We use types for keywords, with custom deserialize impls, to avoid needing
/// to transiently allocate strings each time we encounter them. :shrug:
enum ValueKeyword {
    Dict,
    Array,
    Integer,
    Real,
    String,
    Data,
    Date,
    True,
    False,
}

// the logic for deserializing a dict is a subset of the general deser logic,
// so we reuse this type for both cases.
enum ValueVisitor {
    AnyValue,
    DictOnly,
}

impl<'de> Visitor<'de> for ValueVisitor {
    type Value = Value;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            ValueVisitor::AnyValue => formatter.write_str("plist value"),
            ValueVisitor::DictOnly => formatter.write_str("plist dictionary"),
        }
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::MapAccess<'de>,
        A::Error: DeError,
    {
        match read_xml_value(&mut map, matches!(self, ValueVisitor::DictOnly)) {
            Ok(Some(val)) => Ok(val),
            Ok(None) => Err(A::Error::custom("expected value")),
            Err(e) => Err(e),
        }
    }
}

/// shared helper for deserializing a plist value from the serde map repr used by quick_xml
///
/// if `dict_only` is true, this will reject values that are not dicts.
fn read_xml_value<'de, A>(map: &mut A, dict_only: bool) -> Result<Option<Value>, A::Error>
where
    A: serde::de::MapAccess<'de>,
    A::Error: DeError,
{
    let value = match map.next_key::<ValueKeyword>()? {
        Some(ValueKeyword::Dict) => map.next_value::<DictWrapper>().map(|x| Value::Dictionary(x.0)),
        Some(other) if dict_only => {
            Err(A::Error::custom(format!("expected 'dict', found '{other}'")))
        }
        Some(ValueKeyword::String) => map.next_value::<String>().map(Value::String),
        Some(ValueKeyword::Array) => map.next_value::<ArrayWrapper>().map(|x| Value::Array(x.0)),
        Some(ValueKeyword::Data) => {
            //FIXME: remove this + base64 dep when/if we merge
            //<https://github.com/ebarnard/rust-plist/pull/122>
            let b64_str = map.next_value::<&str>()?;
            base64_standard
                .decode(b64_str)
                .map(Value::Data)
                .map_err(|e| A::Error::custom(format!("Invalid XML data: '{e}'")))
        }
        Some(ValueKeyword::Date) => {
            let date_str = map.next_value::<&str>()?;
            plist::Date::from_xml_format(date_str).map_err(A::Error::custom).map(Value::Date)
        }
        Some(ValueKeyword::Real) => map.next_value::<f64>().map(Value::Real),
        Some(ValueKeyword::Integer) => map.next_value::<IntWrapper>().map(|x| Value::Integer(x.0)),
        Some(kw @ ValueKeyword::True | kw @ ValueKeyword::False) => {
            // there's no value, but we need to call this to not confuse the parser
            let _ = map.next_value::<()>();
            Ok(Value::Boolean(matches!(kw, ValueKeyword::True)))
        }
        None => return Ok(None),
    };
    value.map(Some)
}

impl<'de> Deserialize<'de> for ValueWrapper {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_any(ValueVisitor::AnyValue).map(ValueWrapper)
    }
}

impl<'de> Deserialize<'de> for DictWrapper {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        // read a key in the form, "<key>SomeKey</key>"
        fn read_key<'de, A>(map: &mut A) -> Result<Option<String>, A::Error>
        where
            A: serde::de::MapAccess<'de>,
            A::Error: DeError,
        {
            match map.next_key::<KeyKeywordLiteral>()? {
                Some(_) => map.next_value(),
                None => Ok(None),
            }
        }

        struct DictVisitor;

        impl<'de> Visitor<'de> for DictVisitor {
            type Value = Dictionary;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("plist dictionary")
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: serde::de::MapAccess<'de>,
            {
                let mut dict = plist::Dictionary::new();
                // each logical key/value pair is two xml key/value pairs,
                // where the first is the key and the second is the value.
                while let Some(key) = read_key(&mut map)? {
                    // if we read a key it's an error for the value to be missing
                    let value = read_xml_value(&mut map, false)?
                        .ok_or_else(|| A::Error::custom("expected value"))?;
                    dict.insert(key, value);
                }
                Ok(dict)
            }
        }

        deserializer.deserialize_map(DictVisitor).map(DictWrapper)
    }
}

impl<'de> Deserialize<'de> for ArrayWrapper {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct ArrayVisitor;

        impl<'de> Visitor<'de> for ArrayVisitor {
            type Value = Vec<Value>;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("plist array")
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: serde::de::MapAccess<'de>,
            {
                let mut array = Vec::with_capacity(map.size_hint().unwrap_or_default());
                while let Some(value) = read_xml_value(&mut map, false)? {
                    array.push(value)
                }
                Ok(array)
            }
        }

        // NOTE: in quick_xml our arrays are represented as maps, where the key
        // is the tag and the content is the value.
        deserializer.deserialize_map(ArrayVisitor).map(ArrayWrapper)
    }
}

// a bit of over-engineering to match the semantics of Apple/the plist crate
//
// TL;DR: we deserialize hex values, but always as unsigned values.
impl<'de> Deserialize<'de> for IntWrapper {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct IntegerVisitor;

        impl<'de> Visitor<'de> for IntegerVisitor {
            type Value = plist::Integer;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("plist integer. NOTE: this currently expects the to only be used with the quick-xml crate, otherwise you'll need to impl more visitor methods")
            }

            // taken from the plist crate, under MIT license:
            // <https://docs.rs/plist/latest/src/plist/integer.rs.html#29>
            fn visit_str<E>(self, s: &str) -> Result<Self::Value, E>
            where
                E: DeError,
            {
                if s.starts_with("0x") {
                    // NetBSD dialect adds the `0x` numeric objects,
                    // which are always unsigned.
                    // See the `PROP_NUMBER(3)` man page
                    let s = s.trim_start_matches("0x");
                    u64::from_str_radix(s, 16).map(Into::into).map_err(E::custom)
                } else {
                    // Match Apple's implementation in CFPropertyList.h - always try to parse as an i64 first.
                    // TODO: Use IntErrorKind once stable and retry parsing on overflow only.
                    Ok(match s.parse::<i64>() {
                        Ok(v) => v.into(),
                        Err(_) => s.parse::<u64>().map_err(E::custom)?.into(),
                    })
                }
            }
            // END MIT license use
        }
        deserializer.deserialize_str(IntegerVisitor).map(IntWrapper)
    }
}

// visitor impl shared between key/value keywords
struct KeywordVisitor<T>(PhantomData<*const T>);

impl<'de, T> Visitor<'de> for KeywordVisitor<T>
where
    T: FromStr,
    T::Err: Display,
{
    type Value = T;

    fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", std::any::type_name::<T>())
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: DeError,
    {
        v.parse().map_err(E::custom)
    }
}

impl<'de> Deserialize<'de> for KeyKeywordLiteral {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_str(KeywordVisitor::<KeyKeywordLiteral>(PhantomData))
    }
}

impl<'de> Deserialize<'de> for ValueKeyword {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_str(KeywordVisitor::<ValueKeyword>(PhantomData))
    }
}

impl FromStr for KeyKeywordLiteral {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "key" => Ok(Self),
            other => Err(other.to_string()),
        }
    }
}

impl FromStr for ValueKeyword {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "dict" => Ok(Self::Dict),
            "array" => Ok(Self::Array),
            "integer" => Ok(Self::Integer),
            "real" => Ok(Self::Real),
            "string" => Ok(Self::String),
            "data" => Ok(Self::Data),
            "date" => Ok(Self::Date),
            "true" => Ok(Self::True),
            "false" => Ok(Self::False),
            other => Err(other.to_string()),
        }
    }
}

impl Display for ValueKeyword {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            ValueKeyword::Dict => "dict",
            ValueKeyword::Array => "array",
            ValueKeyword::Integer => "integer",
            ValueKeyword::Real => "real",
            ValueKeyword::String => "string",
            ValueKeyword::Data => "data",
            ValueKeyword::Date => "date",
            ValueKeyword::True => "true",
            ValueKeyword::False => "false",
        };
        f.write_str(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Deserialize)]
    struct TestMe {
        #[serde(deserialize_with = "deserialize_dict")]
        lib: Dictionary,
    }

    #[test]
    fn deserialize_everything() {
        let xml = r#"
<?xml version='1.0' encoding='UTF-8'?>
  <object>
  <lib>
    <dict>
      <key>hasLoadedLib</key>
      <string>Absolutely!</string>
      <key>anArray</key>
      <array>
        <dict>
          <key>class</key>
          <string>aristocracy</string>
          <key>heft</key>
          <real>42.42</real>
        </dict>
        <integer>6</integer>
      </array>
      <key>isWorking</key>
      <true/>
      <key>isBroken</key>
      <false/>
      <key>bestBefore</key>
      <date>2345-01-24T23:22:21Z</date>
      <key>payload</key>
      <data>
      dSBnb3QgMHduZWQ=
      </data>
    </dict>
  </lib>
  </object>
"#;

        let readme: TestMe = quick_xml::de::from_str(xml).unwrap();
        assert_eq!(readme.lib.get("hasLoadedLib").unwrap().as_string(), Some("Absolutely!"));
        let array = readme.lib.get("anArray").unwrap().as_array().unwrap();
        assert_eq!(
            array[0].as_dictionary().and_then(|d| d.get("class")),
            Some(&Value::String("aristocracy".into()))
        );
        assert_eq!(array[0].as_dictionary().and_then(|d| d.get("heft")), Some(&Value::Real(42.42)));
        assert_eq!(array[1].as_signed_integer(), Some(6));
        assert_eq!(readme.lib.get("isWorking"), Some(&Value::Boolean(true)));
        assert_eq!(readme.lib.get("isBroken"), Some(&Value::Boolean(false)));
        assert_eq!(
            readme.lib.get("bestBefore").and_then(Value::as_date).map(|d| d.to_xml_format()),
            Some("2345-01-24T23:22:21Z".into())
        );
        assert_eq!(
            readme.lib.get("payload").and_then(Value::as_data),
            Some("u got 0wned".as_bytes())
        );
    }

    #[test]
    fn empty_array_is_a_okay() {
        let xml = r#"
<?xml version='1.0' encoding='UTF-8'?>
  <object>
  <lib>
    <dict>
        <key>emptyDict</key>
        <dict></dict>
        <key>emptyArray</key>
        <array></array>
        <key>emptyString</key>
        <string></string>
    </dict>
  </lib>
  </object>
"#;

        let readme: TestMe = quick_xml::de::from_str(xml).unwrap();
        assert_eq!(
            readme.lib.get("emptyDict").and_then(Value::as_dictionary),
            Some(&Dictionary::new())
        );
        assert_eq!(readme.lib.get("emptyArray").and_then(Value::as_array), Some(&Vec::new()));
        assert_eq!(readme.lib.get("emptyString").and_then(Value::as_string), Some(""));
    }

    #[test]
    #[should_panic(expected = "Invalid XML data")]
    fn invalid_data() {
        let xml = r#"
<?xml version='1.0' encoding='UTF-8'?>
  <object>
  <lib>
    <dict>
        <key>badData</key>
        <data>💣</data>
    </dict>
  </lib>
  </object>
"#;

        let _readme: TestMe = quick_xml::de::from_str(xml).unwrap();
    }

    #[test]
    #[should_panic(expected = "date")]
    fn invalid_date() {
        let xml = r#"
<?xml version='1.0' encoding='UTF-8'?>
  <object>
  <lib>
    <dict>
        <key>badDate</key>
        <date>yesterday</date>
    </dict>
  </lib>
  </object>
"#;

        let _readme: TestMe = quick_xml::de::from_str(xml).unwrap();
    }
}
