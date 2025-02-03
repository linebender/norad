//! The contents of [`fontinfo.plist`].
//!
//! [`fontinfo.plist`]: https://unifiedfontobject.org/versions/ufo3/fontinfo.plist/

use std::path::Path;
use std::{collections::HashSet, convert::TryFrom, ops::Deref};

use serde::de::Deserializer;
use serde::ser::{SerializeSeq, Serializer};
use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};

use crate::error::{ExpectedPositiveValue, FontInfoErrorKind, FontInfoLoadError};
use crate::shared_types::PUBLIC_OBJECT_LIBS_KEY;
use crate::{FormatVersion, Guideline, Identifier, Plist};

/// A signed integer.
pub type Integer = i32;

/// An unsigned integer.
pub type NonNegativeInteger = u32;

/// A number that may be be an integer or a float.
///
/// This is basically an implementation detail: it is more economical to serialize
/// an integer where possible.
pub type IntegerOrFloat = f64;

/// A non-negative number.
///
/// This is a detail of the spec. If the fractional part of this number is
/// 0, it will be serialized as an integer, else as a float.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NonNegativeIntegerOrFloat(f64);

/// A floating-point number.
pub type Float = f64;

/// a list of ["bit numbers"].
///
/// ["bit numbers"]: https://unifiedfontobject.org/versions/ufo3/fontinfo.plist/#bit-numbers
pub type Bitlist = Vec<u8>;

