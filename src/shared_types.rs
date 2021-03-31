use std::convert::TryFrom;
use std::ops::Deref;

use serde::de::Deserializer;
use serde::ser::Serializer;
use serde::{Deserialize, Serialize};

use crate::Error;

pub static PUBLIC_OBJECT_LIBS_KEY: &str = "public.objectLibs";

/// A Plist dictionary.
pub type Plist = plist::Dictionary;

// Types used in fontinfo.plist.

pub type Integer = i32;
pub type NonNegativeInteger = u32;
pub type Float = f64;
pub type Bitlist = Vec<u8>;

/// IntegerOrFloat represents a number that can be an integer or float. It should
/// serialize to an integer if it effectively represents one.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct IntegerOrFloat(f64);

impl IntegerOrFloat {
    pub fn new(value: f64) -> Self {
        IntegerOrFloat(value)
    }

    pub fn get(&self) -> f64 {
        self.0
    }

    pub fn set(&mut self, value: f64) {
        self.0 = value
    }

    pub fn is_integer(&self) -> bool {
        (self.0 - self.round()).abs() < std::f64::EPSILON
    }
}

impl Deref for IntegerOrFloat {
    type Target = f64;

    fn deref(&self) -> &f64 {
        &self.0
    }
}

impl From<i32> for IntegerOrFloat {
    fn from(value: i32) -> Self {
        IntegerOrFloat(value as f64)
    }
}

impl From<f64> for IntegerOrFloat {
    fn from(value: f64) -> Self {
        IntegerOrFloat(value)
    }
}

impl Serialize for IntegerOrFloat {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        if self.is_integer() {
            serializer.serialize_i32(self.0 as i32)
        } else {
            serializer.serialize_f64(self.0)
        }
    }
}

impl<'de> Deserialize<'de> for IntegerOrFloat {
    fn deserialize<D>(deserializer: D) -> Result<IntegerOrFloat, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value: f64 = Deserialize::deserialize(deserializer)?;
        Ok(IntegerOrFloat(value))
    }
}

/// NonNegativeIntegerOrFloat represents a number that can be a NonNegative integer or float.
/// It should serialize to an integer if it effectively represents one.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NonNegativeIntegerOrFloat(f64);

impl NonNegativeIntegerOrFloat {
    pub fn new(value: f64) -> Option<Self> {
        if value.is_sign_positive() {
            Some(NonNegativeIntegerOrFloat(value))
        } else {
            None
        }
    }

    pub fn get(&self) -> f64 {
        self.0
    }

    pub fn try_set(&mut self, value: f64) -> Result<(), Error> {
        if value.is_sign_positive() {
            self.0 = value;
            Ok(())
        } else {
            Err(Error::ExpectedPositiveValue)
        }
    }

    pub fn is_integer(&self) -> bool {
        (self.0 - self.round()).abs() < std::f64::EPSILON
    }
}

impl Deref for NonNegativeIntegerOrFloat {
    type Target = f64;

    fn deref(&self) -> &f64 {
        &self.0
    }
}

impl TryFrom<i32> for NonNegativeIntegerOrFloat {
    type Error = Error;

    fn try_from(value: i32) -> Result<Self, Self::Error> {
        match NonNegativeIntegerOrFloat::new(value as f64) {
            Some(v) => Ok(v),
            _ => Err(Error::ExpectedPositiveValue),
        }
    }
}

impl TryFrom<f64> for NonNegativeIntegerOrFloat {
    type Error = Error;

    fn try_from(value: f64) -> Result<Self, Self::Error> {
        match NonNegativeIntegerOrFloat::new(value) {
            Some(v) => Ok(v),
            _ => Err(Error::ExpectedPositiveValue),
        }
    }
}

impl TryFrom<IntegerOrFloat> for NonNegativeIntegerOrFloat {
    type Error = Error;

    fn try_from(value: IntegerOrFloat) -> Result<Self, Self::Error> {
        match NonNegativeIntegerOrFloat::new(*value) {
            Some(v) => Ok(v),
            _ => Err(Error::ExpectedPositiveValue),
        }
    }
}

