use serde::de::{Deserializer, SeqAccess, Visitor};
use serde::Deserialize;
use std::fmt;

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
    pub guidelines: Option<Vec<Guideline>>, // TODO: Use same struct as glyph::guideline
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
    pub open_type_os2_family_class: Option<OS2FamilyClass>,
    #[serde(rename = "openTypeOS2Panose")]
    pub open_type_os2_panose: Option<OS2Panose>,
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
    pub woff_metadata_extensions: Option<Vec<WoffMetadataExtensionRecord>>, // TODO: validate must have 1+ items
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
#[serde(deny_unknown_fields)]
pub struct Guideline {
    x: Option<f64>,
    y: Option<f64>,
    angle: Option<f64>,
    name: Option<String>,
    color: Option<String>,
    identifier: Option<String>,
}

// TODO: validate!
// http://unifiedfontobject.org/versions/ufo3/fontinfo.plist/#opentype-gasp-table-fields
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct GaspRangeRecord {
    #[serde(rename = "rangeMaxPPEM")]
    range_max_ppem: u16,
    range_gasp_behavior: Vec<u8>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NameRecord {
    #[serde(rename = "nameID")]
    name_id: u16,
    #[serde(rename = "platformID")]
    platform_id: u16,
    #[serde(rename = "encodingID")]
    encoding_id: u16,
    #[serde(rename = "languageID")]
    language_id: u16,
    string: String,
}

#[derive(Debug, Clone, Default, Serialize, PartialEq)]
pub struct OS2FamilyClass {
    class_id: u8,
    subclass_id: u8,
}

struct OS2FamilyClassVisitor;

impl<'de> Visitor<'de> for OS2FamilyClassVisitor {
    type Value = OS2FamilyClass;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a list of two u8s.")
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let class_id: u8 = seq.next_element().unwrap().unwrap();
        let subclass_id: u8 = seq.next_element().unwrap().unwrap();

        if let Ok(Some(_)) = seq.next_element::<u8>() {
            return Err(serde::de::Error::custom(
                "openTypeOS2FamilyClass must have exactly two elements but has more.",
            ));
        }

        Ok(OS2FamilyClass { class_id, subclass_id })
    }
}

impl<'de> Deserialize<'de> for OS2FamilyClass {
    fn deserialize<D>(deserializer: D) -> Result<OS2FamilyClass, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_seq(OS2FamilyClassVisitor)
    }
}

#[derive(Debug, Clone, Default, Serialize, PartialEq)]
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

struct OS2PanoseVisitor;

impl<'de> Visitor<'de> for OS2PanoseVisitor {
    type Value = OS2Panose;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a list of ten u8s.")
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let family_type: u8 = seq.next_element().unwrap().unwrap();
        let serif_style: u8 = seq.next_element().unwrap().unwrap();
        let weight: u8 = seq.next_element().unwrap().unwrap();
        let proportion: u8 = seq.next_element().unwrap().unwrap();
        let contrast: u8 = seq.next_element().unwrap().unwrap();
        let stroke_variation: u8 = seq.next_element().unwrap().unwrap();
        let arm_style: u8 = seq.next_element().unwrap().unwrap();
        let letterform: u8 = seq.next_element().unwrap().unwrap();
        let midline: u8 = seq.next_element().unwrap().unwrap();
        let x_height: u8 = seq.next_element().unwrap().unwrap();

        if let Ok(Some(_)) = seq.next_element::<u8>() {
            return Err(serde::de::Error::custom(
                "openTypeOS2Panose must have exactly ten elements but has more.",
            ));
        }

        Ok(OS2Panose {
            family_type,
            serif_style,
            weight,
            proportion,
            contrast,
            stroke_variation,
            arm_style,
            letterform,
            midline,
            x_height,
        })
    }
}

impl<'de> Deserialize<'de> for OS2Panose {
    fn deserialize<D>(deserializer: D) -> Result<OS2Panose, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_seq(OS2PanoseVisitor)
    }
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
    text: Vec<WoffMetadataTextRecord>, // TODO: validate must have 1+ items
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WoffMetadataCredits {
    credits: Vec<WoffMetadataCredit>, // TODO: validate must have 1+ items
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WoffMetadataCredit {
    name: String,
    url: Option<String>,
    role: Option<String>,
    dir: Option<String>, // TODO: Option<"ltr" | "rtl">
    class: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WoffMetadataDescription {
    url: Option<String>,
    text: Vec<WoffMetadataTextRecord>, // TODO: validate must have 1+ items
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WoffMetadataTextRecord {
    text: String,
    language: Option<String>,
    dir: Option<String>, // TODO: Option<"ltr" | "rtl">
    class: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WoffMetadataExtensionRecord {
    id: Option<String>,
    names: Vec<WoffMetadataExtensionNameRecord>,
    items: Vec<WoffMetadataExtensionItemRecord>, // TODO: validate must have 1+ items
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WoffMetadataExtensionNameRecord {
    text: String,
    language: Option<String>,
    dir: Option<String>, // TODO: Option<"ltr" | "rtl">
    class: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WoffMetadataExtensionItemRecord {
    id: Option<String>,
    names: Vec<WoffMetadataExtensionNameRecord>, // TODO: validate must have 1+ items
    values: Vec<WoffMetadataExtensionValueRecord>, // TODO: validate must have 1+ items
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WoffMetadataExtensionValueRecord {
    text: String,
    language: Option<String>,
    dir: Option<String>, // TODO: Option<"ltr" | "rtl">
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
    dir: Option<String>, // TODO: Option<"ltr" | "rtl">
    class: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WoffMetadataTrademark {
    text: Vec<WoffMetadataTextRecord>, // TODO: validate must have 1+ items
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WoffMetadataUniqueID {
    id: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WoffMetadataVendor {
    name: String,
    url: String,
    dir: Option<String>, // TODO: Option<"ltr" | "rtl">
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

mod tests {
    use super::*;

    #[test]
    fn fontinfo() {
        let path = "testdata/mutatorSans/MutatorSansLightWide.ufo/fontinfo.plist";
        let font_info: FontInfo = plist::from_file(path).expect("failed to load fontinfo");
        assert_eq!(font_info.family_name, Some("MutatorMathTest".to_string()));
        assert_eq!(font_info.trademark, None);
        assert_eq!(font_info.style_map_style_name, Some(StyleMapStyle::Regular));
        assert_eq!(font_info.open_type_os2_vendor_id, Some("LTTR".into()));
    }

    #[test]
    fn fontinfo2() {
        let path = "testdata/fontinfotest.ufo/fontinfo.plist";
        let font_info: FontInfo = plist::from_file(path).expect("failed to load fontinfo");
        assert_eq!(font_info.family_name, Some("a".to_string()));
        assert_eq!(
            font_info.open_type_os2_family_class,
            Some(OS2FamilyClass { class_id: 0, subclass_id: 0 })
        );
        assert_eq!(
            font_info.open_type_os2_panose,
            Some(OS2Panose {
                family_type: 2,
                serif_style: 2,
                weight: 2,
                proportion: 2,
                contrast: 6,
                stroke_variation: 5,
                arm_style: 11,
                letterform: 4,
                midline: 2,
                x_height: 5,
            })
        );
    }
}
