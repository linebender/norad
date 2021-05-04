use std::convert::TryInto;
use std::ops::DerefMut;
use std::sync::{Arc, RwLock};

use super::guideline::{GuidelinesProxy, PyGuideline};
use super::PyFont;
use norad::{fontinfo::StyleMapStyle, FontInfo, Guideline};
use pyo3::{exceptions::PyValueError, prelude::*, types::PyType};

#[pyclass]
#[derive(Debug, Clone)]
pub struct PyFontInfo {
    inner: FontInfoProxy,
}

#[pymethods]
impl PyFontInfo {
    #[classmethod]
    fn concrete(_cls: &PyType) -> Self {
        PyFontInfo { inner: FontInfoProxy::Concrete(Arc::new(RwLock::new(FontInfo::default()))) }
    }
}

#[derive(Debug, Clone)]
enum FontInfoProxy {
    Font { font: PyFont },
    Concrete(Arc<RwLock<FontInfo>>),
}

impl PyFontInfo {
    pub(crate) fn proxy(font: PyFont) -> Self {
        PyFontInfo { inner: FontInfoProxy::Font { font } }
    }
    pub fn with<R>(&self, f: impl FnOnce(&FontInfo) -> R) -> Option<R> {
        match &self.inner {
            FontInfoProxy::Font { font } => font.read().font_info.as_ref().map(f),
            FontInfoProxy::Concrete(info) => Some(f(&info.read().unwrap())),
        }
    }

    pub fn with_mut<R>(&mut self, f: impl FnOnce(&mut FontInfo) -> R) -> Option<R> {
        match &mut self.inner {
            FontInfoProxy::Font { font } => {
                Some(f(font.write().font_info.get_or_insert_with(Default::default)))
            }
            FontInfoProxy::Concrete(info) => Some(f(&mut info.write().unwrap())),
        }
    }
}
fn conv_into<T: Into<R>, R>(item: T) -> PyResult<R> {
    Ok(item.into())
}

fn conv_try_into<T, R>(item: T) -> PyResult<R>
where
    T: TryInto<R>,
    <T as TryInto<R>>::Error: std::error::Error,
{
    item.try_into().map_err(|e| PyValueError::new_err(e.to_string()))
}

fn from_style(style: StyleMapStyle) -> PyResult<String> {
    Ok(style.to_string())
}

fn style_from_string(string: String) -> PyResult<StyleMapStyle> {
    match string.as_str() {
        "bold" => Ok(StyleMapStyle::Bold),
        "italic" => Ok(StyleMapStyle::Italic),
        "bold italic" => Ok(StyleMapStyle::BoldItalic),
        "regular" => Ok(StyleMapStyle::Regular),
        _ => Err(PyValueError::new_err(format!("invalid style name '{}'", string))),
    }
}

fn from_float_vec(v: Vec<f64>) -> PyResult<Vec<norad::IntegerOrFloat>> {
    Ok(v.into_iter().map(Into::into).collect())
}

fn to_float_vec(v: Vec<norad::IntegerOrFloat>) -> PyResult<Vec<f64>> {
    Ok(v.into_iter().map(Into::into).collect())
}

fn conv_de<T, D>(v: D) -> PyResult<T>
where
    T: serde::de::DeserializeOwned,
    D: serde::de::IntoDeserializer<'static>,
{
    T::deserialize(v.into_deserializer()).map_err(|e| PyValueError::new_err(e.to_string()))
}

macro_rules! gettersetter {
    ($extname:ident, $intname:ident, $setname:ident, $typ:ty) => {
        gettersetter!($extname, $intname, $setname, $typ, conv_into, conv_into);
    };

    ($extname:ident, $intname:ident, $setname:ident, $typ:ty, $setconv:ident, $getconv:ident) => {
        #[pymethods]
        impl PyFontInfo {
            #[getter($extname)]
            fn $intname(&self) -> PyResult<Option<$typ>> {
                self.with(|info| info.$intname.clone().map($getconv)).flatten().transpose()
            }

            #[setter($extname)]
            fn $setname(&mut self, value: Option<$typ>) -> PyResult<()> {
                let value = value.map($setconv).transpose()?;
                self.with_mut(|info| info.$intname = value);
                Ok(())
            }
        }
    };
}