/// The contents of the [`fontinfo.plist`][] file. This structure is hard-wired to the
/// available attributes in UFO version 3.
///
/// [`fontinfo.plist`]: http://unifiedfontobject.org/versions/ufo3/fontinfo.plist/
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct FontInfo {
    // INFO: Keep this struct sorted alphabetically, serde serializes it in the order you see
    // here and Plist files should be sorted.
    /// Ascender value (ascender).
    #[serde(serialize_with = "serde_impls::ser_opt_int_or_float")]
    pub ascender: Option<f64>,
    /// Cap height value (capHeight).
    #[serde(serialize_with = "serde_impls::ser_opt_int_or_float")]
    pub cap_height: Option<IntegerOrFloat>,
    /// Copyright statement (copyright).
    pub copyright: Option<String>,
    /// Descender value (descender).
    #[serde(serialize_with = "serde_impls::ser_opt_int_or_float")]
    pub descender: Option<IntegerOrFloat>,
    /// Family name (familyName).
    pub family_name: Option<String>,
    /// Guideline definitions that apply to all glyphs in
    /// all layers (guidelines).
    pub guidelines: Option<Vec<Guideline>>,
    /// Italic angle in counter-clockwise degrees (italicAngle).
    #[serde(serialize_with = "serde_impls::ser_opt_int_or_float")]
    pub italic_angle: Option<IntegerOrFloat>,
    /// Family ID number (macintoshFONDFamilyID).
    #[serde(rename = "macintoshFONDFamilyID")]
    pub macintosh_fond_family_id: Option<Integer>,
    /// Font name for the FOND resource (macintoshFONDName).
    #[serde(rename = "macintoshFONDName")]
    pub macintosh_fond_name: Option<String>,
    /// Arbitrary note (note).
    pub note: Option<String>,
    /// A collection of gasp Range Records (openTypeGaspRangeRecords).
    pub open_type_gasp_range_records: Option<Vec<GaspRangeRecord>>,
    /// Creation date (openTypeHeadCreated).
    pub open_type_head_created: Option<String>,
    /// head table flags (openTypeHeadFlags).
    pub open_type_head_flags: Option<Bitlist>,
    /// Smallest readable size in pixels (openTypeHeadLowestRecPPEM).
    #[serde(rename = "openTypeHeadLowestRecPPEM")]
    pub open_type_head_lowest_rec_ppem: Option<NonNegativeInteger>,
    /// Ascender value (openTypeHheaAscender).
    pub open_type_hhea_ascender: Option<Integer>,
    /// Caret offset value (openTypeHheaCaretOffset).
    pub open_type_hhea_caret_offset: Option<Integer>,
    /// Caret slope rise value (openTypeHheaCaretSlopeRise).
    pub open_type_hhea_caret_slope_rise: Option<Integer>,
    /// Caret slope run value (openTypeHheaCaretSlopeRun)
    pub open_type_hhea_caret_slope_run: Option<Integer>,
    /// Descender value (openTypeHheaDescender).
    pub open_type_hhea_descender: Option<Integer>,
    /// Line gap value (openTypeHheaLineGap).
    pub open_type_hhea_line_gap: Option<Integer>,
    /// Compatible full name (openTypeNameCompatibleFullName).
    pub open_type_name_compatible_full_name: Option<String>,
    /// Description of the font (openTypeNameDescription).
    pub open_type_name_description: Option<String>,
    /// URL for the designer (openTypeNameDesignerURL).
    #[serde(rename = "openTypeNameDesignerURL")]
    pub open_type_name_designer_url: Option<String>,
    /// Designer name (openTypeNameDesigner).
    pub open_type_name_designer: Option<String>,
    /// License text (openTypeNameLicense).
    pub open_type_name_license: Option<String>,
    /// License URL (openTypeNameLicenseURL).
    #[serde(rename = "openTypeNameLicenseURL")]
    pub open_type_name_license_url: Option<String>,
    /// Manufacturer name (openTypeNameManufacturer).
    pub open_type_name_manufacturer: Option<String>,
    /// Manufacturer URL (openTypeNameManufacturerURL).
    #[serde(rename = "openTypeNameManufacturerURL")]
    pub open_type_name_manufacturer_url: Option<String>,
    /// Preferred family name (openTypeNamePreferredFamilyName).
    pub open_type_name_preferred_family_name: Option<String>,
    /// Preferred sub-family name (openTypeNamePreferredSubfamilyName).
    pub open_type_name_preferred_subfamily_name: Option<String>,
    /// A collection of name records (openTypeNameRecords).
    pub open_type_name_records: Option<Vec<NameRecord>>,
    /// Sample text (openTypeNameSampleText).
    pub open_type_name_sample_text: Option<String>,
    /// Unique ID string (openTypeNameUniqueID).
    #[serde(rename = "openTypeNameUniqueID")]
    pub open_type_name_unique_id: Option<String>,
    /// Version string (openTypeNameVersion).
    pub open_type_name_version: Option<String>,
    /// WWS family name (openTypeNameWWSFamilyName).
    #[serde(rename = "openTypeNameWWSFamilyName")]
    pub open_type_name_wws_family_name: Option<String>,
    /// WWS sub-family name (openTypeNameWWSSubfamilyName).
    #[serde(rename = "openTypeNameWWSSubfamilyName")]
    pub open_type_name_wws_subfamily_name: Option<String>,
    /// Bit flags that represent code page ranges present in the font
    /// (openTypeOS2CodePageRanges).
    #[serde(rename = "openTypeOS2CodePageRanges")]
    pub open_type_os2_code_page_ranges: Option<Bitlist>,
    /// Font class and sub-class (openTypeOS2FamilyClass).
    #[serde(rename = "openTypeOS2FamilyClass")]
    pub open_type_os2_family_class: Option<Os2FamilyClass>,
    /// Panose specification settings (openTypeOS2Panose).
    #[serde(rename = "openTypeOS2Panose")]
    pub open_type_os2_panose: Option<Os2Panose>,
    /// fsSelection bit settings (openTypeOS2Selection).
    #[serde(rename = "openTypeOS2Selection")]
    pub open_type_os2_selection: Option<Bitlist>,
    /// Strikeout position (openTypeOS2StrikeoutPosition).
    #[serde(rename = "openTypeOS2StrikeoutPosition")]
    pub open_type_os2_strikeout_position: Option<Integer>,
    /// Strikeout size (openTypeOS2StrikeoutSize).
    #[serde(rename = "openTypeOS2StrikeoutSize")]
    pub open_type_os2_strikeout_size: Option<Integer>,
    /// Subscript x offset (openTypeOS2SubscriptXOffset).
    #[serde(rename = "openTypeOS2SubscriptXOffset")]
    pub open_type_os2_subscript_x_offset: Option<Integer>,
    /// Subscript horizontal size (openTypeOS2SubscriptXSize).
    #[serde(rename = "openTypeOS2SubscriptXSize")]
    pub open_type_os2_subscript_x_size: Option<Integer>,
    /// Subscript y offset (openTypeOS2SubscriptYOffset).
    #[serde(rename = "openTypeOS2SubscriptYOffset")]
    pub open_type_os2_subscript_y_offset: Option<Integer>,
    /// Subscript vertical size (openTypeOS2SubscriptYSize).
    #[serde(rename = "openTypeOS2SubscriptYSize")]
    pub open_type_os2_subscript_y_size: Option<Integer>,
    /// Superscript x offset (openTypeOS2SuperscriptXOffset).
    #[serde(rename = "openTypeOS2SuperscriptXOffset")]
    pub open_type_os2_superscript_x_offset: Option<Integer>,
    /// Superscript horizontal size (openTypeOS2SuperscriptXSize).
    #[serde(rename = "openTypeOS2SuperscriptXSize")]
    pub open_type_os2_superscript_x_size: Option<Integer>,
    /// Superscript y offset (openTypeOS2SuperscriptYOffset).
    #[serde(rename = "openTypeOS2SuperscriptYOffset")]
    pub open_type_os2_superscript_y_offset: Option<Integer>,
    /// Superscript vertical size (openTypeOS2SuperscriptYSize).
    #[serde(rename = "openTypeOS2SuperscriptYSize")]
    pub open_type_os2_superscript_y_size: Option<Integer>,
    /// Bit flags indicating the embedding type (openTypeOS2Type).
    #[serde(rename = "openTypeOS2Type")]
    pub open_type_os2_type: Option<Bitlist>,
    /// Ascender value (openTypeOS2TypoAscender).
    #[serde(rename = "openTypeOS2TypoAscender")]
    pub open_type_os2_typo_ascender: Option<Integer>,
    /// Descender value (openTypeOS2TypoDescender).
    #[serde(rename = "openTypeOS2TypoDescender")]
    pub open_type_os2_typo_descender: Option<Integer>,
    /// Line gap value (openTypeOS2TypoLineGap).
    #[serde(rename = "openTypeOS2TypoLineGap")]
    pub open_type_os2_typo_line_gap: Option<Integer>,
    /// Bit flags that represent Unicode ranges present in
    /// the font (openTypeOS2UnicodeRanges).
    #[serde(rename = "openTypeOS2UnicodeRanges")]
    pub open_type_os2_unicode_ranges: Option<Bitlist>,
    /// Four character vendor ID (openTypeOS2VendorID).
    #[serde(rename = "openTypeOS2VendorID")]
    pub open_type_os2_vendor_id: Option<String>,
    /// OS/2 weight class (openTypeOS2WeightClass).
    #[serde(rename = "openTypeOS2WeightClass")]
    pub open_type_os2_weight_class: Option<NonNegativeInteger>,
    /// OS/2 width class (openTypeOS2WidthClass).
    #[serde(rename = "openTypeOS2WidthClass")]
    pub open_type_os2_width_class: Option<Os2WidthClass>,
    /// Ascender value (openTypeOS2WinAscent).
    #[serde(rename = "openTypeOS2WinAscent")]
    pub open_type_os2_win_ascent: Option<NonNegativeInteger>,
    /// Descender value (openTypeOS2WinDescent).
    #[serde(rename = "openTypeOS2WinDescent")]
    pub open_type_os2_win_descent: Option<NonNegativeInteger>,
    /// Caret offset value (openTypeVheaCaretOffset).
    pub open_type_vhea_caret_offset: Option<Integer>,
    /// Caret slope rise value (openTypeVheaCaretSlopeRise).
    pub open_type_vhea_caret_slope_rise: Option<Integer>,
    /// Caret slope run value (openTypeVheaCaretSlopeRun).
    pub open_type_vhea_caret_slope_run: Option<Integer>,
    /// Ascender value (openTypeVheaVertTypoAscender).
    pub open_type_vhea_vert_typo_ascender: Option<Integer>,
    /// Descender value (openTypeVheaVertTypoDescender).
    pub open_type_vhea_vert_typo_descender: Option<Integer>,
    /// Line gap value (openTypeVheaVertTypoLineGap).
    pub open_type_vhea_vert_typo_line_gap: Option<Integer>,
    /// Postscript BlueFuzz value (postscriptBlueFuzz).
    #[serde(serialize_with = "serde_impls::ser_opt_int_or_float")]
    pub postscript_blue_fuzz: Option<IntegerOrFloat>,
    /// Postscript BlueScale value (postscriptBlueScale).
    pub postscript_blue_scale: Option<Float>,
    /// Postscript BlueShift value (postscriptBlueShift).
    #[serde(serialize_with = "serde_impls::ser_opt_int_or_float")]
    pub postscript_blue_shift: Option<IntegerOrFloat>,
    /// A collection of values that should be in the
    /// Type 1/CFF BlueValues field (postscriptBlueValues).
    #[serde(serialize_with = "serde_impls::ser_opt_vec_int_or_float")]
    pub postscript_blue_values: Option<Vec<IntegerOrFloat>>,
    /// Name of default glyph in PFM files (postscriptDefaultCharacter).
    pub postscript_default_character: Option<String>,
    /// Default glyph width (postscriptDefaultWidthX).
    #[serde(serialize_with = "serde_impls::ser_opt_int_or_float")]
    pub postscript_default_width_x: Option<IntegerOrFloat>,
    /// A collection of values that should be in the Type 1/CFF
    /// FamilyBlues field (postscriptFamilyBlues).
    #[serde(serialize_with = "serde_impls::ser_opt_vec_int_or_float")]
    pub postscript_family_blues: Option<Vec<IntegerOrFloat>>,
    /// A collection of values that should be in the Type 1/CFF
    /// FamilyOtherBlues field (postscriptFamilyOtherBlues).
    #[serde(serialize_with = "serde_impls::ser_opt_vec_int_or_float")]
    pub postscript_family_other_blues: Option<Vec<IntegerOrFloat>>,
    /// Type 1/CFF table FontName field (postscriptFontName).
    pub postscript_font_name: Option<String>,
    /// Boolean value that indicates how Type 1/CFF ForceBold should
    /// be set (postscriptForceBold).
    pub postscript_force_bold: Option<bool>,
    /// Type 1/CFF table FullName field (postscriptFullName).
    pub postscript_full_name: Option<String>,
    /// Boolean that indicates if a font is monospaced
    /// (postscriptIsFixedPitch).
    pub postscript_is_fixed_pitch: Option<bool>,
    /// Glyph nominal width (postscriptNominalWidthX).
    #[serde(serialize_with = "serde_impls::ser_opt_int_or_float")]
    pub postscript_nominal_width_x: Option<IntegerOrFloat>,
    /// A collection of values that should be in the Type 1/CFF
    /// OtherBlues field (postscriptOtherBlues).
    #[serde(serialize_with = "serde_impls::ser_opt_vec_int_or_float")]
    pub postscript_other_blues: Option<Vec<IntegerOrFloat>>,
    /// Slant angle in counter-clockwise degrees from the vertical
    /// (postscriptSlantAngle).
    #[serde(serialize_with = "serde_impls::ser_opt_int_or_float")]
    pub postscript_slant_angle: Option<IntegerOrFloat>,
    /// A collection of horizontal stems sorted in the order specified
    /// in the Type 1/CFF specification (postscriptStemSnapH).
    #[serde(serialize_with = "serde_impls::ser_opt_vec_int_or_float")]
    pub postscript_stem_snap_h: Option<Vec<IntegerOrFloat>>,
    /// A collection of vertical stems sorted in the order specified
    /// in the Type 1/CFF specification (postscriptStemSnapV).
    #[serde(serialize_with = "serde_impls::ser_opt_vec_int_or_float")]
    pub postscript_stem_snap_v: Option<Vec<IntegerOrFloat>>,
    /// Underline position value (postscriptUnderlinePosition).
    #[serde(serialize_with = "serde_impls::ser_opt_int_or_float")]
    pub postscript_underline_position: Option<IntegerOrFloat>,
    /// Underline thickness value (postscriptUnderlineThickness).
    #[serde(serialize_with = "serde_impls::ser_opt_int_or_float")]
    pub postscript_underline_thickness: Option<IntegerOrFloat>,
    /// Unique ID as specified by the Type 1/CFF specification
    /// (postscriptUniqueID).
    #[serde(rename = "postscriptUniqueID")]
    pub postscript_unique_id: Option<Integer>,
    /// Overall font weight (postscriptWeightName).
    pub postscript_weight_name: Option<String>,
    /// Windows character set (postscriptWindowsCharacterSet).
    pub postscript_windows_character_set: Option<PostscriptWindowsCharacterSet>,
    /// Family name in bold, italic, bold italic style mapping
    /// (styleMapFamilyName).
    pub style_map_family_name: Option<String>,
    /// Style map style (styleMapStyleName).
    pub style_map_style_name: Option<StyleMapStyle>,
    /// Style name (styleName).
    pub style_name: Option<String>,
    /// Trademark statement (trademark).
    pub trademark: Option<String>,
    /// Units per em (unitsPerEm).
    pub units_per_em: Option<NonNegativeIntegerOrFloat>,
    /// Major version number (versionMajor).
    pub version_major: Option<Integer>,
    /// Minor version number (versionMinor).
    pub version_minor: Option<NonNegativeInteger>,
    /// Major version number (woffMajorVersion).
    pub woff_major_version: Option<NonNegativeInteger>,
    /// Font copyright (woffMetadataCopyright).
    pub woff_metadata_copyright: Option<WoffMetadataCopyright>,
    /// Font credits (woffMetadataCredits).
    pub woff_metadata_credits: Option<WoffMetadataCredits>,
    /// Font description (woffMetadataDescription).
    pub woff_metadata_description: Option<WoffMetadataDescription>,
    /// A collection of metadata extension records (woffMetadataExtensions).
    pub woff_metadata_extensions: Option<Vec<WoffMetadataExtensionRecord>>,
    /// Font license (woffMetadataLicense).
    pub woff_metadata_license: Option<WoffMetadataLicense>,
    /// Font licensee (woffMetadataLicensee).
    pub woff_metadata_licensee: Option<WoffMetadataLicensee>,
    /// Font trademark (woffMetadataTrademark).
    pub woff_metadata_trademark: Option<WoffMetadataTrademark>,
    /// Font unique ID (woffMetadataUniqueID).
    #[serde(rename = "woffMetadataUniqueID")]
    pub woff_metadata_unique_id: Option<WoffMetadataUniqueId>,
    /// Font vendor (woffMetadataVendor).
    pub woff_metadata_vendor: Option<WoffMetadataVendor>,
    /// Minor version number (woffMinorVersion).
    pub woff_minor_version: Option<NonNegativeInteger>,
    /// x-height value (xHeight).
    #[serde(serialize_with = "serde_impls::ser_opt_int_or_float")]
    pub x_height: Option<IntegerOrFloat>,
    /// Year that the font was created (year).
    pub year: Option<Integer>,
}

