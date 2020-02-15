//! Reading and (maybe) writing Unified Font Object files.

#![deny(intra_doc_link_resolution_failure)]

use std::borrow::Borrow;
use std::collections::BTreeMap;
use std::collections::HashSet;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::error::GroupsError;
use crate::fontinfo::FontInfo;
use crate::glyph::{Glyph, GlyphName};
use crate::layer::Layer;
use crate::Error;
use plist;

static LAYER_CONTENTS_FILE: &str = "layercontents.plist";
static METAINFO_FILE: &str = "metainfo.plist";
static FONTINFO_FILE: &str = "fontinfo.plist";
static LIB_FILE: &str = "lib.plist";
static GROUPS_FILE: &str = "groups.plist";
static KERNING_FILE: &str = "kerning.plist";
static FEATURES_FILE: &str = "features.fea";
static DEFAULT_LAYER_NAME: &str = "public.default";
static DEFAULT_GLYPHS_DIRNAME: &str = "glyphs";
static DEFAULT_METAINFO_CREATOR: &str = "org.linebender.norad";

/// A Unified Font Object.
#[derive(Default, Clone)]
pub struct Ufo {
    pub meta: MetaInfo,
    pub font_info: Option<FontInfo>,
    pub layers: Vec<LayerInfo>,
    pub lib: Option<plist::Dictionary>,
    // groups and kerning: BTreeMap because we need sorting for deserialization.
    pub groups: Option<BTreeMap<String, Vec<String>>>,
    pub kerning: Option<BTreeMap<String, BTreeMap<String, f32>>>,
    pub features: Option<String>,
    __non_exhaustive: (),
}

