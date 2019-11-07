//! Reading and (maybe) writing Unified Font Object files.

#![deny(intra_doc_link_resolution_failure)]

use std::borrow::Borrow;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::fontinfo::FontInfo;
use crate::glyph::{Glyph, GlyphName};
use crate::layer::Layer;
use crate::Error;

static LAYER_CONTENTS_FILE: &str = "layercontents.plist";
static METAINFO_FILE: &str = "metainfo.plist";
static FONTINFO_FILE: &str = "fontinfo.plist";
static DEFAULT_LAYER_NAME: &str = "public.default";
static DEFAULT_GLYPHS_DIRNAME: &str = "glyphs";
static DEFAULT_METAINFO_CREATOR: &str = "org.linebender.norad";

/// A Unified Font Object.
#[derive(Default)]
pub struct Ufo {
    pub meta: MetaInfo,
    pub font_info: Option<FontInfo>,
    pub layers: Vec<LayerInfo>,
    __non_exhaustive: (),
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

/// A version of the [UFO spec].
///
/// [UFO spec]: http://unifiedfontobject.org
#[derive(Debug, Clone, Copy, Serialize_repr, Deserialize_repr, PartialEq)]
#[repr(u8)]
pub enum FormatVersion {
    V1 = 1,
    V2 = 2,
    V3 = 3,
}

/// The contents of the [`metainfo.plist`] file.
///
/// [`metainfo.plist`]: http://unifiedfontobject.org/versions/ufo3/metainfo.plist/
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MetaInfo {
    pub creator: String,
    pub format_version: FormatVersion,
}

impl Default for MetaInfo {
    fn default() -> Self {
        MetaInfo {
            creator: DEFAULT_METAINFO_CREATOR.to_string(),
            format_version: FormatVersion::V3,
        }
    }
}

impl Ufo {
    /// Crate a new `Ufo`.
    pub fn new(meta: MetaInfo) -> Self {
        let main_layer = LayerInfo {
            name: DEFAULT_LAYER_NAME.into(),
            path: PathBuf::from(DEFAULT_GLYPHS_DIRNAME),
            layer: Layer::default(),
        };

        Ufo { meta, font_info: None, layers: vec![main_layer], __non_exhaustive: () }
    }

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
            let layers = layers?;
            Ok(Ufo { layers, meta, font_info, __non_exhaustive: () })
        }
    }

    /// Returns a reference to the first layer matching a predicate.
    /// The predicate takes a `LayerInfo` struct, which includes the layer's
    /// name and path as well as the layer itself.
    pub fn find_layer<P>(&self, mut predicate: P) -> Option<&Layer>
    where
        P: FnMut(&LayerInfo) -> bool,
    {
        self.layers.iter().find(|l| predicate(l)).map(|l| &l.layer)
    }

    /// Returns a mutable reference to the first layer matching a predicate.
    /// The predicate takes a `LayerInfo` struct, which includes the layer's
    /// name and path as well as the layer itself.
    pub fn find_layer_mut<P>(&mut self, mut predicate: P) -> Option<&mut Layer>
    where
        P: FnMut(&LayerInfo) -> bool,
    {
        self.layers.iter_mut().find(|l| predicate(l)).map(|l| &mut l.layer)
    }

    /// Returns a reference to the default layer, if it exists.
    pub fn get_default_layer(&self) -> Option<&Layer> {
        self.layers
            .iter()
            .find(|l| l.path.file_name() == Some(OsStr::new(DEFAULT_GLYPHS_DIRNAME)))
            .map(|l| &l.layer)
    }

    /// Returns a mutable reference to the default layer, if it exists.
    pub fn get_default_layer_mut(&mut self) -> Option<&mut Layer> {
        self.layers
            .iter_mut()
            .find(|l| l.path.file_name() == Some(OsStr::new(DEFAULT_GLYPHS_DIRNAME)))
            .map(|l| &mut l.layer)
    }

    /// Returns an iterator over all layers in this font object.
    pub fn iter_layers(&self) -> impl Iterator<Item = &LayerInfo> {
        self.layers.iter()
    }

    /// Returns an iterator over all the glyphs in the default layer.
    pub fn iter_names(&self) -> impl Iterator<Item = GlyphName> + '_ {
        // this is overly complicated for opaque lifetime reasons, aka 'trust me'
        self.layers
            .iter()
            .filter(|l| l.path.file_name() == Some(OsStr::new(DEFAULT_GLYPHS_DIRNAME)))
            .flat_map(|l| l.layer.glyphs.keys().cloned())
    }

    //FIXME: support for multiple layers.
    /// Returns a reference to the glyph with the given name,
    /// IN THE DEFAULT LAYER, if it exists.
    pub fn get_glyph<K>(&self, key: &K) -> Option<&Arc<Glyph>>
    where
        GlyphName: Borrow<K>,
        K: Ord + ?Sized,
    {
        self.get_default_layer().and_then(|l| l.get_glyph(key))
    }

    /// Returns a mutable reference to the glyph with the given name,
    /// IN THE DEFAULT LAYER, if it exists.
    pub fn get_glyph_mut<K>(&mut self, key: &K) -> Option<&mut Arc<Glyph>>
    where
        GlyphName: Borrow<K>,
        K: Ord + ?Sized,
    {
        self.get_default_layer_mut().and_then(|l| l.get_glyph_mut(key))
    }

    /// Returns the total number of glyphs in the default layer.
    pub fn glyph_count(&self) -> usize {
        self.get_default_layer().map(|l| l.glyphs.len()).unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_is_v3() {
        let font = Ufo::new(MetaInfo::default());
        assert_eq!(font.meta.format_version, FormatVersion::V3);

        let font2 = Ufo::default();
        assert_eq!(font2.meta.format_version, FormatVersion::V3);
    }

    #[test]
    fn loading() {
        let path = "testdata/mutatorSans/MutatorSansLightWide.ufo";
        let font_obj = Ufo::load(path).unwrap();
        assert_eq!(font_obj.iter_layers().count(), 2);
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
