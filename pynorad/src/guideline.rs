use std::cell::Cell;
use std::sync::{Arc, RwLock};

use super::glyph::{GlyphGuidelineProxy, GlyphGuidelinesProxy};
use super::{util, ProxyError, PyFont};
use norad::{Guideline, Line, PyId};
use pyo3::{exceptions::PyValueError, prelude::*, types::PyType, PySequenceProtocol};

#[pyclass]
#[derive(Clone, Debug)]
pub struct PyGuideline {
    inner: GuidelineProxy,
}

#[derive(Clone, Debug)]
enum GuidelineProxy {
    Font { font: PyFont, py_id: PyId },
    Glyph(GlyphGuidelineProxy),
    Concrete { guideline: Arc<RwLock<Guideline>> },
}

#[pyclass]
pub struct GuidelinesProxy {
    font: PyFont,
}

#[pymethods]
impl PyGuideline {
    #[classmethod]
    fn concrete(
        _cls: &PyType,
        x: Option<f32>,
        y: Option<f32>,
        angle: Option<f32>,
        name: Option<String>,
        color: Option<&str>,
        identifier: Option<&str>,
    ) -> PyResult<Self> {
        let line = match (x, y, angle) {
            (Some(x), None, None) => Line::Vertical(x),
            (None, Some(y), None) => Line::Horizontal(y),
            (Some(x), Some(y), Some(degrees)) if (0.0..=360.0).contains(&degrees) => {
                Line::Angle { x, y, degrees }
            }
            (None, None, _) => return Err(PyValueError::new_err("x or y must be present")),
            (None, _, Some(_)) | (_, None, Some(_)) => {
                return Err(PyValueError::new_err(
                    "if 'x' or 'y' are None, 'angle' must not be present",
                ))
            }
            (Some(_), Some(_), None) => {
                return Err(PyValueError::new_err(
                    "if 'x' and 'y' are defined, 'angle' must be defined",
                ))
            }
            (_, _, Some(_)) => {
                return Err(PyValueError::new_err("angle must be between 0 and 360"))
            }
        };
        let identifier = util::to_identifier(identifier)?;
        let color = util::to_color(color)?;
        let guide = Guideline::new(line, name, color, identifier, None);
        Ok(PyGuideline {
            inner: GuidelineProxy::Concrete { guideline: Arc::new(RwLock::new(guide)) },
        })
    }

    fn py_eq(&self, other: PyRef<PyGuideline>) -> PyResult<bool> {
        let other: &PyGuideline = &*other;
        super::flatten!(self.with(|p| other.with(|p2| p == p2))).map_err(Into::into)
    }
}

impl PyGuideline {
    pub(crate) fn font_proxy(font: PyFont, py_id: PyId) -> Self {
        PyGuideline { inner: GuidelineProxy::Font { font, py_id } }
    }

    pub(crate) fn proxy(inner: GlyphGuidelinesProxy, idx: usize, py_id: PyId) -> Self {
        let idx = Cell::new(idx);
        let proxy = GlyphGuidelineProxy { inner, idx, py_id };
        PyGuideline { inner: GuidelineProxy::Glyph(proxy) }
    }

    pub fn with<R>(&self, f: impl FnOnce(&Guideline) -> R) -> Result<R, ProxyError> {
        match &self.inner {
            GuidelineProxy::Font { font, py_id } => font
                .read()
                .guidelines()
                .iter()
                .find(|guide| guide.py_id == *py_id)
                .map(f)
                .ok_or(ProxyError::MissingGlobalGuideline),
            GuidelineProxy::Glyph(proxy) => proxy.with(f),
            GuidelineProxy::Concrete { guideline, .. } => Ok(f(&guideline.read().unwrap())),
        }
    }

    pub fn with_mut<R>(&mut self, f: impl FnOnce(&mut Guideline) -> R) -> Result<R, ProxyError> {
        match &mut self.inner {
            GuidelineProxy::Font { font, py_id } => font
                .write()
                .guidelines_mut()
                .iter_mut()
                .find(|guide| guide.py_id == *py_id)
                .map(f)
                .ok_or(ProxyError::MissingGlobalGuideline),
            GuidelineProxy::Glyph(proxy) => proxy.with_mut(f),
            GuidelineProxy::Concrete { guideline, .. } => Ok(f(&mut guideline.write().unwrap())),
        }
    }
}

impl GuidelinesProxy {
    pub fn with<R>(&self, f: impl FnOnce(&[Guideline]) -> R) -> Result<R, ProxyError> {
        Ok(f(self.font.read().guidelines()))
    }

    pub fn with_mut<R>(
        &mut self,
        f: impl FnOnce(&mut Vec<Guideline>) -> R,
    ) -> Result<R, ProxyError> {
        Ok(f(self.font.write().guidelines_mut()))
    }
}

#[pyproto]
impl PySequenceProtocol for GuidelinesProxy {
    fn __len__(&self) -> PyResult<usize> {
        self.with(|guides| guides.len()).map_err(Into::into)
    }

    fn __getitem__(&'p self, idx: isize) -> PyResult<PyGuideline> {
        let idx = util::python_idx_to_idx(idx, self.__len__()?)?;
        self.with(|guides| PyGuideline {
            inner: GuidelineProxy::Font { font: self.font.clone(), py_id: guides[idx].py_id },
        })
        .map_err(Into::into)
    }

    fn __delitem__(&'p mut self, idx: isize) -> PyResult<()> {
        let idx = util::python_idx_to_idx(idx, self.__len__()?)?;
        self.with_mut(|guides| guides.remove(idx))?;
        Ok(())
    }
}
