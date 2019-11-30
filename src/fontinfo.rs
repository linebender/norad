use serde::de::{Deserializer, SeqAccess, Visitor};
use serde::ser::{SerializeSeq, Serializer};
use serde::{Deserialize, Serialize};
use std::fmt;

use crate::shared_types::Guideline;
use crate::Error;

// The specification is vague about data type limits, usually implicitly meaning
// Python types. Since Python is dynamic, the spec does not nail down the exact type
// in several locations and we have to assume the biggest type that can hold it.
type Integer = i32;
type NonNegativeInteger = u32;
type IntegerOrFloat = f64;
type Float = f64;
type NonNegativeIntegerOrFloat = f64; // Must be validated to be non-negative.
type Bitlist = Vec<u8>;

/// The contents of the [`fontinfo.plist`][] file. This structure is hard-wired to the
/// available attributes in UFO version 3.
///
/// [`fontinfo.plist`]: http://unifiedfontobject.org/versions/ufo3/fontinfo.plist/
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct FontInfo {
    // Generic Identification Information
    pub family_name: Option<String>,
    pub style_name: Option<String>,
    pub style_map_family_name: Option<String>,
    pub style_map_style_name: Option<StyleMapStyle>,
    pub version_major: Option<Integer>,
    pub version_minor: Option<NonNegativeInteger>,
    pub year: Option<Integer>,

    // Generic Legal Information
    pub copyright: Option<String>,
    pub trademark: Option<String>,

    // Generic Dimension Information
    pub units_per_em: Option<NonNegativeIntegerOrFloat>,
    pub descender: Option<IntegerOrFloat>,
    pub x_height: Option<IntegerOrFloat>,
    pub cap_height: Option<IntegerOrFloat>,
    pub ascender: Option<IntegerOrFloat>,
    pub italic_angle: Option<IntegerOrFloat>,

    // Guidelines
    pub guidelines: Option<Vec<Guideline>>,

    // Generic Miscellaneous Information
    pub note: Option<String>,

    // OpenType gasp Table Fields
    pub open_type_gasp_range_records: Option<Vec<GaspRangeRecord>>,

    // OpenType head Table Fields
    pub open_type_head_created: Option<String>,
    #[serde(rename = "openTypeHeadLowestRecPPEM")]
    pub open_type_head_lowest_rec_ppem: Option<NonNegativeInteger>,
    pub open_type_head_flags: Option<Bitlist>,

    // OpenType hhea Table Fields
    pub open_type_hhea_ascender: Option<Integer>,
    pub open_type_hhea_descender: Option<Integer>,
    pub open_type_hhea_line_gap: Option<Integer>,
    pub open_type_hhea_caret_slope_rise: Option<Integer>,
    pub open_type_hhea_caret_slope_run: Option<Integer>,
    pub open_type_hhea_caret_offset: Option<Integer>,

    // OpenType Name Table Fields
    pub open_type_name_designer: Option<String>,
    #[serde(rename = "openTypeNameDesignerURL")]
    pub open_type_name_designer_url: Option<String>,
    pub open_type_name_manufacturer: Option<String>,
    #[serde(rename = "openTypeNameManufacturerURL")]
    pub open_type_name_manufacturer_url: Option<String>,
    pub open_type_name_license: Option<String>,
    #[serde(rename = "openTypeNameLicenseURL")]
    pub open_type_name_license_url: Option<String>,
    pub open_type_name_version: Option<String>,
    #[serde(rename = "openTypeNameUniqueID")]
    pub open_type_name_unique_id: Option<String>,
    pub open_type_name_description: Option<String>,
    pub open_type_name_preferred_family_name: Option<String>,
    pub open_type_name_preferred_subfamily_name: Option<String>,
    pub open_type_name_compatible_full_name: Option<String>,
    pub open_type_name_sample_text: Option<String>,
    #[serde(rename = "openTypeNameWWSFamilyName")]
    pub open_type_name_wws_family_name: Option<String>,
    #[serde(rename = "openTypeNameWWSSubfamilyName")]
    pub open_type_name_wws_subfamily_name: Option<String>,
    pub open_type_name_records: Option<Vec<NameRecord>>,

    // OpenType OS/2 Table Fields
    #[serde(rename = "openTypeOS2WidthClass")]
    pub open_type_os2_width_class: Option<OS2WidthClass>,
    #[serde(rename = "openTypeOS2WeightClass")]
    pub open_type_os2_weight_class: Option<NonNegativeInteger>,
    #[serde(rename = "openTypeOS2Selection")]
    pub open_type_os2_selection: Option<Bitlist>,
    #[serde(rename = "openTypeOS2VendorID")]
    pub open_type_os2_vendor_id: Option<String>,
    #[serde(rename = "openTypeOS2Panose")]
    pub open_type_os2_panose: Option<OS2Panose>,
    #[serde(rename = "openTypeOS2FamilyClass")]
    pub open_type_os2_family_class: Option<OS2FamilyClass>,

    #[serde(rename = "openTypeOS2UnicodeRanges")]
    pub open_type_os2_unicode_ranges: Option<Bitlist>,
    #[serde(rename = "openTypeOS2CodePageRanges")]
    pub open_type_os2_code_page_ranges: Option<Bitlist>,

    #[serde(rename = "openTypeOS2TypoAscender")]
    pub open_type_os2_typo_ascender: Option<Integer>,
    #[serde(rename = "openTypeOS2TypoDescender")]
    pub open_type_os2_typo_descender: Option<Integer>,
    #[serde(rename = "openTypeOS2TypoLineGap")]
    pub open_type_os2_typo_line_gap: Option<Integer>,
    #[serde(rename = "openTypeOS2WinAscent")]
    pub open_type_os2_win_ascent: Option<NonNegativeInteger>,
    #[serde(rename = "openTypeOS2WinDescent")]
    pub open_type_os2_win_descent: Option<NonNegativeInteger>,

    #[serde(rename = "openTypeOS2Type")]
    pub open_type_os2_type: Option<Bitlist>,

    #[serde(rename = "openTypeOS2SubscriptXSize")]
    pub open_type_os2_subscript_x_size: Option<Integer>,
    #[serde(rename = "openTypeOS2SubscriptYSize")]
    pub open_type_os2_subscript_y_size: Option<Integer>,
    #[serde(rename = "openTypeOS2SubscriptXOffset")]
    pub open_type_os2_subscript_x_offset: Option<Integer>,
    #[serde(rename = "openTypeOS2SubscriptYOffset")]
    pub open_type_os2_subscript_y_offset: Option<Integer>,
    #[serde(rename = "openTypeOS2SuperscriptXSize")]
    pub open_type_os2_superscript_x_size: Option<Integer>,
    #[serde(rename = "openTypeOS2SuperscriptYSize")]
    pub open_type_os2_superscript_y_size: Option<Integer>,
    #[serde(rename = "openTypeOS2SuperscriptXOffset")]
    pub open_type_os2_superscript_x_offset: Option<Integer>,
    #[serde(rename = "openTypeOS2SuperscriptYOffset")]
    pub open_type_os2_superscript_y_offset: Option<Integer>,

    #[serde(rename = "openTypeOS2StrikeoutSize")]
    pub open_type_os2_strikeout_size: Option<Integer>,
    #[serde(rename = "openTypeOS2StrikeoutPosition")]
    pub open_type_os2_strikeout_position: Option<Integer>,

    // OpenType vhea Table Fields
    pub open_type_vhea_vert_typo_ascender: Option<Integer>,
    pub open_type_vhea_vert_typo_descender: Option<Integer>,
    pub open_type_vhea_vert_typo_line_gap: Option<Integer>,
    pub open_type_vhea_caret_slope_rise: Option<Integer>,
    pub open_type_vhea_caret_slope_run: Option<Integer>,
    pub open_type_vhea_caret_offset: Option<Integer>,

    // PostScript Specific Data
    pub postscript_font_name: Option<String>,
    pub postscript_full_name: Option<String>,
    pub postscript_slant_angle: Option<IntegerOrFloat>,
    #[serde(rename = "postscriptUniqueID")]
    pub postscript_unique_id: Option<Integer>,
    pub postscript_underline_thickness: Option<IntegerOrFloat>,
    pub postscript_underline_position: Option<IntegerOrFloat>,
    pub postscript_is_fixed_pitch: Option<bool>,
    pub postscript_blue_values: Option<Vec<IntegerOrFloat>>,
    pub postscript_other_blues: Option<Vec<IntegerOrFloat>>,
    pub postscript_family_blues: Option<Vec<IntegerOrFloat>>,
    pub postscript_family_other_blues: Option<Vec<IntegerOrFloat>>,
    pub postscript_stem_snap_h: Option<Vec<IntegerOrFloat>>,
    pub postscript_stem_snap_v: Option<Vec<IntegerOrFloat>>,
    pub postscript_blue_fuzz: Option<IntegerOrFloat>,
    pub postscript_blue_shift: Option<IntegerOrFloat>,
    pub postscript_blue_scale: Option<Float>,
    pub postscript_force_bold: Option<bool>,
    pub postscript_default_width_x: Option<IntegerOrFloat>,
    pub postscript_nominal_width_x: Option<IntegerOrFloat>,
    pub postscript_weight_name: Option<String>,
    pub postscript_default_character: Option<String>,
    pub postscript_windows_character_set: Option<PostscriptWindowsCharacterSet>,

    // Macintosh FOND Resource Data
    #[serde(rename = "macintoshFONDFamilyID")]
    pub macintosh_fond_family_id: Option<Integer>,
    #[serde(rename = "macintoshFONDName")]
    pub macintosh_fond_name: Option<String>,

    // WOFF Data
    pub woff_major_version: Option<NonNegativeInteger>,
    pub woff_minor_version: Option<NonNegativeInteger>,
    #[serde(rename = "woffMetadataUniqueID")]
    pub woff_metadata_unique_id: Option<WoffMetadataUniqueID>,
    pub woff_metadata_vendor: Option<WoffMetadataVendor>,
    pub woff_metadata_credits: Option<WoffMetadataCredits>,
    pub woff_metadata_description: Option<WoffMetadataDescription>,
    pub woff_metadata_license: Option<WoffMetadataLicense>,
    pub woff_metadata_copyright: Option<WoffMetadataCopyright>,
    pub woff_metadata_trademark: Option<WoffMetadataTrademark>,
    pub woff_metadata_licensee: Option<WoffMetadataLicensee>,
    pub woff_metadata_extensions: Option<Vec<WoffMetadataExtensionRecord>>,
}