/// The contents of the [`fontinfo.plist`][] file specified for UFO version 2. Its only purpose is
/// to enable upconversion.
///
/// [`fontinfo.plist`]: http://unifiedfontobject.org/versions/ufo2/fontinfo.plist/
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
#[allow(non_snake_case)]
struct FontInfoV2 {
    ascender: Option<IntegerOrFloat>,
    capHeight: Option<IntegerOrFloat>,
    copyright: Option<String>,
    descender: Option<IntegerOrFloat>,
    familyName: Option<String>,
    italicAngle: Option<IntegerOrFloat>,
    macintoshFONDFamilyID: Option<Integer>,
    macintoshFONDName: Option<String>,
    note: Option<String>,
    openTypeHeadCreated: Option<String>,
    openTypeHeadFlags: Option<Bitlist>,
    openTypeHeadLowestRecPPEM: Option<IntegerOrFloat>,
    openTypeHheaAscender: Option<IntegerOrFloat>,
    openTypeHheaCaretOffset: Option<IntegerOrFloat>,
    openTypeHheaCaretSlopeRise: Option<Integer>,
    openTypeHheaCaretSlopeRun: Option<Integer>,
    openTypeHheaDescender: Option<IntegerOrFloat>,
    openTypeHheaLineGap: Option<IntegerOrFloat>,
    openTypeNameCompatibleFullName: Option<String>,
    openTypeNameDescription: Option<String>,
    openTypeNameDesigner: Option<String>,
    openTypeNameDesignerURL: Option<String>,
    openTypeNameLicense: Option<String>,
    openTypeNameLicenseURL: Option<String>,
    openTypeNameManufacturer: Option<String>,
    openTypeNameManufacturerURL: Option<String>,
    openTypeNamePreferredFamilyName: Option<String>,
    openTypeNamePreferredSubfamilyName: Option<String>,
    openTypeNameSampleText: Option<String>,
    openTypeNameUniqueID: Option<String>,
    openTypeNameVersion: Option<String>,
    openTypeNameWWSFamilyName: Option<String>,
    openTypeNameWWSSubfamilyName: Option<String>,
    openTypeOS2CodePageRanges: Option<Bitlist>,
    openTypeOS2FamilyClass: Option<Os2FamilyClass>,
    openTypeOS2Panose: Option<Os2PanoseV2>,
    openTypeOS2Selection: Option<Bitlist>,
    openTypeOS2StrikeoutPosition: Option<IntegerOrFloat>,
    openTypeOS2StrikeoutSize: Option<IntegerOrFloat>,
    openTypeOS2SubscriptXOffset: Option<IntegerOrFloat>,
    openTypeOS2SubscriptXSize: Option<IntegerOrFloat>,
    openTypeOS2SubscriptYOffset: Option<IntegerOrFloat>,
    openTypeOS2SubscriptYSize: Option<IntegerOrFloat>,
    openTypeOS2SuperscriptXOffset: Option<IntegerOrFloat>,
    openTypeOS2SuperscriptXSize: Option<IntegerOrFloat>,
    openTypeOS2SuperscriptYOffset: Option<IntegerOrFloat>,
    openTypeOS2SuperscriptYSize: Option<IntegerOrFloat>,
    openTypeOS2Type: Option<Bitlist>,
    openTypeOS2TypoAscender: Option<IntegerOrFloat>,
    openTypeOS2TypoDescender: Option<IntegerOrFloat>,
    openTypeOS2TypoLineGap: Option<IntegerOrFloat>,
    openTypeOS2UnicodeRanges: Option<Bitlist>,
    openTypeOS2VendorID: Option<String>,
    openTypeOS2WeightClass: Option<NonNegativeInteger>,
    openTypeOS2WidthClass: Option<Os2WidthClass>,
    openTypeOS2WinAscent: Option<IntegerOrFloat>,
    openTypeOS2WinDescent: Option<IntegerOrFloat>,
    openTypeVheaCaretOffset: Option<IntegerOrFloat>,
    openTypeVheaCaretSlopeRise: Option<Integer>,
    openTypeVheaCaretSlopeRun: Option<Integer>,
    openTypeVheaVertTypoAscender: Option<IntegerOrFloat>,
    openTypeVheaVertTypoDescender: Option<IntegerOrFloat>,
    openTypeVheaVertTypoLineGap: Option<IntegerOrFloat>,
    postscriptBlueFuzz: Option<IntegerOrFloat>,
    postscriptBlueScale: Option<Float>,
    postscriptBlueShift: Option<IntegerOrFloat>,
    postscriptBlueValues: Option<Vec<IntegerOrFloat>>,
    postscriptDefaultCharacter: Option<String>,
    postscriptDefaultWidthX: Option<IntegerOrFloat>,
    postscriptFamilyBlues: Option<Vec<IntegerOrFloat>>,
    postscriptFamilyOtherBlues: Option<Vec<IntegerOrFloat>>,
    postscriptFontName: Option<String>,
    postscriptForceBold: Option<bool>,
    postscriptFullName: Option<String>,
    postscriptIsFixedPitch: Option<bool>,
    postscriptNominalWidthX: Option<IntegerOrFloat>,
    postscriptOtherBlues: Option<Vec<IntegerOrFloat>>,
    postscriptSlantAngle: Option<IntegerOrFloat>,
    postscriptStemSnapH: Option<Vec<IntegerOrFloat>>,
    postscriptStemSnapV: Option<Vec<IntegerOrFloat>>,
    postscriptUnderlinePosition: Option<IntegerOrFloat>,
    postscriptUnderlineThickness: Option<IntegerOrFloat>,
    postscriptUniqueID: Option<Integer>,
    postscriptWeightName: Option<String>,
    postscriptWindowsCharacterSet: Option<PostscriptWindowsCharacterSet>,
    styleMapFamilyName: Option<String>,
    styleMapStyleName: Option<StyleMapStyle>,
    styleName: Option<String>,
    trademark: Option<String>,
    unitsPerEm: Option<IntegerOrFloat>,
    versionMajor: Option<Integer>,
    versionMinor: Option<Integer>,
    xHeight: Option<IntegerOrFloat>,
    year: Option<Integer>,
}

/// The contents of the [`fontinfo.plist`][] file specified for UFO version 1. Its only purpose is
/// to enable upconversion.
///
/// [`fontinfo.plist`]: http://unifiedfontobject.org/versions/ufo1/fontinfo.plist/
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
#[allow(non_snake_case)]
struct FontInfoV1 {
    ascender: Option<IntegerOrFloat>,
    capHeight: Option<IntegerOrFloat>,
    copyright: Option<String>,
    createdBy: Option<String>,
    defaultWidth: Option<IntegerOrFloat>,
    descender: Option<IntegerOrFloat>,
    designer: Option<String>,
    designerURL: Option<String>,
    familyName: Option<String>,
    fondID: Option<Integer>,
    fondName: Option<String>,
    fontName: Option<String>,
    fontStyle: Option<Integer>,
    fullName: Option<String>,
    italicAngle: Option<IntegerOrFloat>,
    license: Option<String>,
    licenseURL: Option<String>,
    menuName: Option<String>,
    msCharSet: Option<Integer>,
    note: Option<String>,
    notice: Option<String>,
    otFamilyName: Option<String>,
    otMacName: Option<String>,
    otStyleName: Option<String>,
    slantAngle: Option<IntegerOrFloat>,
    styleName: Option<String>,
    trademark: Option<String>,
    ttUniqueID: Option<String>,
    ttVendor: Option<String>,
    ttVersion: Option<String>,
    uniqueID: Option<Integer>,
    unitsPerEm: Option<IntegerOrFloat>,
    vendorURL: Option<String>,
    versionMajor: Option<Integer>,
    versionMinor: Option<Integer>,
    weightName: Option<String>,
    weightValue: Option<Integer>,
    widthName: Option<String>,
    xHeight: Option<IntegerOrFloat>, // Does not appear in spec but ufoLib.
    year: Option<Integer>,           // Does not appear in spec but ufoLib.
}

