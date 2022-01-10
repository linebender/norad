//! Utilties for working with [Unified Font Object][ufo] files.
//!
//! The types in this crate correspond to types described in the spec.
//!
//! [ufo]: http://unifiedfontobject.org/versions/ufo3
//!
//! # Basic Usage
//!
//! Instantiate a UFO font object with a [`Font`] struct like this:
//!
//! ```no_run
//! use norad::Font;
//!
//! let inpath = "RoflsExtraDim.ufo";
//! let mut font_obj = Font::load(inpath).expect("failed to load font");
//! # let layer = font_obj.default_layer();
//! # let glyph_a = layer.get_glyph("A").expect("missing glyph");
//! # assert_eq!(glyph_a.name.as_ref(), "A");
//! # let outpath = "RoflsSemiDim.ufo";
//! # font_obj.save(outpath);
//! ```
//!
//! The API may be used to access and modify data in the [`Font`]:
//!
//!```no_run
//! # use norad::Font;
//! # let inpath = "RoflsExtraDim.ufo";
//! # let mut font_obj = Font::load(inpath).expect("failed to load font");
//! let layer = font_obj.default_layer();
//! let glyph_a = layer.get_glyph("A").expect("missing glyph");
//! assert_eq!(glyph_a.name.as_ref(), "A");
//! # let outpath = "RoflsSemiDim.ufo";
//! # font_obj.save(outpath);
//! ```
//!
//! Serialize the [`Font`] to UFO files on disk with the [`Font::save`] method:
//!
//!```no_run
//! # use norad::Font;
//! # let inpath = "RoflsExtraDim.ufo";
//! # let mut font_obj = Font::load(inpath).expect("failed to load font");
//! # let layer = font_obj.default_layer();
//! # let glyph_a = layer.get_glyph("A").expect("missing glyph");
//! # assert_eq!(glyph_a.name.as_ref(), "A");
//! let outpath = "RoflsSemiDim.ufo";
//! font_obj.save(outpath);
//! ```
//!
//! Refer to the [`examples` directory of the source repository](https://github.com/linebender/norad/tree/master/examples)
//! for additional source code examples.
//!
//! # API Documentation
//!
//! Details on the full API for working with UFO fonts are available in these docs.
//!
//! # License
//!
//! norad is licensed under the [MIT](https://github.com/linebender/norad/blob/master/LICENSE-MIT)
//! and [Apache v2.0](https://github.com/linebender/norad/blob/master/LICENSE-APACHE) licenses.
//!
//! # Source
//!
//! Source files are available at <https://github.com/linebender/norad>.

#![warn(missing_docs)]
#![deny(rustdoc::broken_intra_doc_links, unsafe_code)]

#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate serde_repr;

mod data_request;
pub mod datastore;
pub mod error;
mod font;
pub mod fontinfo;
mod glyph;
mod groups;
mod guideline;
mod identifier;
mod kerning;
mod layer;
mod name;
mod names;
mod shared_types;
mod upconversion;
pub(crate) mod util;
mod write;

pub use data_request::DataRequest;
pub use font::{Font, FormatVersion, MetaInfo};
pub use fontinfo::FontInfo;
pub use glyph::{
    AffineTransform, Anchor, Component, Contour, ContourPoint, Glyph, Image, PointType,
};

pub use name::Name;

pub use groups::Groups;
pub use guideline::{Guideline, Line};
pub use identifier::Identifier;
pub use kerning::Kerning;
pub use layer::{Layer, LayerSet};
pub use shared_types::{Color, Plist};
pub use write::{QuoteChar, WriteOptions};
