//! Utilties for working with [Universal Font Object][ufo] files.
//!
//! [ufo]: http://unifiedfontobject.org/versions/ufo3
//!
//! # Basic usage:
//!
//! ```no_run
//!
//! let path = "RoflsSansLight.ufo";
//! let font_obj = Ufo::load(path).expect("failed to load font");
//! let mut layer = font_obj.find_layer(|layer| layer.name == "glyphs").unwrap();
//! let glyph_a = layer.get_glyph("A").expect("missing glyph");
//! assert_eq!(glyph_a.name.as_str(), "A");
//! ```

mod parse;
mod ufo;
pub mod error;

pub use ufo::glyph;
pub use error::Error;
pub use ufo::{Glyph, Layer, Ufo};