impl FontInfo {
    /// Validates various fields according to the [specification][].
    ///
    /// [specification]: http://unifiedfontobject.org/versions/ufo3/fontinfo.plist/
    pub fn validate(&self) -> Result<(), Error> {
        // unitsPerEm must be non-negative.
        if let Some(v) = self.units_per_em {
            if v.is_sign_negative() {
                return Err(Error::FontInfoError);
            }
        }

        // The date format is "YYYY/MM/DD HH:MM:SS". This does not validate that the
        // days ceiling is valid for the month, as this would probably need a specialist
        // datetime library.
        if let Some(v) = &self.open_type_head_created {
            const DATE_LENGTH: usize = 19;
            if v.len() != DATE_LENGTH {
                return Err(Error::FontInfoError);
            }
            if !v.chars().all(|b| b.is_ascii_digit() || b == ' ' || b == '/' || b == ':') {
                return Err(Error::FontInfoError);
            }

            if !(v[0..4].parse::<u16>().is_ok()
                && &v[4..5] == "/"
                && v[5..7].parse::<u8>().unwrap_or(99) <= 12
                && &v[7..8] == "/"
                && v[8..10].parse::<u8>().unwrap_or(99) <= 31
                && &v[10..11] == " "
                && v[11..13].parse::<u8>().unwrap_or(99) < 24
                && &v[13..14] == ":"
                && v[14..16].parse::<u8>().unwrap_or(99) < 60
                && &v[16..17] == ":"
                && v[17..19].parse::<u8>().unwrap_or(99) < 60)
            {
                return Err(Error::FontInfoError);
            }
        }

        // These must be sorted in ascending order based on the rangeMaxPPEM value of
        // the record.
        if let Some(v) = &self.open_type_gasp_range_records {
            // No or one entry are always in the right order.
            if v.len() > 1 {
                let vs: Vec<u32> = v.iter().map(|g| g.range_max_ppem).collect();

                let mut vs_iter = vs.iter();
                let mut last = vs_iter.next().unwrap();
                while let Some(current) = vs_iter.next() {
                    if last > current {
                        return Err(Error::FontInfoError);
                    }
                    last = current;
                }
            }
        }

        // openTypeOS2Selection must not contain bits 0, 5 or 6.
        if let Some(v) = &self.open_type_os2_selection {
            if v.contains(&0) || v.contains(&5) || v.contains(&6) {
                return Err(Error::FontInfoError);
            }
        }

        if let Some(v) = &self.open_type_os2_family_class {
            if !v.is_valid() {
                return Err(Error::FontInfoError);
            }
        }

        // The Postscript blue zone and stem widths lists have a length limitation.
        if let Some(v) = &self.postscript_blue_values {
            if v.len() > 14 {
                return Err(Error::FontInfoError);
            }
        }
        if let Some(v) = &self.postscript_other_blues {
            if v.len() > 10 {
                return Err(Error::FontInfoError);
            }
        }
        if let Some(v) = &self.postscript_family_blues {
            if v.len() > 14 {
                return Err(Error::FontInfoError);
            }
        }
        if let Some(v) = &self.postscript_family_other_blues {
            if v.len() > 10 {
                return Err(Error::FontInfoError);
            }
        }
        if let Some(v) = &self.postscript_stem_snap_h {
            if v.len() > 12 {
                return Err(Error::FontInfoError);
            }
        }
        if let Some(v) = &self.postscript_stem_snap_v {
            if v.len() > 12 {
                return Err(Error::FontInfoError);
            }
        }

        // Certain WOFF attributes must contain at least one item if they are present.
        if let Some(v) = &self.woff_metadata_extensions {
            if v.len() == 0 {
                return Err(Error::FontInfoError);
            }

            for record in v.iter() {
                if record.items.len() == 0 {
                    return Err(Error::FontInfoError);
                }

                for record_item in record.items.iter() {
                    if record_item.names.len() == 0 || record_item.values.len() == 0 {
                        return Err(Error::FontInfoError);
                    }
                }
            }
        }
        if let Some(v) = &self.woff_metadata_credits {
            if v.credits.len() == 0 {
                return Err(Error::FontInfoError);
            }
        }
        if let Some(v) = &self.woff_metadata_copyright {
            if v.text.len() == 0 {
                return Err(Error::FontInfoError);
            }
        }
        if let Some(v) = &self.woff_metadata_description {
            if v.text.len() == 0 {
                return Err(Error::FontInfoError);
            }
        }
        if let Some(v) = &self.woff_metadata_trademark {
            if v.text.len() == 0 {
                return Err(Error::FontInfoError);
            }
        }

        Ok(())
    }
}

