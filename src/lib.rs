//! Utilties for working with [Unified Font Object][ufo] files.
//!
//! The types in this crate correspond to types described in the spec.
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

#![deny(broken_intra_doc_links, unsafe_code)]

#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate serde_repr;

mod data_request;
pub mod error;
mod font;
pub mod fontinfo;
mod glyph;
mod groups;
mod guideline;
mod identifier;
mod kerning;
mod layer;
mod names;
mod shared_types;
mod upconversion;
pub mod util;
mod write;

pub use data_request::DataRequest;
pub use error::Error;
pub use font::{Font, FormatVersion, MetaInfo};
pub use fontinfo::FontInfo;
pub use glyph::{
    AffineTransform, Anchor, Component, Contour, ContourPoint, GlifVersion, Glyph, GlyphName,
    Image, PointType,
};

pub use groups::Groups;
pub use guideline::{Guideline, Line};
pub use identifier::Identifier;
pub use kerning::Kerning;
pub use layer::{Layer, LayerSet};
pub use shared_types::{Color, IntegerOrFloat, NonNegativeIntegerOrFloat, Plist};
pub use write::{QuoteChar, WriteOptions};

#[allow(deprecated)]
pub use font::Ufo;