impl FontInfo {
    /// Returns [`FontInfo`] from a file, upgrading from the supplied `format_version` to the highest
    /// internally supported version.
    ///
    /// The conversion follows what ufoLib and defcon are doing, e.g. various fields that were
    /// implicitly signed integers before and are unsigned integers in the newest spec, are
    /// converted by taking their absolute value. Fields that could be floats before and are
    /// integers now are rounded. Fields that could be floats before and are unsigned integers
    /// now are rounded before taking their absolute value.
    pub(crate) fn from_file<P: AsRef<Path>>(
        path: P,
        format_version: FormatVersion,
        lib: &mut Plist,
    ) -> Result<Self, FontInfoLoadError> {
        let path = path.as_ref();
        match format_version {
            FormatVersion::V3 => {
                let mut fontinfo: FontInfo =
                    plist::from_file(path).map_err(FontInfoLoadError::ParsePlist)?;
                fontinfo.validate().map_err(FontInfoLoadError::InvalidData)?;
                fontinfo.load_object_libs(lib)?;
                Ok(fontinfo)
            }
            FormatVersion::V2 => {
                let fontinfo_v2: FontInfoV2 =
                    plist::from_file(path).map_err(FontInfoLoadError::ParsePlist)?;
                let fontinfo = FontInfo {
                    ascender: fontinfo_v2.ascender,
                    cap_height: fontinfo_v2.capHeight,
                    copyright: fontinfo_v2.copyright,
                    descender: fontinfo_v2.descender,
                    family_name: fontinfo_v2.familyName,
                    italic_angle: fontinfo_v2.italicAngle,
                    macintosh_fond_family_id: fontinfo_v2.macintoshFONDFamilyID,
                    macintosh_fond_name: fontinfo_v2.macintoshFONDName,
                    note: fontinfo_v2.note,
                    open_type_head_created: fontinfo_v2.openTypeHeadCreated,
                    open_type_head_flags: fontinfo_v2.openTypeHeadFlags,
                    open_type_head_lowest_rec_ppem: fontinfo_v2
                        .openTypeHeadLowestRecPPEM
                        .map(|v| v.round().abs() as NonNegativeInteger),
                    open_type_hhea_ascender: fontinfo_v2
                        .openTypeHheaAscender
                        .map(|v| v.round() as Integer),
                    open_type_hhea_caret_offset: fontinfo_v2
                        .openTypeHheaCaretOffset
                        .map(|v| v.round() as Integer),
                    open_type_hhea_caret_slope_rise: fontinfo_v2.openTypeHheaCaretSlopeRise,
                    open_type_hhea_caret_slope_run: fontinfo_v2.openTypeHheaCaretSlopeRun,
                    open_type_hhea_descender: fontinfo_v2
                        .openTypeHheaDescender
                        .map(|v| v.round() as Integer),
                    open_type_hhea_line_gap: fontinfo_v2
                        .openTypeHheaLineGap
                        .map(|v| v.round() as Integer),
                    open_type_name_compatible_full_name: fontinfo_v2.openTypeNameCompatibleFullName,
                    open_type_name_description: fontinfo_v2.openTypeNameDescription,
                    open_type_name_designer: fontinfo_v2.openTypeNameDesigner,
                    open_type_name_designer_url: fontinfo_v2.openTypeNameDesignerURL,
                    open_type_name_license: fontinfo_v2.openTypeNameLicense,
                    open_type_name_license_url: fontinfo_v2.openTypeNameLicenseURL,
                    open_type_name_manufacturer: fontinfo_v2.openTypeNameManufacturer,
                    open_type_name_manufacturer_url: fontinfo_v2.openTypeNameManufacturerURL,
                    open_type_name_preferred_family_name: fontinfo_v2
                        .openTypeNamePreferredFamilyName,
                    open_type_name_preferred_subfamily_name: fontinfo_v2
                        .openTypeNamePreferredSubfamilyName,
                    open_type_name_sample_text: fontinfo_v2.openTypeNameSampleText,
                    open_type_name_unique_id: fontinfo_v2.openTypeNameUniqueID,
                    open_type_name_version: fontinfo_v2.openTypeNameVersion,
                    open_type_name_wws_family_name: fontinfo_v2.openTypeNameWWSFamilyName,
                    open_type_name_wws_subfamily_name: fontinfo_v2.openTypeNameWWSSubfamilyName,
                    open_type_os2_code_page_ranges: fontinfo_v2.openTypeOS2CodePageRanges,
                    open_type_os2_family_class: fontinfo_v2.openTypeOS2FamilyClass,
                    open_type_os2_panose: fontinfo_v2.openTypeOS2Panose.map(Os2Panose::from),
                    open_type_os2_selection: fontinfo_v2.openTypeOS2Selection,
                    open_type_os2_strikeout_position: fontinfo_v2
                        .openTypeOS2StrikeoutPosition
                        .map(|v| v.round() as Integer),
                    open_type_os2_strikeout_size: fontinfo_v2
                        .openTypeOS2StrikeoutSize
                        .map(|v| v.round() as Integer),
                    open_type_os2_subscript_x_offset: fontinfo_v2
                        .openTypeOS2SubscriptXOffset
                        .map(|v| v.round() as Integer),
                    open_type_os2_subscript_x_size: fontinfo_v2
                        .openTypeOS2SubscriptXSize
                        .map(|v| v.round() as Integer),
                    open_type_os2_subscript_y_offset: fontinfo_v2
                        .openTypeOS2SubscriptYOffset
                        .map(|v| v.round() as Integer),
                    open_type_os2_subscript_y_size: fontinfo_v2
                        .openTypeOS2SubscriptYSize
                        .map(|v| v.round() as Integer),
                    open_type_os2_superscript_x_offset: fontinfo_v2
                        .openTypeOS2SuperscriptXOffset
                        .map(|v| v.round() as Integer),
                    open_type_os2_superscript_x_size: fontinfo_v2
                        .openTypeOS2SuperscriptXSize
                        .map(|v| v.round() as Integer),
                    open_type_os2_superscript_y_offset: fontinfo_v2
                        .openTypeOS2SuperscriptYOffset
                        .map(|v| v.round() as Integer),
                    open_type_os2_superscript_y_size: fontinfo_v2
                        .openTypeOS2SuperscriptYSize
                        .map(|v| v.round() as Integer),
                    open_type_os2_type: fontinfo_v2.openTypeOS2Type,
                    open_type_os2_typo_ascender: fontinfo_v2
                        .openTypeOS2TypoAscender
                        .map(|v| v.round() as Integer),
                    open_type_os2_typo_descender: fontinfo_v2
                        .openTypeOS2TypoDescender
                        .map(|v| v.round() as Integer),
                    open_type_os2_typo_line_gap: fontinfo_v2
                        .openTypeOS2TypoLineGap
                        .map(|v| v.round() as Integer),
                    open_type_os2_unicode_ranges: fontinfo_v2.openTypeOS2UnicodeRanges,
                    open_type_os2_vendor_id: fontinfo_v2.openTypeOS2VendorID,
                    open_type_os2_weight_class: fontinfo_v2.openTypeOS2WeightClass,
                    open_type_os2_width_class: fontinfo_v2.openTypeOS2WidthClass,
                    open_type_os2_win_ascent: fontinfo_v2
                        .openTypeOS2WinAscent
                        .map(|v| v.round().abs() as NonNegativeInteger),
                    open_type_os2_win_descent: fontinfo_v2
                        .openTypeOS2WinDescent
                        .map(|v| v.round().abs() as NonNegativeInteger),
                    open_type_vhea_caret_offset: fontinfo_v2
                        .openTypeVheaCaretOffset
                        .map(|v| v.round() as Integer),
                    open_type_vhea_caret_slope_rise: fontinfo_v2.openTypeVheaCaretSlopeRise,
                    open_type_vhea_caret_slope_run: fontinfo_v2.openTypeVheaCaretSlopeRun,
                    open_type_vhea_vert_typo_ascender: fontinfo_v2
                        .openTypeVheaVertTypoAscender
                        .map(|v| v.round() as Integer),
                    open_type_vhea_vert_typo_descender: fontinfo_v2
                        .openTypeVheaVertTypoDescender
                        .map(|v| v.round() as Integer),
                    open_type_vhea_vert_typo_line_gap: fontinfo_v2
                        .openTypeVheaVertTypoLineGap
                        .map(|v| v.round() as Integer),
                    postscript_blue_fuzz: fontinfo_v2.postscriptBlueFuzz,
                    postscript_blue_scale: fontinfo_v2.postscriptBlueScale,
                    postscript_blue_shift: fontinfo_v2.postscriptBlueShift,
                    postscript_blue_values: fontinfo_v2.postscriptBlueValues,
                    postscript_default_character: fontinfo_v2.postscriptDefaultCharacter,
                    postscript_default_width_x: fontinfo_v2.postscriptDefaultWidthX,
                    postscript_family_blues: fontinfo_v2.postscriptFamilyBlues,
                    postscript_family_other_blues: fontinfo_v2.postscriptFamilyOtherBlues,
                    postscript_font_name: fontinfo_v2.postscriptFontName,
                    postscript_force_bold: fontinfo_v2.postscriptForceBold,
                    postscript_full_name: fontinfo_v2.postscriptFullName,
                    postscript_is_fixed_pitch: fontinfo_v2.postscriptIsFixedPitch,
                    postscript_nominal_width_x: fontinfo_v2.postscriptNominalWidthX,
                    postscript_other_blues: fontinfo_v2.postscriptOtherBlues,
                    postscript_slant_angle: fontinfo_v2.postscriptSlantAngle,
                    postscript_stem_snap_h: fontinfo_v2.postscriptStemSnapH,
                    postscript_stem_snap_v: fontinfo_v2.postscriptStemSnapV,
                    postscript_underline_position: fontinfo_v2.postscriptUnderlinePosition,
                    postscript_underline_thickness: fontinfo_v2.postscriptUnderlineThickness,
                    postscript_unique_id: fontinfo_v2.postscriptUniqueID,
                    postscript_weight_name: fontinfo_v2.postscriptWeightName,
                    postscript_windows_character_set: fontinfo_v2.postscriptWindowsCharacterSet,
                    style_map_family_name: fontinfo_v2.styleMapFamilyName,
                    style_map_style_name: fontinfo_v2.styleMapStyleName,
                    style_name: fontinfo_v2.styleName,
                    trademark: fontinfo_v2.trademark,
                    units_per_em: fontinfo_v2
                        .unitsPerEm
                        .map(|v| NonNegativeIntegerOrFloat::new(v.abs()).unwrap()),
                    version_major: fontinfo_v2.versionMajor,
                    version_minor: fontinfo_v2.versionMinor.map(|v| v.unsigned_abs()),
                    x_height: fontinfo_v2.xHeight,
                    year: fontinfo_v2.year,
                    ..FontInfo::default()
                };
                fontinfo.validate().map_err(FontInfoLoadError::FontInfoUpconversion)?;
                Ok(fontinfo)
            }
            FormatVersion::V1 => {
                let fontinfo_v1: FontInfoV1 =
                    plist::from_file(path).map_err(FontInfoLoadError::ParsePlist)?;
                let fontinfo = FontInfo {
                    ascender: fontinfo_v1.ascender,
                    cap_height: fontinfo_v1.capHeight,
                    copyright: fontinfo_v1.copyright,
                    descender: fontinfo_v1.descender,
                    family_name: fontinfo_v1.familyName,
                    italic_angle: fontinfo_v1.italicAngle,
                    macintosh_fond_family_id: fontinfo_v1.fondID,
                    macintosh_fond_name: fontinfo_v1.fondName,
                    note: fontinfo_v1.note,
                    open_type_name_compatible_full_name: fontinfo_v1.otMacName,
                    open_type_name_description: fontinfo_v1.notice,
                    open_type_name_designer_url: fontinfo_v1.designerURL,
                    open_type_name_designer: fontinfo_v1.designer,
                    open_type_name_license_url: fontinfo_v1.licenseURL,
                    open_type_name_license: fontinfo_v1.license,
                    open_type_name_manufacturer_url: fontinfo_v1.vendorURL,
                    open_type_name_manufacturer: fontinfo_v1.createdBy,
                    open_type_name_preferred_family_name: fontinfo_v1.otFamilyName,
                    open_type_name_preferred_subfamily_name: fontinfo_v1.otStyleName,
                    open_type_name_unique_id: fontinfo_v1.ttUniqueID,
                    open_type_name_version: fontinfo_v1.ttVersion,
                    open_type_os2_vendor_id: fontinfo_v1.ttVendor,
                    open_type_os2_weight_class: match fontinfo_v1.weightValue {
                        Some(v) => match v {
                            -1 => None,
                            _ => Some(v.unsigned_abs()),
                        },
                        None => None,
                    },
                    open_type_os2_width_class: match fontinfo_v1.widthName {
                        Some(v) => match v.as_ref() {
                            "Ultra-condensed" => Some(Os2WidthClass::UltraCondensed),
                            "Extra-condensed" => Some(Os2WidthClass::ExtraCondensed),
                            "Condensed" => Some(Os2WidthClass::Condensed),
                            "Semi-condensed" => Some(Os2WidthClass::SemiCondensed),
                            "Medium (normal)" => Some(Os2WidthClass::Normal),
                            "Normal" => Some(Os2WidthClass::Normal),
                            "All" => Some(Os2WidthClass::Normal),
                            "medium" => Some(Os2WidthClass::Normal),
                            "Medium" => Some(Os2WidthClass::Normal),
                            "Semi-expanded" => Some(Os2WidthClass::SemiExpanded),
                            "Expanded" => Some(Os2WidthClass::Expanded),
                            "Extra-expanded" => Some(Os2WidthClass::ExtraExpanded),
                            "Ultra-expanded" => Some(Os2WidthClass::UltraExpanded),
                            _ => {
                                return Err(FontInfoLoadError::FontInfoUpconversion(
                                    FontInfoErrorKind::UnknownWidthClass(v.clone()),
                                ))
                            }
                        },
                        None => None,
                    },
                    postscript_default_width_x: fontinfo_v1.defaultWidth,
                    postscript_font_name: fontinfo_v1.fontName,
                    postscript_full_name: fontinfo_v1.fullName,
                    postscript_slant_angle: fontinfo_v1.slantAngle,
                    postscript_unique_id: fontinfo_v1.uniqueID,
                    postscript_weight_name: fontinfo_v1.weightName,
                    postscript_windows_character_set: match fontinfo_v1.msCharSet {
                        Some(v) => match v {
                            0 => Some(PostscriptWindowsCharacterSet::Ansi),
                            1 => Some(PostscriptWindowsCharacterSet::Default),
                            2 => Some(PostscriptWindowsCharacterSet::Symbol),
                            77 => Some(PostscriptWindowsCharacterSet::Macintosh),
                            128 => Some(PostscriptWindowsCharacterSet::ShiftJis),
                            129 => Some(PostscriptWindowsCharacterSet::Hangul),
                            130 => Some(PostscriptWindowsCharacterSet::HangulJohab),
                            134 => Some(PostscriptWindowsCharacterSet::Gb2312),
                            136 => Some(PostscriptWindowsCharacterSet::ChineseBig5),
                            161 => Some(PostscriptWindowsCharacterSet::Greek),
                            162 => Some(PostscriptWindowsCharacterSet::Turkish),
                            163 => Some(PostscriptWindowsCharacterSet::Vietnamese),
                            177 => Some(PostscriptWindowsCharacterSet::Hebrew),
                            178 => Some(PostscriptWindowsCharacterSet::Arabic),
                            186 => Some(PostscriptWindowsCharacterSet::Baltic),
                            200 => Some(PostscriptWindowsCharacterSet::Bitstream),
                            204 => Some(PostscriptWindowsCharacterSet::Cyrillic),
                            222 => Some(PostscriptWindowsCharacterSet::Thai),
                            238 => Some(PostscriptWindowsCharacterSet::EasternEuropean),
                            255 => Some(PostscriptWindowsCharacterSet::Oem),
                            _ => {
                                return Err(FontInfoLoadError::FontInfoUpconversion(
                                    FontInfoErrorKind::UnknownMsCharSet(v),
                                ))
                            }
                        },
                        None => None,
                    },
                    style_map_family_name: fontinfo_v1.menuName,
                    style_map_style_name: match fontinfo_v1.fontStyle {
                        Some(v) => match v {
                            0 | 64 => Some(StyleMapStyle::Regular),
                            1 => Some(StyleMapStyle::Italic),
                            32 => Some(StyleMapStyle::Bold),
                            33 => Some(StyleMapStyle::BoldItalic),
                            _ => {
                                return Err(FontInfoLoadError::FontInfoUpconversion(
                                    FontInfoErrorKind::UnknownFontStyle(v),
                                ))
                            }
                        },
                        None => None,
                    },
                    style_name: fontinfo_v1.styleName,
                    trademark: fontinfo_v1.trademark,
                    units_per_em: fontinfo_v1
                        .unitsPerEm
                        .map(|v| NonNegativeIntegerOrFloat::new(v.abs()).unwrap()),
                    version_major: fontinfo_v1.versionMajor,
                    version_minor: fontinfo_v1.versionMinor.map(|v| v.unsigned_abs()),
                    x_height: fontinfo_v1.xHeight,
                    year: fontinfo_v1.year,
                    ..FontInfo::default()
                };
                fontinfo.validate().map_err(FontInfoLoadError::FontInfoUpconversion)?;
                Ok(fontinfo)
            }
        }
    }

