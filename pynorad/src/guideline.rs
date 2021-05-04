use std::cell::Cell;
use std::sync::{Arc, RwLock};

use super::glyph::{GlyphGuidelineProxy, GlyphGuidelinesProxy};
use super::{util, ProxyError, PyFontInfo};
use norad::{Guideline, Line, PyId};
use pyo3::{exceptions::PyValueError, prelude::*, types::PyType, PySequenceProtocol};

#[pyclass]
#[derive(Clone, Debug)]
pub struct PyGuideline {
    inner: GuidelineProxy,
}

#[derive(Clone, Debug)]
enum GuidelineProxy {
    Font { font: PyFontInfo, py_id: PyId },
    Glyph(GlyphGuidelineProxy),
    Concrete { guideline: Arc<RwLock<Guideline>> },
}

#[pyclass]
pub struct GuidelinesProxy {
    pub(crate) info: PyFontInfo,
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

    #[getter]
    fn name(&self) -> PyResult<Option<String>> {
        self.with(|g| g.name.clone()).map_err(Into::into)
    }

    #[getter]
    fn identifier(&self) -> PyResult<Option<String>> {
        self.with(|g| g.identifier().map(|id| id.as_str().to_owned())).map_err(Into::into)
    }

    #[getter]
    fn color(&self) -> PyResult<Option<String>> {
        self.with(|g| g.color.as_ref().map(|c| c.to_string())).map_err(Into::into)
    }

    #[getter]
    fn x(&self) -> PyResult<Option<f32>> {
        self.with(|g| match g.line {
            Line::Angle { x, .. } => Some(x),
            Line::Vertical(x) => Some(x),
            Line::Horizontal(_) => None,
        })
        .map_err(Into::into)
    }

    #[getter]
    fn y(&self) -> PyResult<Option<f32>> {
        self.with(|g| match g.line {
            Line::Angle { y, .. } => Some(y),
            Line::Horizontal(y) => Some(y),
            Line::Vertical(_) => None,
        })
        .map_err(Into::into)
    }

    #[getter]
    fn angle(&self) -> PyResult<Option<f32>> {
        self.with(|g| match g.line {
            Line::Angle { degrees, .. } => Some(degrees),
            _ => None,
        })
        .map_err(Into::into)
    }
}

impl PyGuideline {
    pub(crate) fn font_proxy(font: PyFontInfo, py_id: PyId) -> Self {
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
                .with(|info| {
                    info.guidelines.as_ref().map(|gs| gs.iter().find(|g| g.py_id == *py_id).map(f))
                })
                .flatten()
                .flatten()
                .ok_or(ProxyError::MissingGlobalGuideline),
            GuidelineProxy::Glyph(proxy) => proxy.with(f),
            GuidelineProxy::Concrete { guideline, .. } => Ok(f(&guideline.read().unwrap())),
        }
    }

    pub fn with_mut<R>(&mut self, f: impl FnOnce(&mut Guideline) -> R) -> Result<R, ProxyError> {
        match &mut self.inner {
            GuidelineProxy::Font { font, py_id } => font
                .with_mut(|info| {
                    info.guidelines
                        .get_or_insert_with(Default::default)
                        .iter_mut()
                        .find(|g| g.py_id == *py_id)
                        .map(f)
                })
                .flatten()
                .ok_or(ProxyError::MissingGlobalGuideline),
            GuidelineProxy::Glyph(proxy) => proxy.with_mut(f),
            GuidelineProxy::Concrete { guideline, .. } => Ok(f(&mut guideline.write().unwrap())),
        }
    }
}

impl GuidelinesProxy {
    pub fn with<R>(&self, f: impl FnOnce(&[Guideline]) -> R) -> Result<Option<R>, ProxyError> {
        Ok(self.info.with(|info| match info.guidelines.as_ref() {
            Some(g) => f(g),
            None => f(&[]),
        }))
    }

    pub fn with_mut<R>(
        &mut self,
        f: impl FnOnce(&mut Vec<Guideline>) -> R,
    ) -> Result<Option<R>, ProxyError> {
        Ok(self.info.with_mut(|info| f(info.guidelines.get_or_insert_with(Default::default))))
    }
}

#[pyproto]
impl PySequenceProtocol for GuidelinesProxy {
    fn __len__(&self) -> PyResult<usize> {
        self.with(|guides| guides.len()).map(|i| i.unwrap_or(0)).map_err(Into::into)
    }

    fn __getitem__(&'p self, idx: isize) -> PyResult<Option<PyGuideline>> {
        let idx = util::python_idx_to_idx(idx, self.__len__()?)?;
        self.with(|guides| PyGuideline {
            inner: GuidelineProxy::Font { font: self.info.clone(), py_id: guides[idx].py_id },
        })
        .map_err(Into::into)
    }

    fn __delitem__(&'p mut self, idx: isize) -> PyResult<()> {
        let idx = util::python_idx_to_idx(idx, self.__len__()?)?;
        self.with_mut(|guides| guides.remove(idx))?;
        Ok(())
    }
}