/// Corresponds to [gasp Range Record Format](http://unifiedfontobject.org/versions/ufo3/fontinfo.plist/#gasp-range-record-format).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct GaspRangeRecord {
    #[serde(rename = "rangeMaxPPEM")]
    range_max_ppem: NonNegativeInteger,
    range_gasp_behavior: Vec<GaspBehavior>,
}

/// Corresponds to [rangeGaspBehavior Bits](http://unifiedfontobject.org/versions/ufo3/fontinfo.plist/#rangegaspbehavior-bits).
#[derive(Debug, Clone, Copy, Serialize_repr, Deserialize_repr, PartialEq)]
#[repr(u8)]
pub enum GaspBehavior {
    Gridfit = 0,
    DoGray = 1,
    SymmetricGridfit = 2,
    SymmetricSmoothing = 3,
}

/// Corresponds to [Name Record Format](http://unifiedfontobject.org/versions/ufo3/fontinfo.plist/#name-record-format).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NameRecord {
    #[serde(rename = "nameID")]
    name_id: NonNegativeInteger,
    #[serde(rename = "platformID")]
    platform_id: NonNegativeInteger,
    #[serde(rename = "encodingID")]
    encoding_id: NonNegativeInteger,
    #[serde(rename = "languageID")]
    language_id: NonNegativeInteger,
    string: String,
}

