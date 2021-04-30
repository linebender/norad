use std::collections::HashMap;

use plist::{Dictionary, Value};
use pyo3::{exceptions::PyValueError, prelude::*, types::PyDict};

use super::{ProxyError, PyAnchor, PyComponent, PyContour, PyFont, PyGlyph, PyGuideline, PyPoint};

#[pyclass]
pub struct PyLib {
    pub(crate) inner: LibProxy,
}

pub enum LibProxy {
    Font(PyFont),
    Glyph(PyGlyph),
    Point(PyPoint),
    Contour(PyContour),
    Component(PyComponent),
    Anchor(PyAnchor),
    Guideline(PyGuideline),
}

impl From<PyFont> for PyLib {
    fn from(src: PyFont) -> PyLib {
        PyLib { inner: LibProxy::Font(src) }
    }
}
impl From<PyGlyph> for PyLib {
    fn from(src: PyGlyph) -> PyLib {
        PyLib { inner: LibProxy::Glyph(src) }
    }
}
impl From<PyPoint> for PyLib {
    fn from(src: PyPoint) -> PyLib {
        PyLib { inner: LibProxy::Point(src) }
    }
}

impl From<PyContour> for PyLib {
    fn from(src: PyContour) -> PyLib {
        PyLib { inner: LibProxy::Contour(src) }
    }
}
impl From<PyComponent> for PyLib {
    fn from(src: PyComponent) -> PyLib {
        PyLib { inner: LibProxy::Component(src) }
    }
}

impl From<PyAnchor> for PyLib {
    fn from(src: PyAnchor) -> PyLib {
        PyLib { inner: LibProxy::Anchor(src) }
    }
}

impl From<PyGuideline> for PyLib {
    fn from(src: PyGuideline) -> PyLib {
        PyLib { inner: LibProxy::Guideline(src) }
    }
}

impl PyLib {
    pub(crate) fn with<R>(&self, f: impl FnOnce(&Dictionary) -> R) -> Result<R, ProxyError> {
        match &self.inner {
            LibProxy::Font(font) => Ok(f(&font.read().lib)),
            LibProxy::Glyph(glyph) => glyph.with(|g| f(&g.lib)),
            LibProxy::Point(point) => point.with(|p| match p.lib() {
                Some(lib) => f(lib),
                None => f(&Dictionary::new()),
            }),
            LibProxy::Contour(contour) => contour.with(|c| match c.lib() {
                Some(lib) => f(lib),
                None => f(&Dictionary::new()),
            }),
            LibProxy::Component(component) => component.with(|c| match c.lib() {
                Some(lib) => f(lib),
                None => f(&Dictionary::new()),
            }),
            LibProxy::Anchor(anchor) => anchor.with(|a| match a.lib() {
                Some(lib) => f(lib),
                None => f(&Dictionary::new()),
            }),
            LibProxy::Guideline(guide) => guide.with(|g| match g.lib() {
                Some(lib) => f(lib),
                None => f(&Dictionary::new()),
            }),
        }
    }

    pub(crate) fn with_mut<R>(
        &mut self,
        f: impl FnOnce(&mut Dictionary) -> R,
    ) -> Result<R, ProxyError> {
        match &mut self.inner {
            LibProxy::Font(font) => Ok(f(&mut font.write().lib)),
            LibProxy::Glyph(glyph) => glyph.with_mut(|g| f(&mut g.lib)),
            LibProxy::Point(point) => point.with_mut(|p| match p.lib_mut() {
                Some(lib) => f(lib),
                None => {
                    let mut dict = Dictionary::new();
                    let r = f(&mut dict);
                    p.replace_lib(dict);
                    r
                }
            }),
            LibProxy::Contour(contour) => contour.with_mut(|c| match c.lib_mut() {
                Some(lib) => f(lib),
                None => {
                    let mut dict = Dictionary::new();
                    let r = f(&mut dict);
                    c.replace_lib(dict);
                    r
                }
            }),
            LibProxy::Component(component) => component.with_mut(|c| match c.lib_mut() {
                Some(lib) => f(lib),
                None => {
                    let mut dict = Dictionary::new();
                    let r = f(&mut dict);
                    c.replace_lib(dict);
                    r
                }
            }),
            LibProxy::Anchor(anchor) => anchor.with_mut(|a| match a.lib_mut() {
                Some(lib) => f(lib),
                None => {
                    let mut dict = Dictionary::new();
                    let r = f(&mut dict);
                    a.replace_lib(dict);
                    r
                }
            }),
            LibProxy::Guideline(guide) => guide.with_mut(|g| match g.lib_mut() {
                Some(lib) => f(lib),
                None => {
                    let mut dict = Dictionary::new();
                    let r = f(&mut dict);
                    g.replace_lib(dict);
                    r
                }
            }),
        }
    }
}

