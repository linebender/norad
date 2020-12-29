//! Utilties for working with [Universal Font Object][ufo] files.
//!
//! [ufo]: http://unifiedfontobject.org/versions/ufo3
//!
//! # Basic usage:
//!
//! ```no_run
//! use norad::Ufo;
//!
//! let path = "RoflsSansLight.ufo";
//! let mut font_obj = Ufo::load(path).expect("failed to load font");
//! let layer = font_obj.find_layer(|layer| layer.name == "glyphs").unwrap();
//! let glyph_a = layer.get_glyph("A").expect("missing glyph");
//! assert_eq!(glyph_a.name.as_ref(), "A");
//! ```

#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate serde_repr;

pub mod error;
pub mod fontinfo;
pub mod glyph;
mod layer;
mod names;
mod shared_types;
mod ufo;
mod upconversion;

pub use error::Error;
pub use fontinfo::FontInfo;
pub use glyph::{
    builder::GlyphBuilder, builder::OutlineBuilder, Advance, AffineTransform, Anchor, Component,
    Contour, ContourPoint, GlifVersion, Glyph, GlyphName, Image, Outline, PointType,
};
pub use layer::Layer;
pub use shared_types::{
    Color, Guideline, Identifier, IdentifierAccess, IntegerOrFloat, LibAccess, Line,
    NonNegativeIntegerOrFloat, Plist,
};
pub use ufo::{FormatVersion, LayerInfo, MetaInfo, Ufo};