macro_rules! fakeproperty {
    ($extname:ident, $intname:ident, $setname:ident) => {
        #[pymethods]
        impl PyFontInfo {
            #[getter($extname)]
            fn $intname(&self) -> PyResult<Option<()>> {
                Ok(None)
            }

            #[setter($extname)]
            fn $setname(&mut self, _value: Option<PyObject>) -> PyResult<()> {
                Ok(())
            }
        }
    };
}

gettersetter!(familyName, family_name, set_family_name, String);
gettersetter!(styleName, style_name, set_style_name, String);
gettersetter!(year, year, set_year, i32);
gettersetter!(ascender, ascender, set_ascender, f64);
gettersetter!(descender, descender, set_descender, f64);
gettersetter!(italicAngle, italic_angle, set_italic_angle, f64);
gettersetter!(capHeight, cap_height, set_cap_height, f64);
gettersetter!(xHeight, x_height, set_x_height, f64);
gettersetter!(copyright, copyright, set_copyright, String);
gettersetter!(unitsPerEm, units_per_em, set_units_per_em, f64, conv_try_into, conv_into);
gettersetter!(versionMajor, version_major, set_version_major, i32);
gettersetter!(versionMinor, version_minor, set_version_minor, u32);

gettersetter!(note, note, set_note, String);
gettersetter!(trademark, trademark, set_trademark, String);

gettersetter!(styleMapFamilyName, style_map_family_name, set_style_map_family_name, String);
gettersetter!(
    styleMapStyleName,
    style_map_style_name,
    set_style_map_style_name,
    String,
    style_from_string,
    from_style
);