/// Corresponds to the allowed values for [openTypeOS2WidthClass](http://unifiedfontobject.org/versions/ufo3/fontinfo.plist/#opentype-os2-table-fields).
#[derive(Debug, Clone, Copy, Serialize_repr, Deserialize_repr, PartialEq)]
#[repr(u8)]
pub enum OS2WidthClass {
    UltraCondensed = 1,
    ExtraCondensed = 2,
    Condensed = 3,
    SemiCondensed = 4,
    Normal = 5,
    SemiExpanded = 6,
    Expanded = 7,
    ExtraExpanded = 8,
    UltraExpanded = 9,
}

/// Corresponds to [openTypeOS2FamilyClass](http://unifiedfontobject.org/versions/ufo3/fontinfo.plist/#opentype-os2-table-fields).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct OS2FamilyClass {
    class_id: u8,
    subclass_id: u8,
}

impl OS2FamilyClass {
    /// The first number, representing the class ID, must be in the range 0-14.
    /// The second number, representing the subclass, must be in the range 0-15.
    fn is_valid(&self) -> bool {
        if (0..=14).contains(&self.class_id) && (0..=15).contains(&self.subclass_id) {
            true
        } else {
            false
        }
    }
}

impl Serialize for OS2FamilyClass {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        if !self.is_valid() {
            return Err(serde::ser::Error::custom("openTypeOS2FamilyClass class ID must be in the range 0-14, subclass ID must be in the range 0-15."));
        }

