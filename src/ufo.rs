//! Reading and (maybe) writing Unified Font Object files.

#![deny(intra_doc_link_resolution_failure)]

use std::borrow::Borrow;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde::de::Deserializer;
use serde::Deserialize;

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

/// The contents of the [`fontinfo.plist`][] file. This structure is hard-wired to the
/// available attributes in UFO version 3.
///
/// [`fontinfo.plist`]: http://unifiedfontobject.org/versions/ufo3/fontinfo.plist/
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct FontInfo {
    pub ascender: Option<f64>,
    pub cap_height: Option<f64>,
    pub copyright: Option<String>,
    pub descender: Option<f64>,
    pub family_name: Option<String>,
    pub guidelines: Option<Vec<Guideline>>,
    pub italic_angle: Option<f64>,
    #[serde(rename = "macintoshFONDFamilyID")]
    pub macintosh_fond_family_id: Option<u32>,
    #[serde(rename = "macintoshFONDName")]
    pub macintosh_fond_name: Option<String>,
    pub note: Option<String>,
    pub open_type_gasp_range_records: Option<Vec<GaspRangeRecord>>,
    pub open_type_head_created: Option<String>, // TODO: Validate string
    pub open_type_head_flags: Option<Vec<u32>>,
    #[serde(rename = "openTypeHeadLowestRecPPEM")]
    pub open_type_head_lowest_rec_ppem: Option<u32>,
    pub open_type_hhea_ascender: Option<i32>,
    pub open_type_hhea_caret_offset: Option<i32>,
    pub open_type_hhea_caret_slope_rise: Option<i32>,
    pub open_type_hhea_caret_slope_run: Option<i32>,
    pub open_type_hhea_descender: Option<i32>,
    pub open_type_hhea_line_gap: Option<i32>,
    pub open_type_name_compatible_full_name: Option<String>,
    pub open_type_name_description: Option<String>,
    #[serde(rename = "openTypeNameDesignerURL")]
    pub open_type_name_designer_url: Option<String>,
    pub open_type_name_designer: Option<String>,
    #[serde(rename = "openTypeNameLicenseURL")]
    pub open_type_name_license_url: Option<String>,
    pub open_type_name_license: Option<String>,
    #[serde(rename = "openTypeNameManufacturerURL")]
    pub open_type_name_manufacturer_url: Option<String>,
    pub open_type_name_manufacturer: Option<String>,
    pub open_type_name_preferred_family_name: Option<String>,
    pub open_type_name_preferred_subfamily_name: Option<String>,
    pub open_type_name_records: Option<Vec<NameRecord>>,
    pub open_type_name_sample_text: Option<String>,
    #[serde(rename = "openTypeNameUniqueID")]
    pub open_type_name_unique_id: Option<String>,
    pub open_type_name_version: Option<String>,
    #[serde(rename = "openTypeNameWWSFamilyName")]
    pub open_type_name_wws_family_name: Option<String>,
    #[serde(rename = "openTypeNameWWSSubfamilyName")]
    pub open_type_name_wws_subfamily_name: Option<String>,
    #[serde(rename = "openTypeOS2CodePageRanges")]
    pub open_type_os2_code_page_ranges: Option<Vec<u8>>,
    #[serde(rename = "openTypeOS2FamilyClass")]
    pub open_type_os2_family_class: Option<OS2FamilyClass>, // TODO: validate, de/serialize from list
    #[serde(rename = "openTypeOS2Panose")]
    pub open_type_os2_panose: Option<OS2Panose>, // TODO: validate, de/serialize from list
    #[serde(rename = "openTypeOS2Selection")]
    pub open_type_os2_selection: Option<Vec<u8>>, // TODO: validate
    #[serde(rename = "openTypeOS2StrikeoutPosition")]
    pub open_type_os2_strikeout_position: Option<i32>,
    #[serde(rename = "openTypeOS2StrikeoutSize")]
    pub open_type_os2_strikeout_size: Option<i32>,
    #[serde(rename = "openTypeOS2SubscriptXOffset")]
    pub open_type_os2_subscript_x_offset: Option<i32>,
    #[serde(rename = "openTypeOS2SubscriptXSize")]
    pub open_type_os2_subscript_x_size: Option<i32>,
    #[serde(rename = "openTypeOS2SubscriptYOffset")]
    pub open_type_os2_subscript_y_offset: Option<i32>,
    #[serde(rename = "openTypeOS2SubscriptYSize")]
    pub open_type_os2_subscript_y_size: Option<i32>,
    #[serde(rename = "openTypeOS2SuperscriptXOffset")]
    pub open_type_os2_superscript_x_offset: Option<i32>,
    #[serde(rename = "openTypeOS2SuperscriptXSize")]
    pub open_type_os2_superscript_x_size: Option<i32>,
    #[serde(rename = "openTypeOS2SuperscriptYOffset")]
    pub open_type_os2_superscript_y_offset: Option<i32>,
    #[serde(rename = "openTypeOS2SuperscriptYSize")]
    pub open_type_os2_superscript_y_size: Option<i32>,
    #[serde(rename = "openTypeOS2Type")]
    pub open_type_os2_type: Option<Vec<u8>>,
    #[serde(rename = "openTypeOS2TypoAscender")]
    pub open_type_os2_typo_ascender: Option<i32>,
    #[serde(rename = "openTypeOS2TypoDescender")]
    pub open_type_os2_typo_descender: Option<i32>,
    #[serde(rename = "openTypeOS2TypoLineGap")]
    pub open_type_os2_typo_line_gap: Option<i32>,
    #[serde(rename = "openTypeOS2UnicodeRanges")]
    pub open_type_os2_unicode_ranges: Option<Vec<u8>>,
    #[serde(rename = "openTypeOS2VendorID")]
    pub open_type_os2_vendor_id: Option<String>, // TODO: validate, 4 characters.
    #[serde(rename = "openTypeOS2WeightClass")]
    pub open_type_os2_weight_class: Option<u32>, // TODO: validate
    #[serde(rename = "openTypeOS2WidthClass")]
    pub open_type_os2_width_class: Option<u32>, // TODO: validate
    #[serde(rename = "openTypeOS2WinAscent")]
    pub open_type_os2_win_ascent: Option<u32>,
    #[serde(rename = "openTypeOS2WinDescent")]
    pub open_type_os2_win_descent: Option<u32>,
    pub open_type_vhea_caret_offset: Option<i32>,
    pub open_type_vhea_caret_slope_rise: Option<i32>,
    pub open_type_vhea_caret_slope_run: Option<i32>,
    pub open_type_vhea_vert_typo_ascender: Option<i32>,
    pub open_type_vhea_vert_typo_descender: Option<i32>,
    pub open_type_vhea_vert_typo_line_gap: Option<i32>,
    pub postscript_blue_fuzz: Option<f64>,
    pub postscript_blue_scale: Option<f64>,
    pub postscript_blue_shift: Option<f64>,
    pub postscript_blue_values: Option<Vec<f64>>,
    pub postscript_default_character: Option<String>,
    pub postscript_default_width_x: Option<f64>,
    pub postscript_family_blues: Option<Vec<f64>>,
    pub postscript_family_other_blues: Option<Vec<f64>>,
    pub postscript_font_name: Option<String>,
    pub postscript_force_bold: Option<bool>,
    pub postscript_full_name: Option<String>,
    pub postscript_is_fixed_pitch: Option<bool>,
    pub postscript_nominal_width_x: Option<f64>,
    pub postscript_other_blues: Option<Vec<f64>>,
    pub postscript_slant_angle: Option<f64>,
    pub postscript_stem_snap_h: Option<Vec<f64>>, // TODO: validate. (i32|f64)?
    pub postscript_stem_snap_v: Option<Vec<f64>>, // TODO: validate. (i32|f64)?
    pub postscript_underline_position: Option<f64>,
    pub postscript_underline_thickness: Option<f64>,
    #[serde(rename = "postscriptUniqueID")]
    pub postscript_unique_id: Option<i32>,
    pub postscript_weight_name: Option<String>,
    pub postscript_windows_character_set: Option<PostscriptWindowsCharacterSet>,
    pub style_map_family_name: Option<String>,
    #[serde(deserialize_with = "deserialize_style_map_style_name")]
    pub style_map_style_name: Option<StyleMapStyle>,
    pub style_name: Option<String>,
    pub trademark: Option<String>,
    pub units_per_em: Option<f64>,
    pub version_major: Option<u32>,
    pub version_minor: Option<u32>,
    pub woff_major_version: Option<i32>,
    pub woff_metadata_copyright: Option<WoffMetadataCopyright>,
    pub woff_metadata_credits: Option<WoffMetadataCredits>,
    pub woff_metadata_description: Option<WoffMetadataDescription>,
    pub woff_metadata_extensions: Option<Vec<WoffMetadataExtensionRecord>>,
    pub woff_metadata_license: Option<WoffMetadataLicense>,
    pub woff_metadata_licensee: Option<WoffMetadataLicensee>,
    pub woff_metadata_trademark: Option<WoffMetadataTrademark>,
    #[serde(rename = "woffMetadataUniqueID")]
    pub woff_metadata_unique_id: Option<WoffMetadataUniqueID>,
    pub woff_metadata_vendor: Option<WoffMetadataVendor>,
    pub woff_minor_version: Option<i32>,
    pub x_height: Option<f64>,
    pub year: Option<u32>,
}

