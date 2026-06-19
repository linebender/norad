// Share module docs with README, and continue not running doctests there.
#![cfg_attr(not(doctest), doc = include_str!("../README.md"))]
#![warn(missing_docs)]
#![deny(rustdoc::broken_intra_doc_links, unsafe_code)]

mod data_request;
pub mod datastore;
pub mod designspace;
pub mod error;
mod font;
pub mod fontinfo;
mod glyph;
pub mod groups;
mod guideline;
mod identifier;
pub mod kerning;
mod layer;
mod name;
mod serde_xml_plist;
mod shared_types;
mod upconversion;
pub(crate) mod util;
mod write;

pub use data_request::DataRequest;
pub use font::{Font, FormatVersion, MetaInfo};
pub use fontinfo::FontInfo;
pub use glyph::{
    AffineTransform, Anchor, Codepoints, Component, Contour, ContourPoint, Glyph, Image, PointType,
};

pub use name::Name;

pub use groups::Groups;
pub use guideline::{Guideline, Line};
pub use identifier::Identifier;
pub use kerning::Kerning;
pub use layer::{Layer, LayerContents};
pub use shared_types::{Color, Plist};
pub use util::user_name_to_file_name;
pub use write::{QuoteChar, WriteOptions};