impl Serialize for NonNegativeIntegerOrFloat {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        if self.is_integer() {
            serializer.serialize_i32(self.0 as i32)
        } else {
            serializer.serialize_f64(self.0)
        }
    }
}

impl<'de> Deserialize<'de> for NonNegativeIntegerOrFloat {
    fn deserialize<D>(deserializer: D) -> Result<NonNegativeIntegerOrFloat, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value: f64 = Deserialize::deserialize(deserializer)?;
        match NonNegativeIntegerOrFloat::try_from(value) {
            Ok(v) => Ok(v),
            Err(_) => Err(serde::de::Error::custom("Value must be positive.")),
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_test::{assert_tokens, Token};

    use crate::color::Color;
    use crate::guideline::*;
    use crate::identifier::Identifier;

    use super::*;

    #[test]
    fn color_parsing() {
        let c1 = Color { red: 1.0, green: 0.0, blue: 0.0, alpha: 1.0 };
        assert_tokens(&c1, &[Token::Str("1,0,0,1")]);

        let c2 = Color { red: 0.0, green: 0.5, blue: 0.0, alpha: 0.5 };
        assert_tokens(&c2, &[Token::Str("0,0.5,0,0.5")]);
    }

    #[test]
    fn identifier_parsing() {
        let valid_chars = " !\"#$%&'()*+,-./0123456789:;<=>?@ABCDEFGHIJKLMNOPQRSTUVWXYZ[\\]^_`abcdefghijklmnopqrstuvwxyz{|}~";
        assert!(Identifier::new(valid_chars).is_ok());

        let i2 = Identifier::new("0aAä");
        assert!(i2.is_err());
        let i3 = Identifier::new("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");
        assert!(i3.is_err());
    }

    #[test]
    fn guideline_parsing() {
        let g1 = Guideline::new(
            Line::Angle { x: 10.0, y: 20.0, degrees: 360.0 },
            Some("hello".to_string()),
            Some(Color { red: 0.0, green: 0.5, blue: 0.0, alpha: 0.5 }),
            Some(Identifier::new("abcABC123").unwrap()),
            None,
        );
        assert_tokens(
            &g1,
            &[
                Token::Struct { name: "RawGuideline", len: 6 },
                Token::Str("x"),
                Token::Some,
                Token::F32(10.0),
                Token::Str("y"),
                Token::Some,
                Token::F32(20.0),
                Token::Str("angle"),
                Token::Some,
                Token::F32(360.0),
                Token::Str("name"),
                Token::Some,
                Token::Str("hello"),
                Token::Str("color"),
                Token::Some,
                Token::Str("0,0.5,0,0.5"),
                Token::Str("identifier"),
                Token::Some,
                Token::Str("abcABC123"),
                Token::StructEnd,
            ],
        );
    }

    #[test]
    fn test_integer_or_float_type() {
        let n1 = IntegerOrFloat::new(1.1);
        assert_tokens(&n1, &[Token::F64(1.1)]);
        let n1 = IntegerOrFloat::new(1.0);
        assert_tokens(&n1, &[Token::I32(1)]);
        let n1 = IntegerOrFloat::new(-1.1);
        assert_tokens(&n1, &[Token::F64(-1.1)]);
        let n1 = IntegerOrFloat::new(-1.0);
        assert_tokens(&n1, &[Token::I32(-1)]);

        let n1 = NonNegativeIntegerOrFloat::new(1.1).unwrap();
        assert_tokens(&n1, &[Token::F64(1.1)]);
        let n1 = NonNegativeIntegerOrFloat::new(1.0).unwrap();
        assert_tokens(&n1, &[Token::I32(1)]);
    }

    #[test]
    fn test_positive_int_or_float() {
        assert!(NonNegativeIntegerOrFloat::try_from(-1.0).is_err());

        let mut v = NonNegativeIntegerOrFloat::try_from(1.0).unwrap();
        assert!(v.try_set(-1.0).is_err());
        assert!(v.try_set(1.0).is_ok());
    }
}
