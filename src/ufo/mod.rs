mod fontinfo;
mod layercontents;
mod glyph;

use serde::Deserialize;

use fontinfo::FontInfo;
use layercontents::LayerContents;

pub struct Ufo {
    meta_info: MetaInfo,
    font_info: Option<FontInfo>,
    layer_contents: Vec<LayerContents>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MetaInfo {
    pub creator: String,
    pub format_version: u32,
}