// TODO: validate!
// http://unifiedfontobject.org/versions/ufo3/fontinfo.plist/#guidelines
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Guideline {
    x: Option<f64>,
    y: Option<f64>,
    angle: Option<f64>,
    name: String,
    color: String,
    identifier: String,
}

// TODO: validate!
// http://unifiedfontobject.org/versions/ufo3/fontinfo.plist/#opentype-gasp-table-fields
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GaspRangeRecord {
    range_max_ppem: u16,
    range_gasp_behavior: Vec<u8>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct NameRecord {
    name_id: u16,
    paltform_id: u16,
    encoding_id: u16,
    language_id: u16,
    string: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OS2FamilyClass {
    class_id: u8,
    subclass_id: u8,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OS2Panose {
    family_type: u8,
    serif_style: u8,
    weight: u8,
    proportion: u8,
    contrast: u8,
    stroke_variation: u8,
    arm_style: u8,
    letterform: u8,
    midline: u8,
    x_height: u8,
}

#[derive(Debug, Clone, Copy, Serialize_repr, Deserialize_repr, PartialEq)]
#[repr(u8)]
pub enum PostscriptWindowsCharacterSet {
    ANSI = 1,
    Default = 2,
    Symbol = 3,
    Macintosh = 4,
    ShiftJIS = 5,
    Hangul = 6,
    HangulJohab = 7,
    GB2312 = 8,
    ChineseBIG5 = 9,
    Greek = 10,
    Turkish = 11,
    Vietnamese = 12,
    Hebrew = 13,
    Arabic = 14,
    Baltic = 15,
    Bitstream = 16,
    Cyrillic = 17,
    Thai = 18,
    EasternEuropean = 19,
    OEM = 20,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WoffMetadataCopyright {
    text: Vec<WoffMetadataTextRecord>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WoffMetadataCredits {
    credits: Vec<WoffMetadataCredit>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WoffMetadataCredit {
    name: String,
    url: Option<String>,
    role: Option<String>,
    dir: Option<String>,
    class: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WoffMetadataDescription {
    url: Option<String>,
    text: Vec<WoffMetadataTextRecord>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WoffMetadataTextRecord {
    text: String,
    language: Option<String>,
    dir: Option<String>,
    class: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WoffMetadataExtensionRecord {
    id: Option<String>,
    names: Vec<WoffMetadataExtensionNameRecord>,
    items: Vec<WoffMetadataExtensionItemRecord>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WoffMetadataExtensionNameRecord {
    text: String,
    language: Option<String>,
    dir: Option<String>,
    class: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WoffMetadataExtensionItemRecord {
    id: Option<String>,
    names: Vec<WoffMetadataExtensionNameRecord>,
    values: Vec<WoffMetadataExtensionValueRecord>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WoffMetadataExtensionValueRecord {
    text: String,
    language: Option<String>,
    dir: Option<String>,
    class: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WoffMetadataLicense {
    url: Option<String>,
    id: Option<String>,
    text: Vec<WoffMetadataTextRecord>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WoffMetadataLicensee {
    name: String,
    dir: Option<String>,
    class: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WoffMetadataTrademark {
    text: Vec<WoffMetadataTextRecord>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WoffMetadataUniqueID {
    id: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WoffMetadataVendor {
    name: String,
    url: String,
    dir: Option<String>,
    class: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Eq, PartialEq, Clone)]
pub enum StyleMapStyle {
    Regular,
    Italic,
    Bold,
    BoldItalic,
}

fn deserialize_style_map_style_name<'de, D>(de: D) -> Result<Option<StyleMapStyle>, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    struct Helper(#[serde(with = "String")] String);

    let helper = Option::deserialize(de)?;
    helper.map_or(Ok(None), |Helper(external)| match external.as_ref() {
        "regular" => Ok(Some(StyleMapStyle::Regular)),
        "italic" => Ok(Some(StyleMapStyle::Italic)),
        "bold" => Ok(Some(StyleMapStyle::Bold)),
        "bold italic" => Ok(Some(StyleMapStyle::BoldItalic)),
        _ => Err(serde::de::Error::custom("unknown value for styleMapStyleName.")),
    })
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

    #[test]
    fn fontinfo() {
        let path = "testdata/mutatorSans/MutatorSansLightWide.ufo/fontinfo.plist";
        let font_info: FontInfo = plist::from_file(path).expect("failed to load fontinfo");
        assert_eq!(font_info.family_name, Some("MutatorMathTest".to_string()));
        assert_eq!(font_info.trademark, None);
        assert_eq!(font_info.style_map_style_name, Some(StyleMapStyle::Regular));
        assert_eq!(font_info.open_type_os2_vendor_id, Some("LTTR".into()));
    }
}