    /// Returns `false` if this [`FontInfo`] has any non-default value, and `true` otherwise.
    pub fn is_empty(&self) -> bool {
        self == &Self::default()
    }

    /// Validates various fields according to the [specification][].
    ///
    /// [specification]: http://unifiedfontobject.org/versions/ufo3/fontinfo.plist/
    pub fn validate(&self) -> Result<(), FontInfoErrorKind> {
        // The date format is "YYYY/MM/DD HH:MM:SS". This does not validate that the
        // days ceiling is valid for the month, as this would probably need a specialist
        // datetime library.
        if let Some(v) = &self.open_type_head_created {
            const DATE_LENGTH: usize = 19;
            if v.len() != DATE_LENGTH {
                return Err(FontInfoErrorKind::InvalidOpenTypeHeadCreatedDate);
            }
            if !v.chars().all(|b| b.is_ascii_digit() || b == ' ' || b == '/' || b == ':') {
                return Err(FontInfoErrorKind::InvalidOpenTypeHeadCreatedDate);
            }

            if !(v[0..4].parse::<u16>().is_ok()
                && &v[4..5] == "/"
                && v[5..7]
                    .parse::<u8>()
                    .map_err(|_| FontInfoErrorKind::InvalidOpenTypeHeadCreatedDate)?
                    <= 12
                && &v[7..8] == "/"
                && v[8..10]
                    .parse::<u8>()
                    .map_err(|_| FontInfoErrorKind::InvalidOpenTypeHeadCreatedDate)?
                    <= 31
                && &v[10..11] == " "
                && v[11..13]
                    .parse::<u8>()
                    .map_err(|_| FontInfoErrorKind::InvalidOpenTypeHeadCreatedDate)?
                    < 24
                && &v[13..14] == ":"
                && v[14..16]
                    .parse::<u8>()
                    .map_err(|_| FontInfoErrorKind::InvalidOpenTypeHeadCreatedDate)?
                    < 60
                && &v[16..17] == ":"
                && v[17..19]
                    .parse::<u8>()
                    .map_err(|_| FontInfoErrorKind::InvalidOpenTypeHeadCreatedDate)?
                    < 60)
            {
                return Err(FontInfoErrorKind::InvalidOpenTypeHeadCreatedDate);
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
                for current in vs_iter {
                    if last > current {
                        return Err(FontInfoErrorKind::UnsortedGaspEntries);
                    }
                    last = current;
                }
            }
        }

        // Guideline identifiers must be unique within fontinfo.
        if let Some(guidelines) = &self.guidelines {
            let mut identifiers: HashSet<Identifier> = HashSet::new();
            for guideline in guidelines {
                if let Some(id) = guideline.identifier() {
                    if !identifiers.insert(id.clone()) {
                        return Err(FontInfoErrorKind::DuplicateGuidelineIdentifiers);
                    }
                }
            }
        }

        // openTypeOS2Selection must not contain bits 0, 5 or 6.
        if let Some(v) = &self.open_type_os2_selection {
            if v.contains(&0) || v.contains(&5) || v.contains(&6) {
                return Err(FontInfoErrorKind::DisallowedSelectionBits);
            }
        }

        if let Some(v) = &self.open_type_os2_family_class {
            if !v.is_valid() {
                return Err(FontInfoErrorKind::InvalidOs2FamilyClass);
            }
        }

        // The Postscript blue zone and stem widths lists have a length limitation.
        if let Some(v) = &self.postscript_blue_values {
            let name = "postscriptBlueValues";
            if v.len() > 14 {
                return Err(FontInfoErrorKind::InvalidPostscriptListLength {
                    name,
                    max_len: 14,
                    len: v.len(),
                });
            }
            if v.len() % 2 != 0 {
                return Err(FontInfoErrorKind::PostscriptListMustBePairs(name));
            }
        }
        if let Some(v) = &self.postscript_other_blues {
            let name = "postscriptOtherBlues";
            if v.len() > 10 {
                return Err(FontInfoErrorKind::InvalidPostscriptListLength {
                    name,
                    max_len: 10,
                    len: v.len(),
                });
            }
            if v.len() % 2 != 0 {
                return Err(FontInfoErrorKind::PostscriptListMustBePairs(name));
            }
        }
        if let Some(v) = &self.postscript_family_blues {
            let name = "postscriptFamilyBlues";
            if v.len() > 14 {
                return Err(FontInfoErrorKind::InvalidPostscriptListLength {
                    name,
                    max_len: 14,
                    len: v.len(),
                });
            }
            if v.len() % 2 != 0 {
                return Err(FontInfoErrorKind::PostscriptListMustBePairs(name));
            }
        }
        if let Some(v) = &self.postscript_family_other_blues {
            let name = "postscriptFamilyOtherBlues";
            if v.len() > 10 {
                return Err(FontInfoErrorKind::InvalidPostscriptListLength {
                    name,
                    max_len: 10,
                    len: v.len(),
                });
            }
            if v.len() % 2 != 0 {
                return Err(FontInfoErrorKind::PostscriptListMustBePairs(name));
            }
        }
        if let Some(v) = &self.postscript_stem_snap_h {
            if v.len() > 12 {
                return Err(FontInfoErrorKind::InvalidPostscriptListLength {
                    name: "postscriptStemSnapH",
                    max_len: 12,
                    len: v.len(),
                });
            }
        }
        if let Some(v) = &self.postscript_stem_snap_v {
            if v.len() > 12 {
                return Err(FontInfoErrorKind::InvalidPostscriptListLength {
                    name: "postscriptStemSnapV",
                    max_len: 12,
                    len: v.len(),
                });
            }
        }

        // Certain WOFF attributes must contain at least one item if they are present.
        if let Some(v) = &self.woff_metadata_extensions {
            if v.is_empty() {
                return Err(FontInfoErrorKind::EmptyWoffAttribute("woffMetadataExtensions"));
            }

            for record in v.iter() {
                if record.items.is_empty() {
                    return Err(FontInfoErrorKind::EmptyWoffAttribute(
                        "woffMetadataExtensions record, items",
                    ));
                }

                for record_item in record.items.iter() {
                    if record_item.names.is_empty() || record_item.values.is_empty() {
                        return Err(FontInfoErrorKind::EmptyWoffAttribute(
                            "woffMetadataExtensions record, item names or values",
                        ));
                    }
                }
            }
        }
        if let Some(v) = &self.woff_metadata_credits {
            if v.credits.is_empty() {
                return Err(FontInfoErrorKind::EmptyWoffAttribute("woffMetadataCredits"));
            }
        }
        if let Some(v) = &self.woff_metadata_copyright {
            if v.text.is_empty() {
                return Err(FontInfoErrorKind::EmptyWoffAttribute("woffMetadataCopyright"));
            }
        }
        if let Some(v) = &self.woff_metadata_description {
            if v.text.is_empty() {
                return Err(FontInfoErrorKind::EmptyWoffAttribute("woffMetadataDescription, text"));
            }
        }
        if let Some(v) = &self.woff_metadata_trademark {
            if v.text.is_empty() {
                return Err(FontInfoErrorKind::EmptyWoffAttribute("woffMetadataTrademark"));
            }
        }

        Ok(())
    }

