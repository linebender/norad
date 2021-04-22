use std::str::FromStr;
use std::sync::{Arc, RwLock};

use super::{util, ProxyError, PyFont};
use norad::{Color, Guideline, Identifier, Line, PyId};
use pyo3::{exceptions::PyValueError, prelude::*, types::PyType, PySequenceProtocol};

#[pyclass]
#[derive(Clone, Debug)]
pub struct PyGuideline {
    inner: GuidelineProxy,
}

#[derive(Clone, Debug)]
enum GuidelineProxy {
    Font { font: PyFont, py_id: PyId },
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
        identifier: Option<String>,
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
        let identifier = identifier.map(Identifier::new).transpose().map_err(|_| {
            PyValueError::new_err(
                "Identifier must be between 0 and 100 characters, each in the range 0x20..=0x7E",
            )
        })?;
        let color = color
            .map(Color::from_str)
            .transpose()
            .map_err(|_| PyValueError::new_err("Invalid color string"))?;
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
    pub(crate) fn proxy(font: PyFont, py_id: PyId) -> Self {
        PyGuideline { inner: GuidelineProxy::Font { font, py_id } }
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
            GuidelineProxy::Concrete { guideline, .. } => Ok(f(&guideline.read().unwrap())),
        }
    }

    pub fn with_mut<R>(&mut self, f: impl FnOnce(&mut Guideline) -> R) -> Result<R, ProxyError> {
        match &self.inner {
            GuidelineProxy::Font { font, py_id } => font
                .write()
                .guidelines_mut()
                .iter_mut()
                .find(|guide| guide.py_id == *py_id)
                .map(f)
                .ok_or(ProxyError::MissingGlobalGuideline),
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
