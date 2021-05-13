//! Reading and (maybe) writing Unified Font Object files.

#![deny(broken_intra_doc_links)]

use std::borrow::Borrow;
use std::collections::{BTreeMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde::ser::{SerializeMap, Serializer};
use serde::Serialize;

use crate::error::GroupsValidationError;
use crate::fontinfo::FontInfo;
use crate::glyph::{Glyph, GlyphName};
use crate::guideline::Guideline;
use crate::layer::{Layer, LayerSet, LAYER_CONTENTS_FILE};
use crate::names::NameList;
use crate::shared_types::{Plist, PUBLIC_OBJECT_LIBS_KEY};
use crate::upconversion;
use crate::Error;

static METAINFO_FILE: &str = "metainfo.plist";
static FONTINFO_FILE: &str = "fontinfo.plist";
static LIB_FILE: &str = "lib.plist";
static GROUPS_FILE: &str = "groups.plist";
static KERNING_FILE: &str = "kerning.plist";
static FEATURES_FILE: &str = "features.fea";
static DEFAULT_METAINFO_CREATOR: &str = "org.linebender.norad";

/// Groups is a map of group name to a list of glyph names. It's a BTreeMap because we need sorting
/// for serialization.
pub type Groups = BTreeMap<String, Vec<GlyphName>>;
/// Kerning is a map of first half of a kerning pair (glyph name or group name) to the second half
/// of a pair (glyph name or group name), which maps to the kerning value (high-level view:
/// (first, second) => value). It's a BTreeMap because we need sorting for serialization.
pub type Kerning = BTreeMap<String, BTreeMap<String, f32>>;

/// A Unified Font Object.
#[derive(Clone, Debug, Default, PartialEq)]
#[non_exhaustive]
pub struct Font {
    pub meta: MetaInfo,
    pub font_info: Option<FontInfo>,
    pub layers: LayerSet,
    pub lib: Plist,
    pub groups: Option<Groups>,
    pub kerning: Option<Kerning>,
    pub features: Option<String>,
    pub data_request: DataRequest,
}

#[doc(hidden)]
#[deprecated(since = "0.4.0", note = "Renamed to Font")]
pub type Ufo = Font;

/// A type that describes which components of a UFO should be loaded.
///
/// By default, we load all components of the UFO file; however if you only
/// need some subset of these, you can pass this struct to [`Ufo::with_fields`]
/// in order to only load the fields specified in this object. This can help a
/// lot with performance with large UFO files if you don't need the glyph data.
///
/// [`Ufo::with_fields`]: struct.Ufo.html#method.with_fields
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct DataRequest {
    pub layers: bool,
    pub lib: bool,
    pub groups: bool,
    pub kerning: bool,
    pub features: bool,
}

impl DataRequest {
    fn from_bool(b: bool) -> Self {
        DataRequest { layers: b, lib: b, groups: b, kerning: b, features: b }
    }

    /// Returns a `DataRequest` requesting all UFO data.
    pub fn all() -> Self {
        DataRequest::from_bool(true)
    }

    /// Returns a `DataRequest` requesting no UFO data.
    pub fn none() -> Self {
        DataRequest::from_bool(false)
    }

    /// Request that returned UFO data include the glyph layers and points.
    pub fn layers(&mut self, b: bool) -> &mut Self {
        self.layers = b;
        self
    }

    /// Request that returned UFO data include <lib> sections.
    pub fn lib(&mut self, b: bool) -> &mut Self {
        self.lib = b;
        self
    }

    /// Request that returned UFO data include parsed `groups.plist`.
    pub fn groups(&mut self, b: bool) -> &mut Self {
        self.groups = b;
        self
    }

    /// Request that returned UFO data include parsed `kerning.plist`.
    pub fn kerning(&mut self, b: bool) -> &mut Self {
        self.kerning = b;
        self
    }

    /// Request that returned UFO data include OpenType Layout features in Adobe
    /// .fea format.
    pub fn features(&mut self, b: bool) -> &mut Self {
        self.features = b;
        self
    }
}

impl Default for DataRequest {
    fn default() -> Self {
        DataRequest::from_bool(true)
    }
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
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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

impl Font {
    /// Create a new `Ufo`.
    pub fn new() -> Self {
        Font::default()
    }

    /// Create a new `Ufo` only with certain fields
    pub fn with_fields(data_request: DataRequest) -> Self {
        let mut ufo = Self::new();
        ufo.data_request = data_request;
        ufo
    }

    /// Attempt to load a font object from a file. `path` must point to
    /// a directory with the structure described in [v3 of the Unified Font Object][v3]
    /// spec.
    ///
    /// NOTE: This will consume the `public.objectLibs` key in the global lib and in glyph
    /// libs and assign object libs found therein to global guidelines and glyph objects
    /// with the matching identifier, respectively.
    ///
    /// [v3]: http://unifiedfontobject.org/versions/ufo3/
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Font, Error> {
        Self::new().load_ufo(path)
    }

    pub fn load_ufo<P: AsRef<Path>>(&self, path: P) -> Result<Font, Error> {
        let path = path.as_ref();

        // minimize monomorphization
        let load_impl = |ufo: &Font, path: &Path| -> Result<Font, Error> {
            let meta_path = path.join(METAINFO_FILE);
            let mut meta: MetaInfo = plist::from_file(meta_path)?;

            let lib_path = path.join(LIB_FILE);
            let mut lib = if lib_path.exists() && self.data_request.lib {
                plist::Value::from_file(&lib_path)?.into_dictionary().ok_or_else(|| {
                    Error::ExpectedPlistDictionary(lib_path.to_string_lossy().into_owned())
                })?
            } else {
                Plist::new()
            };

            let fontinfo_path = path.join(FONTINFO_FILE);
            let mut font_info = if fontinfo_path.exists() {
                let font_info: FontInfo =
                    FontInfo::from_file(fontinfo_path, meta.format_version, &mut lib)?;
                Some(font_info)
            } else {
                None
            };

            let groups_path = path.join(GROUPS_FILE);
            let groups = if groups_path.exists() && self.data_request.groups {
                let groups: Groups = plist::from_file(groups_path)?;
                validate_groups(&groups).map_err(Error::InvalidGroups)?;
                Some(groups)
            } else {
                None
            };

            let kerning_path = path.join(KERNING_FILE);
            let kerning = if kerning_path.exists() && self.data_request.kerning {
                let kerning: Kerning = plist::from_file(kerning_path)?;
                Some(kerning)
            } else {
                None
            };

            let features_path = path.join(FEATURES_FILE);
            let mut features = if features_path.exists() && self.data_request.features {
                let features = fs::read_to_string(features_path)?;
                Some(features)
            } else {
                None
            };

            let glyph_names = NameList::default();
            let layers = if self.data_request.layers {
                if meta.format_version == FormatVersion::V3
                    && !path.join(LAYER_CONTENTS_FILE).exists()
                {
                    return Err(Error::MissingLayerContents);
                }
                LayerSet::load(path, &glyph_names)?
            } else {
                LayerSet::default()
            };

            // Upconvert UFO v1 or v2 kerning data if necessary. To upconvert, we need at least
            // a groups.plist file, while a kerning.plist is optional.
            let (groups, kerning) = match (meta.format_version, groups, kerning) {
                (FormatVersion::V3, g, k) => (g, k), // For v3, we do nothing.
                (_, None, k) => (None, k), // Without a groups.plist, there's nothing to upgrade.
                (_, Some(g), k) => {
                    let (groups, kerning) =
                        upconversion::upconvert_kerning(&g, &k.unwrap_or_default(), &glyph_names);
                    validate_groups(&groups).map_err(Error::GroupsUpconversionFailure)?;
                    (Some(groups), Some(kerning))
                }
            };

            // The v1 format stores some Postscript hinting related data in the lib,
            // which we only import into fontinfo if we're reading a v1 UFO.
            if meta.format_version == FormatVersion::V1 && lib_path.exists() {
                let mut fontinfo =
                    if let Some(fontinfo) = font_info { fontinfo } else { FontInfo::default() };

                let features_upgraded: Option<String> =
                    upconversion::upconvert_ufov1_robofab_data(&lib_path, &mut lib, &mut fontinfo)?;

                if features_upgraded.is_some() && !features_upgraded.as_ref().unwrap().is_empty() {
                    features = features_upgraded;
                }
                font_info = Some(fontinfo);
            }

            meta.format_version = FormatVersion::V3;

            Ok(Font {
                layers,
                meta,
                font_info,
                lib,
                groups,
                kerning,
                features,
                data_request: ufo.data_request,
            })
        };

        load_impl(&self, path)
    }

    /// Attempt to save this UFO to the given path, overriding any existing contents.
    ///
    /// This may fail; instead of saving directly to the target path, it is a good
    /// idea to save to a temporary location and then move that to the target path
    /// if the save is successful.
    ///
    /// This _will_ fail if either the global or any glyph lib contains the
    /// `public.objectLibs` key, as object lib management is done automatically.
    pub fn save(&self, path: impl AsRef<Path>) -> Result<(), Error> {
        let path = path.as_ref();
        self.save_impl(path)
    }

    fn save_impl(&self, path: &Path) -> Result<(), Error> {
        if self.meta.format_version != FormatVersion::V3 {
            return Err(Error::DowngradeUnsupported);
        }

        if self.lib.contains_key(PUBLIC_OBJECT_LIBS_KEY) {
            return Err(Error::PreexistingPublicObjectLibsKey);
        }

        if path.exists() {
            fs::remove_dir_all(path)?;
        }
        fs::create_dir(path)?;

        // we want to always set ourselves as the creator when serializing,
        // but we also don't have mutable access to self.
        if self.meta.creator == DEFAULT_METAINFO_CREATOR {
            plist::to_file_xml(path.join(METAINFO_FILE), &self.meta)?;
        } else {
            plist::to_file_xml(path.join(METAINFO_FILE), &MetaInfo::default())?;
        }

        if let Some(font_info) = self.font_info.as_ref() {
            plist::to_file_xml(path.join(FONTINFO_FILE), &font_info)?;
        }

        // Object libs are treated specially. The UFO v3 format won't allow us
        // to store them inline, so they have to be placed into the font's lib
        // under the public.objectLibs parent key. To avoid mutation behind the
        // client's back, object libs are written out but not stored in
        // font.lib in-memory. If there are object libs to serialize, clone the
        // existing lib and insert them there for serialization, otherwise write
        // out the original.
        let object_libs =
            self.font_info.as_ref().map(|f| f.dump_object_libs()).unwrap_or_else(Plist::new);
        if !object_libs.is_empty() {
            let mut new_lib = self.lib.clone();
            new_lib.insert(PUBLIC_OBJECT_LIBS_KEY.into(), plist::Value::Dictionary(object_libs));
            plist::Value::Dictionary(new_lib).to_file_xml(path.join(LIB_FILE))?;
        } else if !self.lib.is_empty() {
            plist::Value::Dictionary(self.lib.clone()).to_file_xml(path.join(LIB_FILE))?;
        }

        if let Some(groups) = self.groups.as_ref() {
            validate_groups(&groups).map_err(Error::InvalidGroups)?;
            plist::to_file_xml(path.join(GROUPS_FILE), groups)?;
        }

        if let Some(kerning) = self.kerning.as_ref() {
            let kerning_serializer = KerningSerializer { kerning: &kerning };
            plist::to_file_xml(path.join(KERNING_FILE), &kerning_serializer)?;
        }

        if let Some(features) = self.features.as_ref() {
            fs::write(path.join(FEATURES_FILE), features)?;
        }

        let contents: Vec<(&str, &PathBuf)> =
            self.layers.iter().map(|l| (l.name.as_ref(), &l.path)).collect();
        plist::to_file_xml(path.join(LAYER_CONTENTS_FILE), &contents)?;

        for layer in self.layers.iter() {
            let layer_path = path.join(&layer.path);
            layer.save(layer_path)?;
        }

        Ok(())
    }

    /// Returns a reference to the default layer.
    pub fn default_layer(&self) -> &Layer {
        self.layers.default_layer()
    }

    #[deprecated(since = "0.4.0", note = "use default_layer instead")]
    #[doc(hidden)]
    pub fn get_default_layer(&self) -> Option<&Layer> {
        Some(self.default_layer())
    }

    /// Returns a mutable reference to the default layer.
    pub fn default_layer_mut(&mut self) -> &mut Layer {
        self.layers.default_layer_mut()
    }

    #[deprecated(since = "0.4.0", note = "use default_layer instead")]
    #[doc(hidden)]
    pub fn get_default_layer_mut(&mut self) -> Option<&mut Layer> {
        Some(self.default_layer_mut())
    }

    /// Returns an iterator over all layers in this font object.
    pub fn iter_layers(&self) -> impl Iterator<Item = &Layer> {
        self.layers.iter()
    }

    /// Returns an iterator over all the glyphs in the default layer.
    pub fn iter_names(&self) -> impl Iterator<Item = GlyphName> + '_ {
        self.layers.default_layer().glyphs.keys().cloned()
    }

    /// Returns a reference to the glyph with the given name (in the default layer).
    pub fn get_glyph<K>(&self, key: &K) -> Option<&Arc<Glyph>>
    where
        GlyphName: Borrow<K>,
        K: Ord + ?Sized,
    {
        self.default_layer().get_glyph(key)
    }

    /// Returns a mutable reference to the glyph with the given name,
    /// IN THE DEFAULT LAYER, if it exists.
    pub fn get_glyph_mut<K>(&mut self, key: &K) -> Option<&mut Glyph>
    where
        GlyphName: Borrow<K>,
        K: Ord + ?Sized,
    {
        self.default_layer_mut().get_glyph_mut(key)
    }

    /// Returns the total number of glyphs in the default layer.
    pub fn glyph_count(&self) -> usize {
        self.default_layer().len()
    }

    /// Return the font's global guidelines, stored in [`FontInfo`].
    pub fn guidelines(&self) -> &[Guideline] {
        self.font_info.as_ref().and_then(|info| info.guidelines.as_deref()).unwrap_or(&[])
    }

    /// Returns a mutable reference to the font's global guidelines.
    ///
    /// These will be created if they do not already exist.
    pub fn guidelines_mut(&mut self) -> &mut Vec<Guideline> {
        self.font_info
            .get_or_insert_with(Default::default)
            .guidelines
            .get_or_insert_with(Default::default)
    }
}

/// Validate the contents of the groups.plist file according to the rules in the
/// [Unified Font Object v3 specification for groups.plist](http://unifiedfontobject.org/versions/ufo3/groups.plist/#specification).
fn validate_groups(groups_map: &Groups) -> Result<(), GroupsValidationError> {
    let mut kern1_set = HashSet::new();
    let mut kern2_set = HashSet::new();
    for (group_name, group_glyph_names) in groups_map {
        if group_name.is_empty() {
            return Err(GroupsValidationError::InvalidName);
        }

        if group_name.starts_with("public.kern1.") {
            if group_name.len() == 13 {
                // Prefix but no actual name.
                return Err(GroupsValidationError::InvalidName);
            }
            for glyph_name in group_glyph_names {
                if !kern1_set.insert(glyph_name) {
                    return Err(GroupsValidationError::OverlappingKerningGroups {
                        glyph_name: glyph_name.to_string(),
                        group_name: group_name.to_string(),
                    });
                }
            }
        } else if group_name.starts_with("public.kern2.") {
            if group_name.len() == 13 {
                // Prefix but no actual name.
                return Err(GroupsValidationError::InvalidName);
            }
            for glyph_name in group_glyph_names {
                if !kern2_set.insert(glyph_name) {
                    return Err(GroupsValidationError::OverlappingKerningGroups {
                        glyph_name: glyph_name.to_string(),
                        group_name: group_name.to_string(),
                    });
                }
            }
        }
    }

    Ok(())
}

/// KerningSerializer is a crutch to serialize kerning values as integers if they are
/// integers rather than floats. This spares us having to use a wrapper type like
/// IntegerOrFloat for kerning values.
struct KerningSerializer<'a> {
    kerning: &'a Kerning,
}