    /// Move libs from the font lib's `public.objectLibs` key into the actual objects.
    /// The key will be removed from the font lib.
    fn load_object_libs(&mut self, lib: &mut Plist) -> Result<(), FontInfoLoadError> {
        let mut object_libs = match lib.remove(PUBLIC_OBJECT_LIBS_KEY) {
            Some(lib) => {
                lib.into_dictionary().ok_or(FontInfoLoadError::PublicObjectLibsMustBeDictionary)?
            }
            None => return Ok(()),
        };

        if let Some(guidelines) = &mut self.guidelines {
            for guideline in guidelines {
                if let Some(id) = guideline.identifier() {
                    if let Some(lib) = object_libs.remove(id.as_str()) {
                        let lib = lib.into_dictionary().ok_or_else(|| {
                            FontInfoLoadError::GlobalGuidelineLibMustBeDictionary(
                                id.as_str().to_string(),
                            )
                        })?;
                        guideline.lib = Some(lib);
                    }
                }
            }
        }

        Ok(())
    }

    /// Dump guideline libs into a [`Plist`].
    pub(crate) fn dump_object_libs(&self) -> Plist {
        let mut object_libs = Plist::default();

        if let Some(guidelines) = &self.guidelines {
            for guideline in guidelines {
                if let Some(lib) = guideline.lib() {
                    let id = guideline.identifier().map(|id| id.as_str().to_string());
                    object_libs.insert(id.unwrap(), plist::Value::Dictionary(lib.clone()));
                }
            }
        }

        object_libs
    }
}

impl NonNegativeIntegerOrFloat {
    /// A validating constructor.
    ///
    /// Returns a new [`NonNegativeIntegerOrFloat`] with the given `value`,
    /// or `None` if the value is less than or equal to zero.
    ///
    /// In addition to this constructor, this type also implements `TryFrom` for
    /// `f64`, and `From` for `u32`.
    pub fn new(value: f64) -> Option<Self> {
        if value.is_sign_positive() {
            Some(NonNegativeIntegerOrFloat(value))
        } else {
            None
        }
    }

    /// Returns the value as an `f64`.
    pub fn as_f64(&self) -> f64 {
        self.0
    }

    /// Returns `true` if the value is an integer.
    pub(crate) fn is_integer(&self) -> bool {
        self.0.fract().abs() < f64::EPSILON
    }
}

impl Deref for NonNegativeIntegerOrFloat {
    type Target = f64;

    fn deref(&self) -> &f64 {
        &self.0
    }
}

impl TryFrom<f64> for NonNegativeIntegerOrFloat {
    type Error = ExpectedPositiveValue;

    fn try_from(value: f64) -> Result<Self, Self::Error> {
        match NonNegativeIntegerOrFloat::new(value) {
            Some(v) => Ok(v),
            _ => Err(ExpectedPositiveValue),
        }
    }
}

impl From<u32> for NonNegativeIntegerOrFloat {
    fn from(src: u32) -> NonNegativeIntegerOrFloat {
        NonNegativeIntegerOrFloat(src as f64)
    }
}

impl Serialize for NonNegativeIntegerOrFloat {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        if self.is_integer() {
            serializer.serialize_i32(self.0 as i32)
        } else {
            serializer.serialize_f64(self.0)
        }
    }
}

impl<'de> Deserialize<'de> for NonNegativeIntegerOrFloat {
    fn deserialize<D>(deserializer: D) -> Result<NonNegativeIntegerOrFloat, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value: f64 = Deserialize::deserialize(deserializer)?;
        NonNegativeIntegerOrFloat::try_from(value).map_err(serde::de::Error::custom)
    }
}

mod serde_impls {
    use serde::{ser::SerializeSeq, Serialize, Serializer};

    struct IntegerOrFloat(f64);

    impl Serialize for IntegerOrFloat {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            if self.0.fract().abs() <= f64::EPSILON {
                serializer.serialize_i32(self.0 as i32)
            } else {
                serializer.serialize_f64(self.0)
            }
        }
    }

    pub(crate) fn ser_opt_int_or_float<S: Serializer>(
        value: &Option<f64>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        value.map(IntegerOrFloat).serialize(serializer)
    }

    pub(crate) fn ser_opt_vec_int_or_float<S: Serializer>(
        value: &Option<Vec<f64>>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        match value {
            None => serializer.serialize_none(),
            Some(items) => {
                let mut seq = serializer.serialize_seq(Some(items.len()))?;
                for item in items {
                    seq.serialize_element(&IntegerOrFloat(*item))?;
                }
                seq.end()
            }
        }
    }
}

/// Corresponds to [gasp Range Record Format](http://unifiedfontobject.org/versions/ufo3/fontinfo.plist/#gasp-range-record-format).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct GaspRangeRecord {
    /// Range max PPEM.
    #[serde(rename = "rangeMaxPPEM")]
    pub range_max_ppem: NonNegativeInteger,
    /// Range gasp behavior.
    pub range_gasp_behavior: Vec<GaspBehavior>,
}

/// Corresponds to [rangeGaspBehavior Bits](http://unifiedfontobject.org/versions/ufo3/fontinfo.plist/#rangegaspbehavior-bits).
#[derive(Debug, Clone, Copy, Serialize_repr, Deserialize_repr, PartialEq, Eq)]
#[repr(u8)]
pub enum GaspBehavior {
    /// Use grid fitting.
    Gridfit = 0,
    /// Use gray scale rendering.
    DoGray = 1,
    /// Use grid fitting with symmetric smoothing.
    SymmetricGridfit = 2,
    /// Use multi-axis smoothing.
    SymmetricSmoothing = 3,
}

/// Corresponds to [Name Record Format](http://unifiedfontobject.org/versions/ufo3/fontinfo.plist/#name-record-format).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NameRecord {
    /// Name ID.
    #[serde(rename = "nameID")]
    pub name_id: NonNegativeInteger,
    /// Platform ID.
    #[serde(rename = "platformID")]
    pub platform_id: NonNegativeInteger,
    /// Platform encoding ID.
    #[serde(rename = "encodingID")]
    pub encoding_id: NonNegativeInteger,
    /// Language ID.
    #[serde(rename = "languageID")]
    pub language_id: NonNegativeInteger,
    /// Name record string value.
    pub string: String,
}

/// Corresponds to the allowed values for
/// [openTypeOS2WidthClass](http://unifiedfontobject.org/versions/ufo3/fontinfo.plist/#opentype-os2-table-fields).
#[derive(Debug, Clone, Copy, Default, Serialize_repr, Deserialize_repr, PartialEq, Eq)]
#[repr(u8)]
pub enum Os2WidthClass {
    /// Ultra-condensed width.
    UltraCondensed = 1,
    /// Extra-condensed width.
    ExtraCondensed = 2,
    /// Condensed width.
    Condensed = 3,
    /// Semi-condensed width.
    SemiCondensed = 4,
    /// Medium (normal) width.
    #[default]
    Normal = 5,
    /// Semi-expanded width.
    SemiExpanded = 6,
    /// Expanded width.
    Expanded = 7,
    /// Extra-expanded width.
    ExtraExpanded = 8,
    /// Ultra-expanded width.
    UltraExpanded = 9,
}

/// Corresponds to [openTypeOS2FamilyClass](http://unifiedfontobject.org/versions/ufo3/fontinfo.plist/#opentype-os2-table-fields).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Os2FamilyClass {
    /// Class ID.
    pub class_id: u8,
    /// Sub-class ID.
    pub subclass_id: u8,
}

