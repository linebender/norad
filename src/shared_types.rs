use std::str::FromStr;

use serde::de::Deserializer;
use serde::ser::Serializer;
use serde::{Deserialize, Serialize};

#[cfg(feature = "druid")]
use druid::Data;

use crate::error::InvalidColorString;

pub static PUBLIC_OBJECT_LIBS_KEY: &str = "public.objectLibs";

/// A Plist dictionary.
pub type Plist = plist::Dictionary;

/// A color in RGBA (Red-Green-Blue-Alpha) format.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "druid", derive(Data))]
pub struct Color {
    /// Red channel value. Must be in the range 0 to 1, inclusive.
    pub red: f64,
    /// Green channel value. Must be in the range 0 to 1, inclusive.
    pub green: f64,
    /// Blue channel value. Must be in the range 0 to 1, inclusive.
    pub blue: f64,
    /// Alpha (transparency) channel value. Must be in the range 0 to 1, inclusive.
    pub alpha: f64,
}

impl FromStr for Color {
    type Err = InvalidColorString;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut iter = s.split(',').map(|v| match v.parse::<f64>() {
            Ok(val) if (0.0..=1.0).contains(&val) => Ok(val),
            _ => Err(InvalidColorString::new(s.to_owned())),
        });
        let red = iter.next().unwrap_or_else(|| Err(InvalidColorString::new(s.to_owned())))?;
        let green = iter.next().unwrap_or_else(|| Err(InvalidColorString::new(s.to_owned())))?;
        let blue = iter.next().unwrap_or_else(|| Err(InvalidColorString::new(s.to_owned())))?;
        let alpha = iter.next().unwrap_or_else(|| Err(InvalidColorString::new(s.to_owned())))?;
        if iter.next().is_some() {
            Err(InvalidColorString::new(s.to_owned()))
        } else {
            Ok(Color { red, green, blue, alpha })
        }
    }
}

impl Serialize for Color {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let color_string = self.to_rgba_string();
        serializer.serialize_str(&color_string)
    }
}

impl<'de> Deserialize<'de> for Color {
    fn deserialize<D>(deserializer: D) -> Result<Color, D::Error>
    where
        D: Deserializer<'de>,
    {
        let string = String::deserialize(deserializer)?;
        Color::from_str(&string).map_err(|_| serde::de::Error::custom("Malformed color string."))
    }
}

#[cfg(test)]
mod tests {
    use serde_test::{assert_de_tokens, assert_ser_tokens, assert_tokens, Token};

    use crate::{Color, Guideline, Identifier, Line};

    #[test]
    fn color_parsing() {
        let c1 = Color { red: 1.0, green: 0.0, blue: 0.0, alpha: 1.0 };
        assert_tokens(&c1, &[Token::Str("1,0,0,1")]);

        let c2 = Color { red: 0.0, green: 0.5, blue: 0.0, alpha: 0.5 };
        assert_tokens(&c2, &[Token::Str("0,0.5,0,0.5")]);

        let c3 = Color { red: 0.0, green: 0.0, blue: 0.0, alpha: 0.0 };
        assert_tokens(&c3, &[Token::Str("0,0,0,0")]);

        let c4 = Color { red: 0.123, green: 0.456, blue: 0.789, alpha: 0.159 };
        assert_tokens(&c4, &[Token::Str("0.123,0.456,0.789,0.159")]);

        #[allow(clippy::excessive_precision)]
        let c5 = Color { red: 0.123456789, green: 0.456789123, blue: 0.789123456, alpha: 0.1 };
        assert_ser_tokens(&c5, &[Token::Str("0.123,0.457,0.789,0.1")]);

        #[allow(clippy::excessive_precision)]
        let c6 = Color { red: 0.123456789, green: 0.456789123, blue: 0.789123456, alpha: 0.1 };
        assert_de_tokens(&c6, &[Token::Str("0.123456789,0.456789123,0.789123456,0.1")]);
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
                Token::F64(10.0),
                Token::Str("y"),
                Token::Some,
                Token::F64(20.0),
                Token::Str("angle"),
                Token::Some,
                Token::F64(360.0),
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
}