        let mut seq = serializer.serialize_seq(Some(2))?;
        seq.serialize_element(&self.class_id)?;
        seq.serialize_element(&self.subclass_id)?;
        seq.end()
    }
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

        let family_class = OS2FamilyClass { class_id, subclass_id };
        if !family_class.is_valid() {
            return Err(serde::de::Error::custom("openTypeOS2FamilyClass class ID must be in the range 0-14, subclass ID must be in the range 0-15."));
        }

        Ok(family_class)
    }
}

impl<'de> Deserialize<'de> for OS2FamilyClass {
    fn deserialize<D>(deserializer: D) -> Result<OS2FamilyClass, D::Error>
    where
        D: Deserializer<'de>,
    {
        // Take a plist array and turn it into a Rust struct.
        deserializer.deserialize_seq(OS2FamilyClassVisitor)
    }
}

/// Corresponds to [openTypeOS2Panose](http://unifiedfontobject.org/versions/ufo3/fontinfo.plist/#opentype-os2-table-fields).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct OS2Panose {
    family_type: NonNegativeInteger,
    serif_style: NonNegativeInteger,
    weight: NonNegativeInteger,
    proportion: NonNegativeInteger,
    contrast: NonNegativeInteger,
    stroke_variation: NonNegativeInteger,
    arm_style: NonNegativeInteger,
    letterform: NonNegativeInteger,
    midline: NonNegativeInteger,
    x_height: NonNegativeInteger,
}

impl Serialize for OS2Panose {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut seq = serializer.serialize_seq(Some(10))?;
        seq.serialize_element(&self.family_type)?;
        seq.serialize_element(&self.serif_style)?;
        seq.serialize_element(&self.weight)?;
        seq.serialize_element(&self.proportion)?;
        seq.serialize_element(&self.contrast)?;
        seq.serialize_element(&self.stroke_variation)?;
        seq.serialize_element(&self.arm_style)?;
        seq.serialize_element(&self.letterform)?;
        seq.serialize_element(&self.midline)?;
        seq.serialize_element(&self.x_height)?;
        seq.end()
    }
}

struct OS2PanoseVisitor;

impl<'de> Visitor<'de> for OS2PanoseVisitor {
    type Value = OS2Panose;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a list of ten non-negative integers.")
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let family_type: NonNegativeInteger = seq.next_element().unwrap().unwrap();
        let serif_style: NonNegativeInteger = seq.next_element().unwrap().unwrap();
        let weight: NonNegativeInteger = seq.next_element().unwrap().unwrap();
        let proportion: NonNegativeInteger = seq.next_element().unwrap().unwrap();
        let contrast: NonNegativeInteger = seq.next_element().unwrap().unwrap();
        let stroke_variation: NonNegativeInteger = seq.next_element().unwrap().unwrap();
        let arm_style: NonNegativeInteger = seq.next_element().unwrap().unwrap();
        let letterform: NonNegativeInteger = seq.next_element().unwrap().unwrap();
        let midline: NonNegativeInteger = seq.next_element().unwrap().unwrap();
        let x_height: NonNegativeInteger = seq.next_element().unwrap().unwrap();

