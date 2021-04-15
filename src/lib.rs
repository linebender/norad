//! Utilties for working with [Unified Font Object][ufo] files.
//!
//! [ufo]: http://unifiedfontobject.org/versions/ufo3
//!
//! # Basic usage:
//!
//! ```no_run
//! use norad::Font;
//!
//! let path = "RoflsExtraDim.ufo";
//! let mut font_obj = Font::load(path).expect("failed to load font");
//! let layer = font_obj.default_layer();
//! let glyph_a = layer.get_glyph("A").expect("missing glyph");
//! assert_eq!(glyph_a.name.as_ref(), "A");
//! ```

#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate serde_repr;

pub mod error;
mod fontinfo;
mod glyph;
mod guideline;
mod identifier;
mod layer;
mod names;
mod shared_types;
mod ufo;
mod upconversion;
pub mod util;

pub use error::Error;
pub use fontinfo::FontInfo;
pub use glyph::{
    AffineTransform, Anchor, Component, Contour, ContourPoint, GlifVersion, Glyph, GlyphName,
    Image, PointType,
};
pub use guideline::{Guideline, Line};
pub use identifier::Identifier;
pub use layer::{Layer, LayerSet};
pub use shared_types::{Color, IntegerOrFloat, NonNegativeIntegerOrFloat, Plist};
pub use ufo::{DataRequest, Font, FormatVersion, MetaInfo};

#[allow(deprecated)]
pub use ufo::Ufo;
