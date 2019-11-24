use std::fmt;

use serde::de::{Deserializer, Visitor};
use serde::ser::Serializer;
use serde::{Deserialize, Serialize};

#[cfg(feature = "druid")]
use druid::Data;

/// Identifiers are optional attributes of several objects in the UFO.
/// These identifiers are required to be unique within certain contexts
/// as defined on a per object basis throughout this specification.
/// Identifiers are specified as a string between one and 100 characters long.
/// All characters must be in the printable ASCII range, 0x20 to 0x7E.
#[derive(Debug, Clone, PartialEq)]
pub struct Identifier(pub(crate) String);

/// A guideline associated with a glyph.
#[derive(Debug, Clone, PartialEq)]
pub struct Guideline {
    /// The line itself.
    pub line: Line,
    /// An arbitrary name for the guideline.
    pub name: Option<String>,
    /// The color of the line.
    pub color: Option<Color>,
    /// Unique identifier for the guideline. This attribute is not required
    /// and should only be added to guidelines as needed.
    pub identifier: Option<Identifier>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Line {
    /// A vertical line, passing through a given `x` coordinate.
    Vertical(f32),
    /// A horizontal line, passing through a given `y` coordinate.
    Horizontal(f32),
    /// An angled line passing through `(x, y)` at `degrees` degrees counteer-clockwise
    /// to the horizontal.
    Angle { x: f32, y: f32, degrees: f32 },
}

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "druid", derive(Data))]
pub struct Color {
    pub red: f32,
    pub green: f32,
    pub blue: f32,
    pub alpha: f32,
}

impl Serialize for Color {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let color_string = format!("{},{},{},{}", self.red, self.green, self.blue, self.alpha);
        serializer.serialize_str(&color_string)
    }
}

struct ColorVisitor;

impl<'de> Visitor<'de> for ColorVisitor {
    type Value = Color;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a string conforming to the UFO color definition.")
    }

    fn visit_str<E>(self, s: &str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        let colors: Vec<&str> = s.split(",").collect();

        if colors.len() != 4 {
            return Err(serde::de::Error::custom(
                "Color definition must contain exactly 4 values seperated by commas.",
            ));
        }

        let red: f32 = colors[0].parse().unwrap();
        let green: f32 = colors[1].parse().unwrap();
        let blue: f32 = colors[2].parse().unwrap();
        let alpha: f32 = colors[3].parse().unwrap();
        if (0.0..=1.0).contains(&red)
            && (0.0..=1.0).contains(&green)
            && (0.0..=1.0).contains(&blue)
            && (0.0..=1.0).contains(&alpha)
        {
            Ok(Color { red, green, blue, alpha })
        } else {
            Err(serde::de::Error::custom("Colors must be numbers between 0 and 1 inclusive."))
        }
    }
}

impl<'de> Deserialize<'de> for Color {
    fn deserialize<D>(deserializer: D) -> Result<Color, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_str(ColorVisitor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_test::{assert_tokens, Token};

    #[test]
    fn color_parsing() {
        let c1 = Color { red: 1.0, green: 0.0, blue: 0.0, alpha: 1.0 };
        assert_tokens(&c1, &[Token::Str("1,0,0,1")]);

        let c2 = Color { red: 0.0, green: 0.5, blue: 0.0, alpha: 0.5 };
        assert_tokens(&c2, &[Token::Str("0,0.5,0,0.5")]);
    }
}