/// A [font layer], along with its name and path.
///
/// This corresponds to a 'glyphs' directory on disk.
///
/// [font layer]: http://unifiedfontobject.org/versions/ufo3/glyphs/
#[derive(Debug, Clone)]
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

        Ufo {
            meta,
            font_info: None,
            layers: vec![main_layer],
            lib: None,
            groups: None,
            kerning: None,
            features: None,
            __non_exhaustive: (),
        }
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
            let fontinfo_path = path.join(FONTINFO_FILE);
            let font_info = if fontinfo_path.exists() {
                let font_info: FontInfo = plist::from_file(fontinfo_path)?;
                font_info.validate()?;
                Some(font_info)
            } else {
                None
            };

            let lib_path = path.join(LIB_FILE);
            let lib = if lib_path.exists() {
                // Value::as_dictionary(_mut) will only borrow the data, but we want to own it.
                // https://github.com/ebarnard/rust-plist/pull/48
                match plist::Value::from_file(lib_path)? {
                    plist::Value::Dictionary(dict) => Some(dict),
                    _ => return Err(Error::ExpectedPlistDictionaryError),
                }
            } else {
                None
            };

            let groups_path = path.join(GROUPS_FILE);
            let groups = if groups_path.exists() {
                let groups: BTreeMap<String, Vec<String>> = plist::from_file(groups_path)?;
                validate_groups(&groups)?;

                Some(groups)
            } else {
                None
            };

            let kerning_path = path.join(KERNING_FILE);
            let kerning = if kerning_path.exists() {
                let kerning: BTreeMap<String, BTreeMap<String, f32>> =
                    plist::from_file(kerning_path)?;

                Some(kerning)
            } else {
                None
            };

            let features_path = path.join(FEATURES_FILE);
            let features = if features_path.exists() {
                let features = fs::read_to_string(features_path)?;

                Some(features)
            } else {
                None
            };

            let mut glyph_names = HashSet::new();
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
                    let layer = Layer::load_impl(&layer_path, &mut glyph_names)?;
                    Ok(LayerInfo { name, path: p, layer })
                })
                .collect();
            let layers = layers?;

            Ok(Ufo {
                layers,
                meta,
                font_info,
                lib,
                groups,
                kerning,
                features,
                __non_exhaustive: (),
            })
        }
    }

    /// Attempt to save this UFO to the given path, overriding any existing contents.
    ///
    /// This may fail; instead of saving directly to the target path, it is a good
    /// idea to save to a temporary location and then move that to the target path
    /// if the save is successful.
    pub fn save(&self, path: impl AsRef<Path>) -> Result<(), Error> {
        let path = path.as_ref();
        self.save_impl(path)
    }

    fn save_impl(&self, path: &Path) -> Result<(), Error> {
        if self.meta.creator.as_str() != DEFAULT_METAINFO_CREATOR {
            return Err(Error::NotCreatedHere);
        }

        if path.exists() {
            fs::remove_dir_all(path)?;
        }
        fs::create_dir(path)?;
        plist::to_file_xml(path.join(METAINFO_FILE), &self.meta)?;

        if let Some(font_info) = self.font_info.as_ref() {
            plist::to_file_xml(path.join(FONTINFO_FILE), &font_info)?;
        }

        if let Some(lib) = self.lib.as_ref() {
            // XXX: Can this be done without cloning?
            plist::Value::Dictionary(lib.clone()).to_file_xml(path.join(LIB_FILE))?;
        }

        if let Some(groups) = self.groups.as_ref() {
            validate_groups(&groups)?;
            plist::to_file_xml(path.join(GROUPS_FILE), groups)?;
        }

        if let Some(kerning) = self.kerning.as_ref() {
            plist::to_file_xml(path.join(KERNING_FILE), kerning)?;
        }

        if let Some(features) = self.features.as_ref() {
            fs::write(path.join(FEATURES_FILE), features)?;
        }

        let contents: Vec<(&String, &PathBuf)> =
            self.layers.iter().map(|l| (&l.name, &l.path)).collect();
        plist::to_file_xml(path.join(LAYER_CONTENTS_FILE), &contents)?;

        for layer in self.layers.iter() {
            let layer_path = path.join(&layer.path);
            layer.layer.save(layer_path)?;
        }

        Ok(())
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

/// Validate the contents of the groups.plist file according to the rules in the
/// [Unified Font Object v3 specification for groups.plist](http://unifiedfontobject.org/versions/ufo3/groups.plist/#specification).
fn validate_groups(groups_map: &BTreeMap<String, Vec<String>>) -> Result<(), Error> {
    let mut kern1_set = HashSet::new();
    let mut kern2_set = HashSet::new();
    for (group_name, group_glyph_names) in groups_map {
        if group_name.is_empty() {
            return Err(Error::Groups(GroupsError::InvalidName));
        }

        if group_name.starts_with("public.kern1.") {
            if group_name.len() == 13 {
                // Prefix but no actual name.
                return Err(Error::Groups(GroupsError::InvalidName));
            }
            for glyph_name in group_glyph_names {
                if !kern1_set.insert(glyph_name) {
                    return Err(Error::Groups(GroupsError::OverlappingKerningGroups {
                        glyph_name: glyph_name.to_string(),
                        group_name: group_name.to_string(),
                    }));
                }
            }
        } else if group_name.starts_with("public.kern2.") {
            if group_name.len() == 13 {
                // Prefix but no actual name.
                return Err(Error::Groups(GroupsError::InvalidName));
            }
            for glyph_name in group_glyph_names {
                if !kern2_set.insert(glyph_name) {
                    return Err(Error::Groups(GroupsError::OverlappingKerningGroups {
                        glyph_name: glyph_name.to_string(),
                        group_name: group_name.to_string(),
                    }));
                }
            }
        }
    }

    Ok(())
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

        assert_eq!(
            font_obj.lib.unwrap().get("com.typemytype.robofont.compileSettings.autohint"),
            Some(&plist::Value::Boolean(true))
        );

        assert_eq!(
            font_obj.groups.unwrap().get("public.kern1.@MMK_L_A"),
            Some(&vec!["A".to_string()])
        );

        assert_eq!(font_obj.kerning.unwrap().get("B").unwrap().get("H").unwrap(), &-40.0);

        assert_eq!(font_obj.features.unwrap(), "# this is the feature from lightWide\n");
    }

    #[test]
    fn metainfo() {
        let path = "testdata/mutatorSans/MutatorSansLightWide.ufo/metainfo.plist";
        let meta: MetaInfo = plist::from_file(path).expect("failed to load metainfo");
        assert_eq!(meta.creator, "org.robofab.ufoLib");
    }
}
