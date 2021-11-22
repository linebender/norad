use serde::de::{Deserialize, Deserializer};
use serde::ser::{Serialize, SerializeStruct, Serializer};
use serde::{de, ser};

use crate::{Color, Identifier, Plist};

/// A guideline associated with a glyph.
#[derive(Debug, Clone, PartialEq)]
pub struct Guideline {
    /// The line itself.
    pub line: Line,
    /// An arbitrary name for the guideline.
    pub name: Option<String>,
    /// The color of the line.
    pub color: Option<Color>,
    /// Unique identifier for the guideline within the glyph. This attribute is only required
    /// when a lib is present and should otherwise only be added as needed.
    identifier: Option<Identifier>,
    /// The guideline's lib for arbitary data.
    lib: Option<Plist>,
}

/// An infinite line.
#[derive(Debug, Clone, PartialEq)]
pub enum Line {
    /// A vertical line, passing through a given `x` coordinate.
    Vertical(f64),
    /// A horizontal line, passing through a given `y` coordinate.
    Horizontal(f64),
    /// An angled line passing through `(x, y)` at `degrees` degrees counter-clockwise
    /// to the horizontal.
    // TODO: make a Degrees newtype that checks `0 <= degrees <= 360`.
    Angle {
        /// x coordinate.
        x: f64,
        /// y coordinate.
        y: f64,
        /// angle degrees.
        degrees: f64,
    },
}

impl Guideline {
    /// Returns a new [`Guideline`] struct.
    pub fn new(
        line: Line,
        name: Option<String>,
        color: Option<Color>,
        identifier: Option<Identifier>,
        lib: Option<Plist>,
    ) -> Self {
        let mut this = Self { line, name, color, identifier: None, lib: None };
        if let Some(id) = identifier {
            this.replace_identifier(id);
        }
        if let Some(lib) = lib {
            this.replace_lib(lib);
        }
        this
    }

    /// Returns a reference to the Guideline's lib.
    pub fn lib(&self) -> Option<&Plist> {
        self.lib.as_ref()
    }

    /// Returns a mutable reference to the Guideline's lib.
    pub fn lib_mut(&mut self) -> Option<&mut Plist> {
        self.lib.as_mut()
    }

    /// Replaces the actual lib by the lib given in parameter, returning the old
    /// lib if present. Sets a new UUID v4 identifier if none is set already.
    pub fn replace_lib(&mut self, lib: Plist) -> Option<Plist> {
        if self.identifier.is_none() {
            self.identifier.replace(Identifier::from_uuidv4());
        }
        self.lib.replace(lib)
    }

    /// Takes the lib out of the Guideline, leaving a None in its place.
    pub fn take_lib(&mut self) -> Option<Plist> {
        self.lib.take()
    }

    /// Returns a reference to the Guideline's identifier.
    pub fn identifier(&self) -> Option<&Identifier> {
        self.identifier.as_ref()
    }

    /// Replaces the actual identifier by the identifier given in parameter,
    /// returning the old identifier if present.
    pub fn replace_identifier(&mut self, id: Identifier) -> Option<Identifier> {
        self.identifier.replace(id)
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawGuideline {
    x: Option<f64>,
    y: Option<f64>,
    angle: Option<f64>,
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
            Line::Angle { x, y, degrees } => {
                if !(0.0..=360.0).contains(&degrees) {
                    return Err(ser::Error::custom("angle must be between 0 and 360 degrees."));
                }
                (Some(x), Some(y), Some(degrees))
            }
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

        Ok(Guideline::new(line, guideline.name, guideline.color, guideline.identifier, None))
    }
}