struct KerningInnerSerializer<'a> {
    inner_kerning: &'a BTreeMap<String, f32>,
}

impl<'a> Serialize for KerningSerializer<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut map = serializer.serialize_map(Some(self.kerning.len()))?;
        for (k, v) in self.kerning {
            let inner_v = KerningInnerSerializer { inner_kerning: v };
            map.serialize_entry(k, &inner_v)?;
        }
        map.end()
    }
}

impl<'a> Serialize for KerningInnerSerializer<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut map = serializer.serialize_map(Some(self.inner_kerning.len()))?;
        for (k, v) in self.inner_kerning {
            if (v - v.round()).abs() < std::f32::EPSILON {
                map.serialize_entry(k, &(*v as i32))?;
            } else {
                map.serialize_entry(k, v)?;
            }
        }
        map.end()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared_types::IntegerOrFloat;
    use maplit::btreemap;
    use serde_test::{assert_ser_tokens, Token};

    #[test]
    fn new_is_v3() {
        let font = Font::new();
        assert_eq!(font.meta.format_version, FormatVersion::V3);
    }

    #[test]
    fn downgrade_unsupported() {
        let dir = tempdir::TempDir::new("Test.ufo").unwrap();

        let mut font = Font::new();
        font.meta.format_version = FormatVersion::V1;
        assert_eq!(font.save(&dir).is_err(), true);
        font.meta.format_version = FormatVersion::V2;
        assert_eq!(font.save(&dir).is_err(), true);
        font.meta.format_version = FormatVersion::V3;
        assert_eq!(font.save(&dir).is_ok(), true);
    }

    #[test]
    fn loading() {
        let path = "testdata/mutatorSans/MutatorSansLightWide.ufo";
        let font_obj = Font::load(path).unwrap();
        assert_eq!(font_obj.iter_layers().count(), 2);
        font_obj.layers.get("background").expect("missing layer");

        assert_eq!(
            font_obj.lib.get("com.typemytype.robofont.compileSettings.autohint"),
            Some(&plist::Value::Boolean(true))
        );
        assert_eq!(font_obj.groups.unwrap().get("public.kern1.@MMK_L_A"), Some(&vec!["A".into()]));
        assert_eq!(font_obj.kerning.unwrap().get("B").unwrap().get("H").unwrap(), &-40.0);
        assert_eq!(font_obj.features.unwrap(), "# this is the feature from lightWide\n");
    }

    #[test]
    fn data_request() {
        let path = "testdata/mutatorSans/MutatorSansLightWide.ufo";
        let font_obj = Font::with_fields(DataRequest::none()).load_ufo(path).unwrap();
        assert_eq!(font_obj.iter_layers().count(), 1);
        assert!(font_obj.layers.default_layer().is_empty());
        assert_eq!(font_obj.lib, Plist::new());
        assert_eq!(font_obj.groups, None);
        assert_eq!(font_obj.kerning, None);
        assert_eq!(font_obj.features, None);
    }

    #[test]
    fn upconvert_ufov1_robofab_data() {
        let path = "testdata/fontinfotest_v1.ufo";
        let font = Font::load(path).unwrap();

        assert_eq!(font.meta.format_version, FormatVersion::V3);

        let font_info = font.font_info.unwrap();
        assert_eq!(font_info.postscript_blue_fuzz, Some(IntegerOrFloat::from(1)));
        assert_eq!(font_info.postscript_blue_scale, Some(0.039625));
        assert_eq!(font_info.postscript_blue_shift, Some(IntegerOrFloat::from(7)));
        assert_eq!(
            font_info.postscript_blue_values,
            Some(vec![
                IntegerOrFloat::from(-10),
                IntegerOrFloat::from(0),
                IntegerOrFloat::from(482),
                IntegerOrFloat::from(492),
                IntegerOrFloat::from(694),
                IntegerOrFloat::from(704),
                IntegerOrFloat::from(739),
                IntegerOrFloat::from(749)
            ])
        );
        assert_eq!(
            font_info.postscript_other_blues,
            Some(vec![IntegerOrFloat::from(-260), IntegerOrFloat::from(-250)])
        );
        assert_eq!(
            font_info.postscript_family_blues,
            Some(vec![IntegerOrFloat::from(500.0), IntegerOrFloat::from(510.0)])
        );
        assert_eq!(
            font_info.postscript_family_other_blues,
            Some(vec![IntegerOrFloat::from(-260), IntegerOrFloat::from(-250)])
        );
        assert_eq!(font_info.postscript_force_bold, Some(true));
        assert_eq!(
            font_info.postscript_stem_snap_h,
            Some(vec![IntegerOrFloat::from(100), IntegerOrFloat::from(120)])
        );
        assert_eq!(
            font_info.postscript_stem_snap_v,
            Some(vec![IntegerOrFloat::from(80), IntegerOrFloat::from(90)])
        );

        assert_eq!(font.lib.keys().collect::<Vec<&String>>(), vec!["org.robofab.testFontLibData"]);

        assert_eq!(
            font.features.unwrap(),
            "@myClass = [A B];\n\nfeature liga {\n    sub A A by b;\n} liga;\n"
        );
    }

    #[test]
    fn upconversion_fontinfo_v123() {
        let ufo_v1 = Font::load("testdata/fontinfotest_v1.ufo").unwrap();
        let ufo_v2 = Font::load("testdata/fontinfotest_v2.ufo").unwrap();
        let ufo_v3 = Font::load("testdata/fontinfotest_v3.ufo").unwrap();

        assert_eq!(ufo_v1, ufo_v3);
        assert_eq!(ufo_v2, ufo_v3);
    }

    #[test]
    fn metainfo() {
        let path = "testdata/mutatorSans/MutatorSansLightWide.ufo/metainfo.plist";
        let meta: MetaInfo = plist::from_file(path).expect("failed to load metainfo");
        assert_eq!(meta.creator, "org.robofab.ufoLib");
    }

    #[test]
    fn serialize_kerning() {
        let kerning: Kerning = btreemap! {
            "A".into() => btreemap!{
                "A".into() => 1.0,
            },
            "B".into() => btreemap!{
                "A".into() => 5.4,
            },
        };

        let kerning_serializer = KerningSerializer { kerning: &kerning };

        assert_ser_tokens(
            &kerning_serializer,
            &[
                Token::Map { len: Some(2) },
                Token::Str("A"),
                Token::Map { len: Some(1) },
                Token::Str("A"),
                Token::I32(1),
                Token::MapEnd,
                Token::Str("B"),
                Token::Map { len: Some(1) },
                Token::Str("A"),
                Token::F32(5.4),
                Token::MapEnd,
                Token::MapEnd,
            ],
        );
    }
}