        if let Ok(Some(_)) = seq.next_element::<NonNegativeInteger>() {
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

/// Corresponds to postscriptWindowsCharacterSet in [PostScript Specific Data](http://unifiedfontobject.org/versions/ufo3/fontinfo.plist/#postscript-specific-data).
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

/// Corresponds to woffMetadataCopyright in [WOFF Data](http://unifiedfontobject.org/versions/ufo3/fontinfo.plist/#woff-data).
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct WoffMetadataCopyright {
    text: Vec<WoffMetadataTextRecord>,
}

/// Corresponds to woffMetadataCredits in [WOFF Data](http://unifiedfontobject.org/versions/ufo3/fontinfo.plist/#woff-data).
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct WoffMetadataCredits {
    credits: Vec<WoffMetadataCredit>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct WoffMetadataCredit {
    name: String,
    url: Option<String>,
    role: Option<String>,
    dir: Option<WoffAttributeDirection>,
    class: Option<String>,
}

/// Corresponds to woffMetadataDescription in [WOFF Data](http://unifiedfontobject.org/versions/ufo3/fontinfo.plist/#woff-data).
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct WoffMetadataDescription {
    url: Option<String>,
    text: Vec<WoffMetadataTextRecord>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct WoffMetadataTextRecord {
    text: String,
    language: Option<String>,
    dir: Option<WoffAttributeDirection>,
    class: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct WoffMetadataExtensionRecord {
    id: Option<String>,
    names: Vec<WoffMetadataExtensionNameRecord>,
    items: Vec<WoffMetadataExtensionItemRecord>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct WoffMetadataExtensionNameRecord {
    text: String,
    language: Option<String>,
    dir: Option<WoffAttributeDirection>,
    class: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct WoffMetadataExtensionItemRecord {
    id: Option<String>, // XXX: Spec does not specify if required, assume optional.
    names: Vec<WoffMetadataExtensionNameRecord>,
    values: Vec<WoffMetadataExtensionValueRecord>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct WoffMetadataExtensionValueRecord {
    text: String,
    language: Option<String>,
    dir: Option<WoffAttributeDirection>,
    class: Option<String>,
}

/// Corresponds to woffMetadataLicense in [WOFF Data](http://unifiedfontobject.org/versions/ufo3/fontinfo.plist/#woff-data).
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct WoffMetadataLicense {
    url: Option<String>,
    id: Option<String>,
    text: Vec<WoffMetadataTextRecord>,
}

/// Corresponds to woffMetadataLicensee in [WOFF Data](http://unifiedfontobject.org/versions/ufo3/fontinfo.plist/#woff-data).
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct WoffMetadataLicensee {
    name: String,
    dir: Option<WoffAttributeDirection>,
    class: Option<String>,
}

/// Corresponds to woffMetadataTrademark in [WOFF Data](http://unifiedfontobject.org/versions/ufo3/fontinfo.plist/#woff-data).
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct WoffMetadataTrademark {
    text: Vec<WoffMetadataTextRecord>,
}

/// Corresponds to woffMetadataUniqueID in [WOFF Data](http://unifiedfontobject.org/versions/ufo3/fontinfo.plist/#woff-data).
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct WoffMetadataUniqueID {
    id: String,
}

/// Corresponds to woffMetadataVendor in [WOFF Data](http://unifiedfontobject.org/versions/ufo3/fontinfo.plist/#woff-data).
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct WoffMetadataVendor {
    name: String,
    url: String,
    dir: Option<WoffAttributeDirection>,
    class: Option<String>,
}

/// Corresponds to the writing direction attribute used in [WOFF Data](http://unifiedfontobject.org/versions/ufo3/fontinfo.plist/#woff-data).
/// If present, is either "ltr" or "rtl".
#[derive(Debug, Eq, PartialEq, Clone)]
pub enum WoffAttributeDirection {
    LeftToRight,
    RightToLeft,
}

impl Serialize for WoffAttributeDirection {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            WoffAttributeDirection::LeftToRight => serializer.serialize_str(&"ltr"),
            WoffAttributeDirection::RightToLeft => serializer.serialize_str(&"rtl"),
        }
    }
}

struct WoffAttributeDirectionVisitor;

impl<'de> Visitor<'de> for WoffAttributeDirectionVisitor {
    type Value = WoffAttributeDirection;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a string that is either 'ltr' or 'rtl'.")
    }

    fn visit_str<E>(self, s: &str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        match s {
            "ltr" => Ok(WoffAttributeDirection::LeftToRight),
            "rtl" => Ok(WoffAttributeDirection::RightToLeft),
            _ => Err(serde::de::Error::custom("unknown value for the WOFF direction attribute.")),
        }
    }
}

impl<'de> Deserialize<'de> for WoffAttributeDirection {
    fn deserialize<D>(deserializer: D) -> Result<WoffAttributeDirection, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_str(WoffAttributeDirectionVisitor)
    }
}

/// Corresponds to the styleMapStyleName in [Generic Identification Information](http://unifiedfontobject.org/versions/ufo3/fontinfo.plist/#generic-identification-information).
/// If present, is either "regular", "italic", "bold" or "bold italic".
#[derive(Debug, Eq, PartialEq, Clone)]
pub enum StyleMapStyle {
    Regular,
    Italic,
    Bold,
    BoldItalic,
}

impl Serialize for StyleMapStyle {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            StyleMapStyle::Regular => serializer.serialize_str(&"regular"),
            StyleMapStyle::Italic => serializer.serialize_str(&"italic"),
            StyleMapStyle::Bold => serializer.serialize_str(&"bold"),
            StyleMapStyle::BoldItalic => serializer.serialize_str(&"bold italic"),
        }
    }
}

struct StyleMapStyleVisitor;

impl<'de> Visitor<'de> for StyleMapStyleVisitor {
    type Value = StyleMapStyle;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a string that is either 'regular', 'italic', 'bold' or 'bold italic'.")
    }

    fn visit_str<E>(self, s: &str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        match s {
            "regular" => Ok(StyleMapStyle::Regular),
            "italic" => Ok(StyleMapStyle::Italic),
            "bold" => Ok(StyleMapStyle::Bold),
            "bold italic" => Ok(StyleMapStyle::BoldItalic),
            _ => Err(serde::de::Error::custom("unknown value for styleMapStyleName.")),
        }
    }
}

impl<'de> Deserialize<'de> for StyleMapStyle {
    fn deserialize<D>(deserializer: D) -> Result<StyleMapStyle, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_str(StyleMapStyleVisitor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_test::{assert_tokens, Token};

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
        use crate::shared_types::{Color, Identifier, Line};

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
        assert_eq!(
            font_info.open_type_gasp_range_records,
            Some(vec![GaspRangeRecord {
                range_max_ppem: 1,
                range_gasp_behavior: vec![
                    GaspBehavior::Gridfit,
                    GaspBehavior::DoGray,
                    GaspBehavior::SymmetricGridfit,
                    GaspBehavior::SymmetricSmoothing
                ]
            }])
        );
        assert_eq!(
            font_info.guidelines,
            Some(vec![
                Guideline {
                    line: Line::Angle { x: 82.0, y: 720.0, degrees: 90.0 },
                    name: None,
                    color: None,
                    identifier: None
                },
                Guideline {
                    line: Line::Vertical(372.0),
                    name: None,
                    color: None,
                    identifier: None
                },
                Guideline {
                    line: Line::Horizontal(123.0),
                    name: None,
                    color: None,
                    identifier: None
                },
                Guideline {
                    line: Line::Angle { x: 1.0, y: 2.0, degrees: 0.0 },
                    name: Some(" [locked]".to_string()),
                    color: Some(Color { red: 1.0, green: 1.0, blue: 1.0, alpha: 1.0 }),
                    identifier: Some(Identifier("abc".to_string()))
                },
            ])
        );
        assert_eq!(
            font_info.woff_metadata_vendor,
            Some(WoffMetadataVendor {
                name: "a".to_string(),
                url: "b".to_string(),
                dir: Some(WoffAttributeDirection::RightToLeft),
                class: Some("c".to_string()),
            })
        );
    }