gettersetter!(openTypeHeadFlags, open_type_head_flags, set_open_type_head_flags, Vec<u8>);
gettersetter!(
    openTypeOS2CodePageRanges,
    open_type_os2_code_page_ranges,
    set_open_type_os2_code_page_ranges,
    Vec<u8>
);
gettersetter!(openTypeOS2Selection, open_type_os2_selection, set_open_type_os2_selection, Vec<u8>);
gettersetter!(openTypeOS2Type, open_type_os2_type, set_open_type_os2_type, Vec<u8>);
gettersetter!(
    openTypeOS2UnicodeRanges,
    open_type_os2_unicode_ranges,
    set_open_type_os2_unicode_ranges,
    Vec<u8>
);
gettersetter!(
    openTypeOS2FamilyClass,
    open_type_os2_family_class,
    set_open_type_os2_family_class,
    (u8, u8)
);
gettersetter!(
    openTypeOS2Panose,
    open_type_os2_panose,
    set_open_type_os2_panose,
    Vec<u32>,
    conv_de,
    conv_into
);
gettersetter!(
    openTypeOS2WidthClass,
    open_type_os2_width_class,
    set_open_type_os2_width_class,
    u8,
    conv_de,
    conv_into
);
gettersetter!(
    postscriptWindowsCharacterSet,
    postscript_windows_character_set,
    set_postscript_windows_character_set,
    u8,
    conv_de,
    conv_into
);
gettersetter!(openTypeHeadCreated, open_type_head_created, set_open_type_head_created, String);
gettersetter!(
    openTypeNameCompatibleFullName,
    open_type_name_compatible_full_name,
    set_open_type_name_compatible_full_name,
    String
);
gettersetter!(
    openTypeNameDescription,
    open_type_name_description,
    set_open_type_name_description,
    String
);
gettersetter!(openTypeNameDesigner, open_type_name_designer, set_open_type_name_designer, String);
gettersetter!(
    openTypeNameDesignerURL,
    open_type_name_designer_url,
    set_open_type_name_designer_url,
    String
);
gettersetter!(openTypeNameLicense, open_type_name_license, set_open_type_name_license, String);
gettersetter!(
    openTypeNameLicenseURL,
    open_type_name_license_url,
    set_open_type_name_license_url,
    String
);
gettersetter!(
    openTypeNameManufacturer,
    open_type_name_manufacturer,
    set_open_type_name_manufacturer,
    String
);
gettersetter!(
    openTypeNameManufacturerURL,
    open_type_name_manufacturer_url,
    set_open_type_name_manufacturer_url,
    String
);
gettersetter!(
    openTypeNamePreferredFamilyName,
    open_type_name_preferred_family_name,
    set_open_type_name_preferred_family_name,
    String
);
gettersetter!(
    openTypeNamePreferredSubfamilyName,
    open_type_name_preferred_subfamily_name,
    set_open_type_name_preferred_subfamily_name,
    String
);
gettersetter!(
    openTypeNameSampleText,
    open_type_name_sample_text,
    set_open_type_name_sample_text,
    String
);
gettersetter!(openTypeNameUniqueID, open_type_name_unique_id, set_open_type_name_unique_id, String);
gettersetter!(openTypeNameVersion, open_type_name_version, set_open_type_name_version, String);
gettersetter!(
    openTypeNameWWSFamilyName,
    open_type_name_wws_family_name,
    set_open_type_name_wwsfamily_name,
    String
);
gettersetter!(
    openTypeNameWWSSubfamilyName,
    open_type_name_wws_subfamily_name,
    set_open_type_name_wwssubfamily_name,
    String
);
gettersetter!(openTypeOS2VendorID, open_type_os2_vendor_id, set_open_type_os2_vendor_id, String);
gettersetter!(
    postscriptDefaultCharacter,
    postscript_default_character,
    set_postscript_default_character,
    String
);
gettersetter!(postscriptFontName, postscript_font_name, set_postscript_font_name, String);
gettersetter!(postscriptFullName, postscript_full_name, set_postscript_full_name, String);
gettersetter!(postscriptWeightName, postscript_weight_name, set_postscript_weight_name, String);
gettersetter!(
    postscriptBlueValues,
    postscript_blue_values,
    set_postscript_blue_values,
    Vec<f64>,
    from_float_vec,
    to_float_vec
);
gettersetter!(
    postscriptFamilyBlues,
    postscript_family_blues,
    set_postscript_family_blues,
    Vec<f64>,
    from_float_vec,
    to_float_vec
);
gettersetter!(
    postscriptFamilyOtherBlues,
    postscript_family_other_blues,
    set_postscript_family_other_blues,
    Vec<f64>,
    from_float_vec,
    to_float_vec
);
gettersetter!(
    postscriptOtherBlues,
    postscript_other_blues,
    set_postscript_other_blues,
    Vec<f64>,
    from_float_vec,
    to_float_vec
);
gettersetter!(
    postscriptStemSnapH,
    postscript_stem_snap_h,
    set_postscript_stem_snap_h,
    Vec<f64>,
    from_float_vec,
    to_float_vec
);
gettersetter!(
    postscriptStemSnapV,
    postscript_stem_snap_v,
    set_postscript_stem_snap_v,
    Vec<f64>,
    from_float_vec,
    to_float_vec
);
gettersetter!(postscriptForceBold, postscript_force_bold, set_postscript_force_bold, bool);
gettersetter!(
    postscriptIsFixedPitch,
    postscript_is_fixed_pitch,
    set_postscript_is_fixed_pitch,
    bool
);
gettersetter!(
    openTypeHeadLowestRecPPEM,
    open_type_head_lowest_rec_ppem,
    set_open_type_head_lowest_rec_ppem,
    u32
);
gettersetter!(openTypeHheaAscender, open_type_hhea_ascender, set_open_type_hhea_ascender, i32);
gettersetter!(
    openTypeHheaCaretOffset,
    open_type_hhea_caret_offset,
    set_open_type_hhea_caret_offset,
    i32
);
gettersetter!(openTypeHheaDescender, open_type_hhea_descender, set_open_type_hhea_descender, i32);
gettersetter!(openTypeHheaLineGap, open_type_hhea_line_gap, set_open_type_hhea_line_gap, i32);
gettersetter!(
    openTypeOS2StrikeoutPosition,
    open_type_os2_strikeout_position,
    set_open_type_os2_strikeout_position,
    i32
);
gettersetter!(
    openTypeOS2StrikeoutSize,
    open_type_os2_strikeout_size,
    set_open_type_os2_strikeout_size,
    i32
);
gettersetter!(
    openTypeOS2SubscriptXOffset,
    open_type_os2_subscript_x_offset,
    set_open_type_os2_subscript_xoffset,
    i32
);
gettersetter!(
    openTypeOS2SubscriptXSize,
    open_type_os2_subscript_x_size,
    set_open_type_os2_subscript_xsize,
    i32
);
gettersetter!(
    openTypeOS2SubscriptYOffset,
    open_type_os2_subscript_y_offset,
    set_open_type_os2_subscript_yoffset,
    i32
);
gettersetter!(
    openTypeOS2SubscriptYSize,
    open_type_os2_subscript_y_size,
    set_open_type_os2_subscript_ysize,
    i32
);
gettersetter!(
    openTypeOS2SuperscriptXOffset,
    open_type_os2_superscript_x_offset,
    set_open_type_os2_superscript_xoffset,
    i32
);
gettersetter!(
    openTypeOS2SuperscriptXSize,
    open_type_os2_superscript_x_size,
    set_open_type_os2_superscript_xsize,
    i32
);
gettersetter!(
    openTypeOS2SuperscriptYOffset,
    open_type_os2_superscript_y_offset,
    set_open_type_os2_superscript_yoffset,
    i32
);
gettersetter!(
    openTypeOS2SuperscriptYSize,
    open_type_os2_superscript_y_size,
    set_open_type_os2_superscript_ysize,
    i32
);
gettersetter!(
    openTypeOS2TypoAscender,
    open_type_os2_typo_ascender,
    set_open_type_os2_typo_ascender,
    i32
);
gettersetter!(
    openTypeOS2TypoDescender,
    open_type_os2_typo_descender,
    set_open_type_os2_typo_descender,
    i32
);
gettersetter!(
    openTypeOS2TypoLineGap,
    open_type_os2_typo_line_gap,
    set_open_type_os2_typo_line_gap,
    i32
);
gettersetter!(openTypeOS2WinAscent, open_type_os2_win_ascent, set_open_type_os2_win_ascent, u32);
gettersetter!(openTypeOS2WinDescent, open_type_os2_win_descent, set_open_type_os2_win_descent, u32);
gettersetter!(
    openTypeVheaCaretOffset,
    open_type_vhea_caret_offset,
    set_open_type_vhea_caret_offset,
    i32
);
gettersetter!(
    openTypeVheaVertTypoAscender,
    open_type_vhea_vert_typo_ascender,
    set_open_type_vhea_vert_typo_ascender,
    i32
);
gettersetter!(
    openTypeVheaVertTypoDescender,
    open_type_vhea_vert_typo_descender,
    set_open_type_vhea_vert_typo_descender,
    i32
);
gettersetter!(
    openTypeVheaVertTypoLineGap,
    open_type_vhea_vert_typo_line_gap,
    set_open_type_vhea_vert_typo_line_gap,
    i32
);
gettersetter!(postscriptBlueFuzz, postscript_blue_fuzz, set_postscript_blue_fuzz, f64);
gettersetter!(postscriptBlueScale, postscript_blue_scale, set_postscript_blue_scale, f64);
gettersetter!(postscriptBlueShift, postscript_blue_shift, set_postscript_blue_shift, f64);
gettersetter!(
    postscriptDefaultWidthX,
    postscript_default_width_x,
    set_postscript_default_width_x,
    f64
);
gettersetter!(
    postscriptNominalWidthX,
    postscript_nominal_width_x,
    set_postscript_nominal_width_x,
    f64
);
gettersetter!(postscriptSlantAngle, postscript_slant_angle, set_postscript_slant_angle, f64);
gettersetter!(
    postscriptUnderlinePosition,
    postscript_underline_position,
    set_postscript_underline_position,
    f64
);
gettersetter!(
    postscriptUnderlineThickness,
    postscript_underline_thickness,
    set_postscript_underline_thickness,
    f64
);
gettersetter!(
    openTypeHheaCaretSlopeRise,
    open_type_hhea_caret_slope_rise,
    set_open_type_hhea_caret_slope_rise,
    i32
);
gettersetter!(
    openTypeHheaCaretSlopeRun,
    open_type_hhea_caret_slope_run,
    set_open_type_hhea_caret_slope_run,
    i32
);
gettersetter!(
    openTypeVheaCaretSlopeRise,
    open_type_vhea_caret_slope_rise,
    set_open_type_vhea_caret_slope_rise,
    i32
);
gettersetter!(
    openTypeVheaCaretSlopeRun,
    open_type_vhea_caret_slope_run,
    set_open_type_vhea_caret_slope_run,
    i32
);
gettersetter!(postscriptUniqueID, postscript_unique_id, set_postscript_unique_id, i32);
gettersetter!(
    openTypeOS2WeightClass,
    open_type_os2_weight_class,
    set_open_type_os2_weight_class,
    u32
);

