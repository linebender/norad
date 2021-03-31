use std::str::FromStr;

#[cfg(feature = "druid")]
use druid::Data;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::error::ErrorKind;

/// A color.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "druid", derive(Data))]
pub struct Color {
    pub red: f32,
    pub green: f32,
    pub blue: f32,
    pub alpha: f32,
}

impl FromStr for Color {
    type Err = ErrorKind;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut iter = s.split(',').map(|v| match v.parse::<f32>() {
            Ok(val) if (0.0..=1.0).contains(&val) => Ok(val),
            _ => Err(ErrorKind::BadColor),
        });
        let red = iter.next().unwrap_or(Err(ErrorKind::BadColor))?;
        let green = iter.next().unwrap_or(Err(ErrorKind::BadColor))?;
        let blue = iter.next().unwrap_or(Err(ErrorKind::BadColor))?;
        let alpha = iter.next().unwrap_or(Err(ErrorKind::BadColor))?;
        if iter.next().is_some() {
            Err(ErrorKind::BadColor)
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
        Color::from_str(&string).map_err(|_| serde::de::Error::custom("Malformed color string."))
    }
}

#[cfg(feature = "druid")]
impl From<druid::piet::Color> for Color {
    fn from(src: druid::piet::Color) -> Color {
        let rgba = src.as_rgba_u32();
        let r = ((rgba >> 24) & 0xff) as f32 / 255.0;
        let g = ((rgba >> 16) & 0xff) as f32 / 255.0;
        let b = ((rgba >> 8) & 0xff) as f32 / 255.0;
        let a = (rgba & 0xff) as f32 / 255.0;
        assert!((0.0..=1.0).contains(&b), "b: {}, raw {}", b, (rgba & (0xff << 8)));

        Color {
            red: r.max(0.0).min(1.0),
            green: g.max(0.0).min(1.0),
            blue: b.max(0.0).min(1.0),
            alpha: a.max(0.0).min(1.0),
        }
    }
}

#[cfg(feature = "druid")]
impl From<Color> for druid::piet::Color {
    fn from(src: Color) -> druid::piet::Color {
        druid::piet::Color::rgba(
            src.red.into(),
            src.green.into(),
            src.blue.into(),
            src.alpha.into(),
        )
    }
}