impl Os2FamilyClass {
    /// The first number, representing the class ID, must be in the range 0-14.
    /// The second number, representing the subclass, must be in the range 0-15.
    fn is_valid(&self) -> bool {
        (0..=14).contains(&self.class_id) && (0..=15).contains(&self.subclass_id)
    }
}

impl Serialize for Os2FamilyClass {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut seq = serializer.serialize_seq(Some(2))?;
        seq.serialize_element(&self.class_id)?;
        seq.serialize_element(&self.subclass_id)?;
        seq.end()
    }
}

impl<'de> Deserialize<'de> for Os2FamilyClass {
    fn deserialize<D>(deserializer: D) -> Result<Os2FamilyClass, D::Error>
    where
        D: Deserializer<'de>,
    {
        let values: Vec<u8> = Deserialize::deserialize(deserializer)?;
        if values.len() != 2 {
            return Err(serde::de::Error::custom(FontInfoErrorKind::InvalidOs2FamilyClass));
        }

        Ok(Os2FamilyClass { class_id: values[0], subclass_id: values[1] })
    }
}

/// Corresponds to [openTypeOS2Panose](http://unifiedfontobject.org/versions/ufo3/fontinfo.plist/#opentype-os2-table-fields).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Os2Panose {
    /// Panose family type.
    pub family_type: NonNegativeInteger,
    /// Panose serif style.
    pub serif_style: NonNegativeInteger,
    /// Panose weight.
    pub weight: NonNegativeInteger,
    /// Panose proportion.
    pub proportion: NonNegativeInteger,
    /// Panose contrast.
    pub contrast: NonNegativeInteger,
    /// Panose stroke variation.
    pub stroke_variation: NonNegativeInteger,
    /// Panose arm style.
    pub arm_style: NonNegativeInteger,
    /// Panose letterform.
    pub letterform: NonNegativeInteger,
    /// Panose midline.
    pub midline: NonNegativeInteger,
    /// Panose x-height.
    pub x_height: NonNegativeInteger,
}

impl Serialize for Os2Panose {
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

impl<'de> Deserialize<'de> for Os2Panose {
    fn deserialize<D>(deserializer: D) -> Result<Os2Panose, D::Error>
    where
        D: Deserializer<'de>,
    {
        let values: Vec<NonNegativeInteger> = Deserialize::deserialize(deserializer)?;
        if values.len() != 10 {
            return Err(serde::de::Error::custom(FontInfoErrorKind::InvalidOs2Panose));
        }

        Ok(Os2Panose {
            family_type: values[0],
            serif_style: values[1],
            weight: values[2],
            proportion: values[3],
            contrast: values[4],
            stroke_variation: values[5],
            arm_style: values[6],
            letterform: values[7],
            midline: values[8],
            x_height: values[9],
        })
    }
}

impl From<Os2PanoseV2> for Os2Panose {
    fn from(value: Os2PanoseV2) -> Self {
        Os2Panose {
            family_type: value.family_type.unsigned_abs(),
            serif_style: value.serif_style.unsigned_abs(),
            weight: value.weight.unsigned_abs(),
            proportion: value.proportion.unsigned_abs(),
            contrast: value.contrast.unsigned_abs(),
            stroke_variation: value.stroke_variation.unsigned_abs(),
            arm_style: value.arm_style.unsigned_abs(),
            letterform: value.letterform.unsigned_abs(),
            midline: value.midline.unsigned_abs(),
            x_height: value.x_height.unsigned_abs(),
        }
    }
}

/// OS2PanoseV2 is from UFO v2 and allows negative integers, while the OpenType specification
/// specifies unsigned integers.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct Os2PanoseV2 {
    family_type: Integer,
    serif_style: Integer,
    weight: Integer,
    proportion: Integer,
    contrast: Integer,
    stroke_variation: Integer,
    arm_style: Integer,
    letterform: Integer,
    midline: Integer,
    x_height: Integer,
}

impl<'de> Deserialize<'de> for Os2PanoseV2 {
    fn deserialize<D>(deserializer: D) -> Result<Os2PanoseV2, D::Error>
    where
        D: Deserializer<'de>,
    {
        let values: Vec<Integer> = Deserialize::deserialize(deserializer)?;
        if values.len() != 10 {
            return Err(serde::de::Error::custom(FontInfoErrorKind::InvalidOs2Panose));
        }

        Ok(Os2PanoseV2 {
            family_type: values[0],
            serif_style: values[1],
            weight: values[2],
            proportion: values[3],
            contrast: values[4],
            stroke_variation: values[5],
            arm_style: values[6],
            letterform: values[7],
            midline: values[8],
            x_height: values[9],
        })
    }
}

/// Corresponds to postscriptWindowsCharacterSet in [PostScript Specific Data](http://unifiedfontobject.org/versions/ufo3/fontinfo.plist/#postscript-specific-data).
#[derive(Debug, Clone, Copy, Serialize_repr, Deserialize_repr, PartialEq, Eq)]
#[repr(u8)]
pub enum PostscriptWindowsCharacterSet {
    /// ANSI character set.
    Ansi = 1,
    /// Default character set.
    Default = 2,
    /// Symbol character set.
    Symbol = 3,
    /// Macintosh character set.
    Macintosh = 4,
    /// Shift JIS character set.
    ShiftJis = 5,
    /// Hangul character set.
    Hangul = 6,
    /// Hangul (Johab) character set.
    HangulJohab = 7,
    /// GB2312 character set.
    Gb2312 = 8,
    /// Chinese BIG5 character set.
    ChineseBig5 = 9,
    /// Greek character set.
    Greek = 10,
    /// Turkish character set.
    Turkish = 11,
    /// Vietnamese character set.
    Vietnamese = 12,
    /// Hebrew character set.
    Hebrew = 13,
    /// Arabic character set.
    Arabic = 14,
    /// Baltic character set.
    Baltic = 15,
    /// Bitstream character set.
    Bitstream = 16,
    /// Cyrillic character set.
    Cyrillic = 17,
    /// Thai character set.
    Thai = 18,
    /// Eastern European character set.
    EasternEuropean = 19,
    /// OEM character set.
    Oem = 20,
}

/// Corresponds to woffMetadataCopyright in [WOFF Data](http://unifiedfontobject.org/versions/ufo3/fontinfo.plist/#woff-data).
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct WoffMetadataCopyright {
    /// WOFF Metadata Copyright Record
    pub text: Vec<WoffMetadataTextRecord>,
}

/// Corresponds to woffMetadataCredits in [WOFF Data](http://unifiedfontobject.org/versions/ufo3/fontinfo.plist/#woff-data).
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct WoffMetadataCredits {
    /// A collection of WOFF Metadata Credit Records
    pub credits: Vec<WoffMetadataCredit>,
}

/// A WOFF Metadata Credits Record data structure.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct WoffMetadataCredit {
    /// Name.
    pub name: String,
    /// URL.
    pub url: Option<String>,
    /// Role.
    pub role: Option<String>,
    /// Writing direction.
    pub dir: Option<WoffAttributeDirection>,
    /// Class.
    pub class: Option<String>,
}

/// Corresponds to woffMetadataDescription in [WOFF Data](http://unifiedfontobject.org/versions/ufo3/fontinfo.plist/#woff-data).
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct WoffMetadataDescription {
    /// Description URL.
    pub url: Option<String>,
    /// Description Text.
    pub text: Vec<WoffMetadataTextRecord>,
}

/// A WOFF Metadata Text Record data structure.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct WoffMetadataTextRecord {
    /// Text.
    pub text: String,
    /// Language.
    pub language: Option<String>,
    /// Writing direction.
    pub dir: Option<WoffAttributeDirection>,
    /// Class.
    pub class: Option<String>,
}

/// A WOFF Metadata Extension Record data structure.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct WoffMetadataExtensionRecord {
    /// Identifier.
    pub id: Option<String>,
    /// Extension names.
    pub names: Vec<WoffMetadataExtensionNameRecord>,
    /// Extension items.
    pub items: Vec<WoffMetadataExtensionItemRecord>,
}

/// A WOFF Metadata Extension Name Record data structure.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct WoffMetadataExtensionNameRecord {
    /// Text.
    pub text: String,
    /// Language.
    pub language: Option<String>,
    /// Writing direction.
    pub dir: Option<WoffAttributeDirection>,
    /// Class.
    pub class: Option<String>,
}

/// A WOFF Metadata Extension Item Record data structure.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct WoffMetadataExtensionItemRecord {
    /// Identifier.
    pub id: Option<String>, // XXX: Spec does not specify if required, assume optional.
    /// Extension names.
    pub names: Vec<WoffMetadataExtensionNameRecord>,
    /// Extension values.
    pub values: Vec<WoffMetadataExtensionValueRecord>,
}

/// A WOFF Metadata Extension Value Record data structure.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct WoffMetadataExtensionValueRecord {
    /// Text.
    pub text: String,
    /// Language.
    pub language: Option<String>,
    /// Writing direction.
    pub dir: Option<WoffAttributeDirection>,
    /// Class.
    pub class: Option<String>,
}

/// Corresponds to woffMetadataLicense in [WOFF Data](http://unifiedfontobject.org/versions/ufo3/fontinfo.plist/#woff-data).
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct WoffMetadataLicense {
    /// License URL.
    pub url: Option<String>,
    /// License identifier.
    pub id: Option<String>,
    /// License text.
    pub text: Vec<WoffMetadataTextRecord>,
}

/// Corresponds to woffMetadataLicensee in [WOFF Data](http://unifiedfontobject.org/versions/ufo3/fontinfo.plist/#woff-data).
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct WoffMetadataLicensee {
    /// Licensee name.
    pub name: String,
    /// Writing direction.
    pub dir: Option<WoffAttributeDirection>,
    /// Class.
    pub class: Option<String>,
}

