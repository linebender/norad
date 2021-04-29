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
            fn $setname(&mut self, value: $typ) -> PyResult<()> {
                let value = $setconv(value)?;
                self.with_mut(|info| info.$intname = Some(value));
                Ok(())
            }
        }
    };
}

gettersetter!(ascender, ascender, set_ascender, f64);
gettersetter!(descender, descender, set_descender, f64);
gettersetter!(italicAngle, italic_angle, set_italic_angle, f64);
gettersetter!(capHeight, cap_height, set_cap_height, f64);
gettersetter!(copyright, copyright, set_copyright, String);
gettersetter!(unitsPerEm, units_per_em, set_units_per_em, f64, conv_try_into, conv_into);
gettersetter!(versionMajor, version_major, set_version_major, i32);
gettersetter!(versionMajor, version_minor, set_version_minor, u32);
gettersetter!(styleMapFamilyName, style_map_family_name, set_style_map_family_name, String);
gettersetter!(
    styleMapStyleName,
    style_map_style_name,
    set_style_map_style_name,
    String,
    style_from_string,
    from_style
);

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