#[pymethods]
impl PyLib {
    fn set_item(&mut self, key: String, value: RawValue) -> PyResult<()> {
        let value = from_python(value)?;
        self.with_mut(|lib| lib.insert(key, value))?;
        Ok(())
    }

    fn get_item(&self, name: &str) -> PyResult<Option<PyObject>> {
        self.with(|lib| lib.get(name).map(to_python))?.transpose()
    }
}

#[pyproto]
impl pyo3::PyMappingProtocol for PyLib {
    fn __len__(&self) -> PyResult<usize> {
        self.with(|lib| lib.len()).map_err(Into::into)
    }

    fn __getitem__(&'p self, name: &str) -> pyo3::PyResult<Option<PyObject>> {
        self.get_item(name)
    }

    fn __setitem__(&'p mut self, key: String, value: RawValue) -> PyResult<()> {
        self.set_item(key, value)
    }

    fn __delitem__(&'p mut self, name: &str) -> pyo3::PyResult<()> {
        self.with_mut(|lib| lib.remove(name))?;
        Ok(())
    }
}

fn to_python(obj: &Value) -> PyResult<PyObject> {
    let gil = Python::acquire_gil();
    let py = gil.python();
    Ok(match obj {
        Value::String(s) => s.to_object(py),
        Value::Real(f) => f.to_object(py),
        Value::Integer(int) => {
            if let Some(uint) = int.as_unsigned() {
                uint.to_object(py)
            } else if let Some(int) = int.as_signed() {
                int.to_object(py)
            } else {
                unreachable!()
            }
        }
        Value::Uid(hmm) => hmm.get().to_object(py),
        Value::Date(date) => format!("{:?}", date).to_object(py),
        Value::Data(data) => data.to_object(py),
        Value::Array(array) => {
            array.iter().map(to_python).collect::<Result<Vec<_>, _>>()?.to_object(py)
        }
        Value::Dictionary(dict) => {
            let d = PyDict::new(py);
            for (key, value) in dict.iter() {
                let value = to_python(value)?;
                d.set_item(key, value)?;
            }
            d.into()
        }
        Value::Boolean(b) => b.to_object(py),
        // should never happen
        _non_exhaustive => return Err(PyValueError::new_err("Unknown plist type")),
    })
}

pub(crate) fn from_python(value: RawValue) -> PyResult<Value> {
    match value {
        RawValue::Int(int) => Ok(int.into()),
        RawValue::Float(float) => Ok(float.into()),
        RawValue::String(string) => Ok(string.into()),
        RawValue::Array(array) => {
            array.into_iter().map(from_python).collect::<Result<Vec<_>, _>>().map(Into::into)
        }
        RawValue::Dict(dict) => {
            let mut plist = Dictionary::new();
            for (key, value) in dict.into_iter() {
                let value = from_python(value)?;
                plist.insert(key, value);
            }
            Ok(plist.into())
        }
    }
}

crate::proxy_eq!(PyLib);

#[derive(FromPyObject)]
pub enum RawValue {
    #[pyo3(annotation = "str")]
    String(String),
    #[pyo3(annotation = "int")]
    Int(i64),
    #[pyo3(annotation = "float")]
    Float(f64),
    Array(Vec<RawValue>),
    Dict(HashMap<String, RawValue>),
}
