use std::str::FromStr;

use serde::de::Deserializer;
use serde::ser::Serializer;
use serde::{Deserialize, Serialize};

#[cfg(feature = "druid")]
use druid::Data;

pub static PUBLIC_OBJECT_LIBS_KEY: &str = "public.objectLibs";

/// A Plist dictionary.
pub type Plist = plist::Dictionary;

/// A color in RGBA (Red-Green-Blue-Alpha) format.
///
/// See <https://unifiedfontobject.org/versions/ufo3/conventions/#colors>.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "druid", derive(Data))]
pub struct Color {
    /// Red channel value. Must be in the range 0 to 1, inclusive.
    red: f64,
    /// Green channel value. Must be in the range 0 to 1, inclusive.
    green: f64,
    /// Blue channel value. Must be in the range 0 to 1, inclusive.
    blue: f64,
    /// Alpha (transparency) channel value. Must be in the range 0 to 1, inclusive.
    alpha: f64,
}

impl Color {
    /// Create a color with RGBA values in the range `0..=1.0`.
    ///
    /// Returns an error if any of the provided values are not in the allowed range.
    pub fn new(red: f64, green: f64, blue: f64, alpha: f64) -> Result<Self, ColorError> {
        if [red, green, blue, alpha].iter().all(|v| (0.0..=1.0).contains(v)) {
            Ok(Self { red, green, blue, alpha })
        } else {
            Err(ColorError::Value)
        }
    }

    /// Returns the RGBA channel values.
    pub fn channels(&self) -> (f64, f64, f64, f64) {
        (self.red, self.green, self.blue, self.alpha)
    }
}

/// An error representing an invalid [`Color`] string.
///
/// [`Color`]: crate::Color
#[derive(Debug, thiserror::Error)]
pub enum ColorError {
    /// The color string was malformed.
    #[error("failed to parse color string '{0}'")]
    Parse(String),
    /// A channel value was not between 0 and 1, inclusive.
    #[error("color channel values must be between 0 and 1, inclusive")]
    Value,
}

impl FromStr for Color {
    type Err = ColorError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut iter =
            s.split(',').map(|v| v.parse::<f64>().map_err(|_| ColorError::Parse(s.to_owned())));
        let red = iter.next().unwrap_or_else(|| Err(ColorError::Parse(s.to_owned())))?;
        let green = iter.next().unwrap_or_else(|| Err(ColorError::Parse(s.to_owned())))?;
        let blue = iter.next().unwrap_or_else(|| Err(ColorError::Parse(s.to_owned())))?;
        let alpha = iter.next().unwrap_or_else(|| Err(ColorError::Parse(s.to_owned())))?;
        if iter.next().is_some() {
            Err(ColorError::Parse(s.to_owned()))
        } else {
            Color::new(red, green, blue, alpha)
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
        Color::from_str(&string).map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use serde_test::{assert_de_tokens, assert_ser_tokens, assert_tokens, Token};

    use super::*;

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
}