//FIXME: these types are too complicated; these properties are noops
gettersetter!(woffMajorVersion, woff_major_version, set_woff_major_version, u32);
gettersetter!(woffMinorVersion, woff_minor_version, set_minor_version, u32);

//woff

fakeproperty!(
    openTypeGaspRangeRecords,
    open_type_gasp_range_records,
    set_open_type_gasp_range_records
);
fakeproperty!(openTypeNameRecords, open_type_name_records, set_open_type_name_records);
fakeproperty!(woffMetadataCopyright, woff_metadata_copyright, set_metadata_copyright);
fakeproperty!(woffMetadataCredits, woff_metadata_credits, set_metadata_credits);
fakeproperty!(woffMetadataDescription, woff_metadata_description, set_metadata_description);
fakeproperty!(woffMetadataExtensions, woff_metadata_extensions, set_metadata_extensions);
fakeproperty!(woffMetadataLicense, woff_metadata_license, set_metadata_license);
fakeproperty!(woffMetadataLicensee, woff_metadata_licensee, set_metadata_licensee);
fakeproperty!(woffMetadataTrademark, woff_metadata_trademark, set_metadata_trademark);
fakeproperty!(woffMetadataUniqueID, woff_metadata_unique_id, set_metadata_unique_id);
fakeproperty!(woffMetadataVendor, woff_metadata_vendor, set_metadata_vendor);

#[pymethods]
impl PyFontInfo {
    #[getter]
    pub(crate) fn get_guidelines(&self) -> GuidelinesProxy {
        GuidelinesProxy { info: self.clone() }
    }

    #[setter]
    pub(crate) fn set_guidelines(
        &mut self,
        mut guidelines: Vec<PyRefMut<PyGuideline>>,
    ) -> PyResult<()> {
        let self_clone = self.clone();
        let r: Result<_, PyErr> = self
            .with_mut(|info| {
                let mut new_guides = Vec::with_capacity(guidelines.len());
                for py_guide in &mut guidelines {
                    let guide = (&*py_guide).with(Guideline::to_owned)?;
                    let py_id = guide.py_id;
                    new_guides.push(guide);
                    *py_guide.deref_mut() = PyGuideline::font_proxy(self_clone.clone(), py_id);
                }
                info.guidelines = Some(new_guides);
                Ok(())
            })
            .transpose();
        r?;
        Ok(())
    }
}