    #[test]
    fn test_serde_os2_family_class() {
        let c1 = OS2FamilyClass { class_id: 14, subclass_id: 15 };
        assert_tokens(
            &c1,
            &[Token::Seq { len: Some(2) }, Token::U8(14), Token::U8(15), Token::SeqEnd],
        );
    }

    #[test]
    fn test_serde_os2_panose() {
        let p1 = OS2Panose {
            family_type: 1,
            serif_style: 2,
            weight: 3,
            proportion: 4,
            contrast: 5,
            stroke_variation: 6,
            arm_style: 7,
            letterform: 8,
            midline: 9,
            x_height: 10,
        };
        assert_tokens(
            &p1,
            &[
                Token::Seq { len: Some(10) },
                Token::U32(1),
                Token::U32(2),
                Token::U32(3),
                Token::U32(4),
                Token::U32(5),
                Token::U32(6),
                Token::U32(7),
                Token::U32(8),
                Token::U32(9),
                Token::U32(10),
                Token::SeqEnd,
            ],
        );
    }

    #[test]
    fn test_serde_style_map_style() {
        let s1 = StyleMapStyle::Regular;
        assert_tokens(&s1, &[Token::Str("regular")]);
        let s2 = StyleMapStyle::Italic;
        assert_tokens(&s2, &[Token::Str("italic")]);
        let s3 = StyleMapStyle::Bold;
        assert_tokens(&s3, &[Token::Str("bold")]);
        let s4 = StyleMapStyle::BoldItalic;
        assert_tokens(&s4, &[Token::Str("bold italic")]);
    }

