//! Reading and (maybe) writing Unified Font Object files.

use crate::layer::Layer;
use std::path::PathBuf;
use std::ffi::OsStr;

use crate::Error;

static LAYER_CONTENTS_FILE: &str = "layercontents.plist";
static METAINFO_FILE: &str = "metainfo.plist";
static DEFAULT_LAYER_NAME: &str = "public.default";
static DEFAULT_GLYPHS_DIRNAME: &str = "glyphs";

/// A Unified Font Object.
#[allow(dead_code)] // meta isn't used, but we'll need it when writing
pub struct Ufo {
    meta: MetaInfo,
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

impl Ufo {
    /// Attempt to load a font object from a file. `path` must point to
    /// a directory with the structure described in [v3 of the Unified Font Object][v3]
    /// spec.
    ///
    /// [v3]: http://unifiedfontobject.org/versions/ufo3/
    pub fn load<P: Into<PathBuf>>(path: P) -> Result<Ufo, Error> {
        let path = path.into();
        let meta_path = path.join(METAINFO_FILE);
        let meta: MetaInfo = plist::from_file(meta_path)?;
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
        Ok(Ufo { layers: layers?, meta })
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
        self.layers.iter_mut()
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
}
