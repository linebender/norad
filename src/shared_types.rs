use serde::de;
use serde::de::Deserializer;
use serde::ser;
use serde::ser::{SerializeStruct, Serializer};
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

impl Identifier {
    fn is_valid(&self) -> bool {
        self.0.len() <= 100 && self.0.bytes().all(|b| (0x20..=0x7E).contains(&b))
    }
}

impl Serialize for Identifier {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        if self.is_valid() {
            serializer.serialize_str(&self.0)
        } else {
            Err(ser::Error::custom("Identifier must be at most 100 characters long and contain only ASCII characters in the range 0x20 to 0x7E."))
        }
    }
}

impl<'de> Deserialize<'de> for Identifier {
    fn deserialize<D>(deserializer: D) -> Result<Identifier, D::Error>
    where
        D: Deserializer<'de>,
    {
        let string = String::deserialize(deserializer)?;
        let identifier = Identifier(string);

        if identifier.is_valid() {
            Ok(identifier)
        } else {
            Err(de::Error::custom("Identifier must be at most 100 characters long and contain only ASCII characters in the range 0x20 to 0x7E."))
        }
    }
}

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

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawGuideline {
    x: Option<f32>,
    y: Option<f32>,
    angle: Option<f32>,
    name: Option<String>,
    color: Option<Color>,
    identifier: Option<Identifier>,
}

impl Serialize for Guideline {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let (x, y, angle) = match self.line {
            Line::Vertical(x) => (Some(x), None, None),
            Line::Horizontal(y) => (None, Some(y), None),
            Line::Angle { x, y, degrees } => (Some(x), Some(y), Some(degrees)),
        };

        let mut guideline = serializer.serialize_struct("RawGuideline", 6)?;
        guideline.serialize_field("x", &x)?;
        guideline.serialize_field("y", &y)?;
        guideline.serialize_field("angle", &angle)?;
        guideline.serialize_field("name", &self.name)?;
        guideline.serialize_field("color", &self.color)?;
        guideline.serialize_field("identifier", &self.identifier)?;
        guideline.end()
    }
}

impl<'de> Deserialize<'de> for Guideline {
    fn deserialize<D>(deserializer: D) -> Result<Guideline, D::Error>
    where
        D: Deserializer<'de>,
    {
        let guideline = RawGuideline::deserialize(deserializer)?;

        let x = guideline.x;
        let y = guideline.y;
        let angle = guideline.angle;

        let line = match (x, y, angle) {
            // Valid data:
            (Some(x), None, None) => Line::Vertical(x),
            (None, Some(y), None) => Line::Horizontal(y),
            (Some(x), Some(y), Some(degrees)) => {
                if !(0.0..=360.0).contains(&degrees) {
                    return Err(de::Error::custom("angle must be between 0 and 360 degrees."));
                }
                Line::Angle { x, y, degrees }
            }
            // Invalid data:
            (None, None, _) => {
                return Err(de::Error::custom("x or y must be present in a guideline."))
            }
            (None, Some(_), Some(_)) | (Some(_), None, Some(_)) => {
                return Err(de::Error::custom(
                    "angle must only be specified when both x and y are specified.",
                ))
            }
            (Some(_), Some(_), None) => {
                return Err(de::Error::custom(
                    "angle must be specified when both x and y are specified.",
                ))
            }
        };

        Ok(Guideline {
            line,
            name: guideline.name,
            color: guideline.color,
            identifier: guideline.identifier,
        })
    }
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

impl<'de> Deserialize<'de> for Color {
    fn deserialize<D>(deserializer: D) -> Result<Color, D::Error>
    where
        D: Deserializer<'de>,
    {
        let string = String::deserialize(deserializer)?;
        let colors: Vec<f32> = string.split(',').map(|v| v.parse().unwrap()).collect();

        if colors.len() != 4 {
            return Err(serde::de::Error::custom(
                "Color definition must contain exactly 4 values seperated by commas.",
            ));
        }

        let red = colors[0];
        let green = colors[1];
        let blue = colors[2];
        let alpha = colors[3];
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_test::{assert_de_tokens_error, assert_ser_tokens_error, assert_tokens, Token};

    #[test]
    fn color_parsing() {
        let c1 = Color { red: 1.0, green: 0.0, blue: 0.0, alpha: 1.0 };
        assert_tokens(&c1, &[Token::Str("1,0,0,1")]);

        let c2 = Color { red: 0.0, green: 0.5, blue: 0.0, alpha: 0.5 };
        assert_tokens(&c2, &[Token::Str("0,0.5,0,0.5")]);
    }

    #[test]
    fn identifier_parsing() {
        let i1 = Identifier(
            " !\"#$%&'()*+,-./0123456789:;<=>?@ABCDEFGHIJKLMNOPQRSTUVWXYZ[\\]^_`abcdefghijklmnopqrstuvwxyz{|}~".to_string(),
        );
        assert_tokens(
            &i1,
            &[Token::Str(" !\"#$%&'()*+,-./0123456789:;<=>?@ABCDEFGHIJKLMNOPQRSTUVWXYZ[\\]^_`abcdefghijklmnopqrstuvwxyz{|}~")],
        );

        let i2 = Identifier("0aAä".to_string());
        let error = "Identifier must be at most 100 characters long and contain only ASCII characters in the range 0x20 to 0x7E.";
        assert_ser_tokens_error(&i2, &[], error);
        assert_de_tokens_error::<Identifier>(&[Token::Str("0aAä")], error);

        let i3 = Identifier("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_string());
        assert_ser_tokens_error(&i3, &[], error);
        assert_de_tokens_error::<Identifier>(
            &[Token::Str("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa")],
            error,
        );
    }

    #[test]
    fn guideline_parsing() {
        let g1 = Guideline {
            line: Line::Angle { x: 10.0, y: 20.0, degrees: 360.0 },
            name: Some("hello".to_string()),
            color: Some(Color { red: 0.0, green: 0.5, blue: 0.0, alpha: 0.5 }),
            identifier: Some(Identifier("abcABC123".to_string())),
        };
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
}