    #[test]
    fn test_validate_units_per_em() {
        let mut fi = FontInfo::default();
        fi.units_per_em = Some(-1.0);
        assert!(fi.validate().is_err());
    }

    #[test]
    fn test_validate_head_created() {
        let mut fi = FontInfo::default();
        fi.open_type_head_created = Some("YYYY/MM/DD HH:MM:SS".to_string());
        assert!(fi.validate().is_err());
        fi.open_type_head_created = Some("1230/03/27 99:23:10".to_string());
        assert!(fi.validate().is_err());
        fi.open_type_head_created = Some("1230:03/27 99:23:10".to_string());
        assert!(fi.validate().is_err());
        fi.open_type_head_created = Some("9999/12/31 23:59:59".to_string());
        assert!(fi.validate().is_ok());
    }

    #[test]
    fn test_validate_gasp() {
        let mut fi = FontInfo::default();
        assert!(fi.validate().is_ok());

        fi.open_type_gasp_range_records = Some(Vec::new());
        assert!(fi.validate().is_ok());

        if let Some(v) = &mut fi.open_type_gasp_range_records {
            v.push(GaspRangeRecord { range_max_ppem: 1, range_gasp_behavior: Vec::new() });
        }
        assert!(fi.validate().is_ok());

        if let Some(v) = &mut fi.open_type_gasp_range_records {
            v.push(GaspRangeRecord { range_max_ppem: 2, range_gasp_behavior: Vec::new() });
        }
        assert!(fi.validate().is_ok());

        if let Some(v) = &mut fi.open_type_gasp_range_records {
            v.push(GaspRangeRecord { range_max_ppem: 1, range_gasp_behavior: Vec::new() });
        }
        assert!(fi.validate().is_err());
    }

    #[test]
    fn test_validate_woff_extensions() {
        let mut fi = FontInfo::default();
        assert!(fi.validate().is_ok());

        fi.woff_metadata_extensions = Some(Vec::new());
        assert!(fi.validate().is_err());

        if let Some(v) = &mut fi.woff_metadata_extensions {
            v.push(WoffMetadataExtensionRecord { id: None, names: Vec::new(), items: Vec::new() });
        }
        assert!(fi.validate().is_err());

        if let Some(v) = &mut fi.woff_metadata_extensions {
            v[0].items.push(WoffMetadataExtensionItemRecord {
                id: Some("a".to_string()),
                names: Vec::new(),
                values: Vec::new(),
            });
        }
        assert!(fi.validate().is_err());

        if let Some(v) = &mut fi.woff_metadata_extensions {
            v[0].items[0].names.push(WoffMetadataExtensionNameRecord {
                text: "a".to_string(),
                language: None,
                dir: None,
                class: None,
            });
            v[0].items[0].values.push(WoffMetadataExtensionValueRecord {
                text: "b".to_string(),
                language: None,
                dir: None,
                class: None,
            });
        }
        assert!(fi.validate().is_ok());
    }
}