/// Corresponds to woffMetadataTrademark in [WOFF Data](http://unifiedfontobject.org/versions/ufo3/fontinfo.plist/#woff-data).
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct WoffMetadataTrademark {
    /// Trademark text.
    pub text: Vec<WoffMetadataTextRecord>,
}

/// Corresponds to woffMetadataUniqueID in [WOFF Data](http://unifiedfontobject.org/versions/ufo3/fontinfo.plist/#woff-data).
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct WoffMetadataUniqueId {
    /// Unique identifier.
    pub id: String,
}

/// Corresponds to woffMetadataVendor in [WOFF Data](http://unifiedfontobject.org/versions/ufo3/fontinfo.plist/#woff-data).
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct WoffMetadataVendor {
    /// Vendor name.
    pub name: String,
    /// Vendor URL.
    pub url: String,
    /// Writing direction.
    pub dir: Option<WoffAttributeDirection>,
    /// Class.
    pub class: Option<String>,
}

/// Corresponds to the writing direction attribute used in [WOFF Data](http://unifiedfontobject.org/versions/ufo3/fontinfo.plist/#woff-data).
/// If present, is either "ltr" or "rtl".
#[derive(Debug, Eq, PartialEq, Clone)]
pub enum WoffAttributeDirection {
    /// Left to Right writing direction.
    LeftToRight,
    /// Right to Left writing direction.
    RightToLeft,
}

impl Serialize for WoffAttributeDirection {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            WoffAttributeDirection::LeftToRight => serializer.serialize_str("ltr"),
            WoffAttributeDirection::RightToLeft => serializer.serialize_str("rtl"),
        }
    }
}

impl<'de> Deserialize<'de> for WoffAttributeDirection {
    fn deserialize<D>(deserializer: D) -> Result<WoffAttributeDirection, D::Error>
    where
        D: Deserializer<'de>,
    {
        let string = String::deserialize(deserializer)?;
        match string.as_ref() {
            "ltr" => Ok(WoffAttributeDirection::LeftToRight),
            "rtl" => Ok(WoffAttributeDirection::RightToLeft),
            _ => Err(serde::de::Error::custom(FontInfoErrorKind::UnknownWoffDirection)),
        }
    }
}

/// Corresponds to the `styleMapStyleName` in [Generic Identification Information](http://unifiedfontobject.org/versions/ufo3/fontinfo.plist/#generic-identification-information).
///
/// If present, is either "regular", "italic", "bold" or "bold italic".
#[derive(Debug, Eq, PartialEq, Clone)]
pub enum StyleMapStyle {
    /// Regular style.
    Regular,
    /// Italic style.
    Italic,
    /// Bold style.
    Bold,
    /// Bold Italic style.
    BoldItalic,
}

impl Serialize for StyleMapStyle {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            StyleMapStyle::Regular => serializer.serialize_str("regular"),
            StyleMapStyle::Italic => serializer.serialize_str("italic"),
            StyleMapStyle::Bold => serializer.serialize_str("bold"),
            StyleMapStyle::BoldItalic => serializer.serialize_str("bold italic"),
        }
    }
}

impl<'de> Deserialize<'de> for StyleMapStyle {
    fn deserialize<D>(deserializer: D) -> Result<StyleMapStyle, D::Error>
    where
        D: Deserializer<'de>,
    {
        let string = String::deserialize(deserializer)?;
        match string.as_ref() {
            "regular" => Ok(StyleMapStyle::Regular),
            "italic" => Ok(StyleMapStyle::Italic),
            "bold" => Ok(StyleMapStyle::Bold),
            "bold italic" => Ok(StyleMapStyle::BoldItalic),
            _ => Err(serde::de::Error::custom(FontInfoErrorKind::UnknownStyleMapStyleName)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Color, Identifier, Line, Name};
    use serde_test::{assert_tokens, Token};

    #[test]
    fn fontinfo() {
        let path = "testdata/MutatorSansLightWide.ufo/fontinfo.plist";
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
            Some(Os2FamilyClass { class_id: 0, subclass_id: 0 })
        );
        assert_eq!(
            font_info.open_type_os2_panose,
            Some(Os2Panose {
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
                Guideline::new(Line::Angle { x: 82.0, y: 720.0, degrees: 90.0 }, None, None, None,),
                Guideline::new(Line::Vertical(372.0), None, None, None,),
                Guideline::new(Line::Horizontal(123.0), None, None, None,),
                Guideline::new(
                    Line::Angle { x: 1.0, y: 2.0, degrees: 0.0 },
                    Some(Name::new_raw(" [locked]")),
                    Some(Color::new(1.0, 1.0, 1.0, 1.0).unwrap()),
                    Some(Identifier::new_raw("abc")),
                ),
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
        let c1 = Os2FamilyClass { class_id: 14, subclass_id: 15 };
        assert_tokens(
            &c1,
            &[Token::Seq { len: Some(2) }, Token::U8(14), Token::U8(15), Token::SeqEnd],
        );
    }

    #[test]
    fn test_serde_os2_panose() {
        let p1 = Os2Panose {
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
    #[allow(clippy::field_reassign_with_default)]
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

    #[test]
    fn test_validate_guideline_identifiers() {
        let mut fi = FontInfo::default();
        assert!(fi.validate().is_ok());

        fi.guidelines.replace(vec![
            Guideline::new(Line::Horizontal(10.0), None, None, Some(Identifier::new_raw("test1"))),
            Guideline::new(Line::Vertical(20.0), None, None, Some(Identifier::new_raw("test2"))),
        ]);
        assert!(fi.validate().is_ok());

        fi.guidelines.replace(vec![
            Guideline::new(Line::Horizontal(10.0), None, None, Some(Identifier::new_raw("test1"))),
            Guideline::new(Line::Vertical(20.0), None, None, Some(Identifier::new_raw("test1"))),
        ]);
        assert!(fi.validate().is_err());
    }

    #[test]
    fn test_positive_int_or_float() {
        assert!(NonNegativeIntegerOrFloat::try_from(-1.0).is_err());
        assert!(NonNegativeIntegerOrFloat::try_from(1.0).is_ok());
    }

    #[test]
    fn test_postscript_blue_values_pairs() {
        let mut fi = FontInfo {
            postscript_blue_values: Some(vec![1.0, 2.0, 3.0, 4.0]),
            ..Default::default()
        };
        assert!(fi.validate().is_ok());
        fi.postscript_blue_values = Some(vec![]);
        assert!(fi.validate().is_ok());
        fi.postscript_blue_values = Some(vec![1.0, 2.0, 3.0, 4.0, 5.0]);
        assert!(matches!(fi.validate(), Err(FontInfoErrorKind::PostscriptListMustBePairs(_))));
    }

    #[test]
    fn test_postscript_other_blues_pairs() {
        let mut fi = FontInfo {
            postscript_other_blues: Some(vec![1.0, 2.0, 3.0, 4.0]),
            ..Default::default()
        };
        assert!(fi.validate().is_ok());
        fi.postscript_other_blues = Some(vec![]);
        assert!(fi.validate().is_ok());
        fi.postscript_other_blues = Some(vec![1.0, 2.0, 3.0, 4.0, 5.0]);
        assert!(matches!(fi.validate(), Err(FontInfoErrorKind::PostscriptListMustBePairs(_))));
    }

    #[test]
    fn test_postscript_family_blues_pairs() {
        let mut fi = FontInfo {
            postscript_family_blues: Some(vec![1.0, 2.0, 3.0, 4.0]),
            ..Default::default()
        };
        assert!(fi.validate().is_ok());
        fi.postscript_family_blues = Some(vec![]);
        assert!(fi.validate().is_ok());
        fi.postscript_family_blues = Some(vec![1.0, 2.0, 3.0, 4.0, 5.0]);
        assert!(matches!(fi.validate(), Err(FontInfoErrorKind::PostscriptListMustBePairs(_))));
    }

    #[test]
    fn test_postscript_family_other_blues_pairs() {
        let mut fi = FontInfo {
            postscript_family_other_blues: Some(vec![1.0, 2.0, 3.0, 4.0]),
            ..Default::default()
        };
        assert!(fi.validate().is_ok());
        fi.postscript_family_other_blues = Some(vec![]);
        assert!(fi.validate().is_ok());
        fi.postscript_family_other_blues = Some(vec![1.0, 2.0, 3.0, 4.0, 5.0]);
        assert!(matches!(fi.validate(), Err(FontInfoErrorKind::PostscriptListMustBePairs(_))));
    }

    #[test]
    fn test_postscript_stem_snap_h_length() {
        let mut fi =
            FontInfo { postscript_stem_snap_h: Some([0.0; 11].into()), ..Default::default() };
        assert!(fi.validate().is_ok());
        fi.postscript_stem_snap_h = Some(vec![]);
        assert!(fi.validate().is_ok());
        fi.postscript_stem_snap_h = Some([0.0; 12].into());
        assert!(fi.validate().is_ok());
        fi.postscript_stem_snap_h = Some([0.0; 13].into());
        assert!(matches!(
            fi.validate(),
            Err(FontInfoErrorKind::InvalidPostscriptListLength { name: _, max_len: _, len: _ })
        ));
    }

    #[test]
    fn test_postscript_stem_snap_v_length() {
        let mut fi =
            FontInfo { postscript_stem_snap_v: Some([0.0; 11].into()), ..Default::default() };
        assert!(fi.validate().is_ok());
        fi.postscript_stem_snap_v = Some(vec![]);
        assert!(fi.validate().is_ok());
        fi.postscript_stem_snap_v = Some([0.0; 12].into());
        assert!(fi.validate().is_ok());
        fi.postscript_stem_snap_v = Some([0.0; 13].into());
        assert!(matches!(
            fi.validate(),
            Err(FontInfoErrorKind::InvalidPostscriptListLength { name: _, max_len: _, len: _ })
        ));
    }
}
