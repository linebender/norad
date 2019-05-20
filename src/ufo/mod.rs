//mod fontinfo;
mod glyph;
mod layer;

pub use layer::Layer;

pub use glyph::{
    Advance, AffineTransform, Anchor, Color, Component, Contour, ContourPoint, GlifVersion, Glyph,
    Guideline, Identifier, Image, Line, Outline, PointType,
};

pub struct Ufo {
    //meta_info: MetaInfo,
//font_info: Option<FontInfo>,
//layer_contents: Vec<LayerContents>,
}

//#[derive(Debug, Clone, Deserialize)]
//#[serde(rename_all = "camelCase")]
//pub struct MetaInfo {
//pub creator: String,
//pub format_version: u32,
//}
