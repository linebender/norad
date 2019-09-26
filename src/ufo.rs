//! Reading and (maybe) writing Unified Font Object files.

#![deny(intra_doc_link_resolution_failure)]

use std::ffi::OsStr;
use std::path::{Path, PathBuf};

use crate::layer::Layer;
use crate::Error;

static LAYER_CONTENTS_FILE: &str = "layercontents.plist";
static METAINFO_FILE: &str = "metainfo.plist";
static FONTINFO_FILE: &str = "fontinfo.plist";
static DEFAULT_LAYER_NAME: &str = "public.default";
static DEFAULT_GLYPHS_DIRNAME: &str = "glyphs";

/// A Unified Font Object.
#[allow(dead_code)] // meta isn't used, but we'll need it when writing
pub struct Ufo {
    meta: MetaInfo,
    pub font_info: Option<FontInfo>,
    layers: Vec<LayerInfo>,
}

/// A [font layer], along with its name and path.
///
/// This corresponds to a 'glyphs' directory on disk.
///
/// [font layer]: http://unifiedfontobject.org/versions/ufo3/glyphs/
pub struct LayerInfo {
    pub name: String,
    pub path: PathBuf,
    pub layer: Layer,
}

#[derive(Debug, Clone, Copy, Serialize_repr, Deserialize_repr, PartialEq)]
#[repr(u8)]
enum FormatVersion {
    V1 = 1,
    V2 = 2,
    V3 = 3,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct MetaInfo {
    creator: String,
    format_version: FormatVersion,
}

/// The contents of the [`fontinfo.plist`][] file.
///
/// [`fontinfo.plist`]: http://unifiedfontobject.org/versions/ufo1/fontinfo.plist/
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FontInfo {
    pub family_name: Option<String>,
    pub style_name: Option<String>,
    pub style_map_family_name: Option<String>,
    pub style_map_style_name: Option<String>,
    pub version_major: Option<u32>,
    pub version_minor: Option<u32>,
    pub year: Option<u32>,
    pub copyright: Option<String>,
    pub trademark: Option<String>,
    pub units_per_em: Option<f64>,
    pub descender: Option<f64>,
    pub x_height: Option<f64>,
    pub cap_height: Option<f64>,
    pub ascender: Option<f64>,
    pub italic_angle: Option<f64>,
    pub note: Option<String>,
}

impl Ufo {
    /// Attempt to load a font object from a file. `path` must point to
    /// a directory with the structure described in [v3 of the Unified Font Object][v3]
    /// spec.
    ///
    /// [v3]: http://unifiedfontobject.org/versions/ufo3/
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Ufo, Error> {
        let path = path.as_ref();
        return load_impl(path);

        // minimize monomorphization
        fn load_impl(path: &Path) -> Result<Ufo, Error> {
            let meta_path = path.join(METAINFO_FILE);
            let meta: MetaInfo = plist::from_file(meta_path)?;
            let font_path = path.join(FONTINFO_FILE);
            let font_info = if font_path.exists() {
                let font_info = plist::from_file(font_path)?;
                Some(font_info)
            } else {
                None
            };
            let mut contents = match meta.format_version {
                FormatVersion::V3 => {
                    let contents_path = path.join(LAYER_CONTENTS_FILE);
                    let contents: Vec<(String, PathBuf)> = plist::from_file(contents_path)?;
                    contents
                }
                _older => vec![(DEFAULT_LAYER_NAME.into(), DEFAULT_GLYPHS_DIRNAME.into())],
            };

            let layers: Result<Vec<LayerInfo>, Error> = contents
                .drain(..)
                .map(|(name, p)| {
                    let layer_path = path.join(&p);
                    let layer = Layer::load(layer_path)?;
                    Ok(LayerInfo { name, path: p, layer })
                })
                .collect();
            Ok(Ufo { layers: layers?, meta, font_info })
        }
    }

    /// Returns the first layer matching a predicate. The predicate takes a
    /// `LayerInfo` struct, which includes the layer's name and path as well
    /// as the layer itself.
    pub fn find_layer<P>(&mut self, mut predicate: P) -> Option<&mut Layer>
    where
        P: FnMut(&LayerInfo) -> bool,
    {
        self.layers.iter_mut().find(|l| predicate(l)).map(|l| &mut l.layer)
    }

    /// Returns the default layer, if it exists.
    pub fn get_default_layer(&mut self) -> Option<&mut Layer> {
        self.layers
            .iter_mut()
            .find(|l| l.path.file_name() == Some(OsStr::new(DEFAULT_GLYPHS_DIRNAME)))
            .map(|l| &mut l.layer)
    }

    /// Returns an iterator over all layers in this font object.
    pub fn iter(&self) -> impl Iterator<Item = &LayerInfo> {
        self.layers.iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loading() {
        let path = "testdata/mutatorSans/MutatorSansLightWide.ufo";
        let mut font_obj = Ufo::load(path).unwrap();
        assert_eq!(font_obj.iter().count(), 2);
        font_obj
            .find_layer(|l| l.path.to_str() == Some("glyphs.background"))
            .expect("missing layer");
    }

    #[test]
    fn metainfo() {
        let path = "testdata/mutatorSans/MutatorSansLightWide.ufo/metainfo.plist";
        let meta: MetaInfo = plist::from_file(path).expect("failed to load metainfo");
        assert_eq!(meta.creator, "org.robofab.ufoLib");
    }

    #[test]
    fn fontinfo() {
        let path = "testdata/mutatorSans/MutatorSansLightWide.ufo/fontinfo.plist";
        let font_info: FontInfo = plist::from_file(path).expect("failed to load fontinfo");
        assert_eq!(font_info.family_name, Some("MutatorMathTest".to_string()));
        assert_eq!(font_info.trademark, None);
    }
}
