use std::cell::Cell;

use norad::{Contour, ContourPoint, Glyph, GlyphName, PyId};
use pyo3::{
    class::basic::CompareOp, exceptions, prelude::*, PyIterProtocol, PyObjectProtocol, PyRef,
    PySequenceProtocol,
};

use super::{LayerProxy, ProxyError};

#[pyclass]
#[derive(Debug, Clone)]
pub struct GlyphProxy {
    pub(crate) layer: LayerProxy,
    pub(crate) glyph: GlyphName,
}

macro_rules! flatten {
    ($expr:expr $(,)?) => {
        match $expr {
            Err(e) => Err(e),
            Ok(Err(e)) => Err(e),
            Ok(Ok(fine)) => Ok(fine),
        }
    };
}

impl GlyphProxy {
    fn with<R>(&self, f: impl FnOnce(&Glyph) -> R) -> Result<R, ProxyError> {
        flatten!(self.layer.with(|l| l
            .get_glyph(&self.glyph)
            .ok_or_else(|| ProxyError::MissingGlyph {
                layer: self.layer.name.clone(),
                glyph: self.glyph.clone()
            })
            .map(|g| { f(g) })))
    }

    fn with_mut<R>(&self, f: impl FnOnce(&mut Glyph) -> R) -> Result<R, ProxyError> {
        flatten!(self.layer.with_mut(|l| l
            .get_glyph_mut(&self.glyph)
            .ok_or_else(|| ProxyError::MissingGlyph {
                layer: self.layer.name.clone(),
                glyph: self.glyph.clone()
            })
            .map(|g| f(g))))
    }
}

#[pymethods]
impl GlyphProxy {
    #[getter]
    fn contours(&self) -> ContoursProxy {
        ContoursProxy { glyph: self.clone() }
    }

    #[getter]
    fn name(&self) -> &str {
        &self.glyph
    }
}

#[pyclass]
#[derive(Debug, Clone)]
pub struct ContoursProxy {
    glyph: GlyphProxy,
}

#[pyproto]
impl PySequenceProtocol for ContoursProxy {
    fn __len__(&self) -> usize {
        self.glyph.with(|g| g.contours.len()).unwrap_or(0)
    }

    fn __getitem__(&'p self, idx: isize) -> Option<ContourProxy> {
        let idx: usize = if idx.is_negative() {
            self.__len__().checked_sub(idx.abs() as usize)?
        } else {
            idx as usize
        };

        self.glyph
            .with(|g| {
                g.contours.get(idx).map(|contour| ContourProxy {
                    glyph: self.glyph.clone(),
                    idx: Cell::new(idx),
                    py_id: contour.py_id,
                })
            })
            .ok()
            .flatten()
    }
}

#[pyclass]
#[derive(Debug, Clone)]
pub struct ContourProxy {
    glyph: GlyphProxy,
    idx: Cell<usize>,
    py_id: PyId,
}

#[pymethods]
impl ContourProxy {
    #[getter]
    fn points(&self) -> PointsProxy {
        PointsProxy { contour: self.clone() }
    }
}

impl ContourProxy {
    fn with<R>(&self, f: impl FnOnce(&Contour) -> R) -> Result<R, ProxyError> {
        flatten!(self.glyph.with(|g| match g.contours.get(self.idx.get()) {
            Some(c) if c.py_id == self.py_id => Some(c),
            //NOTE: if we don't find the item or the id doesn't match, we do
            // a linear search for the id; if we find it we update our index.
            _ => match g.contours.iter().enumerate().find(|(_, c)| c.py_id == self.py_id) {
                Some((i, c)) => {
                    self.idx.set(i);
                    Some(c)
                }
                None => None,
            },
        }
        .ok_or_else(|| ProxyError::MissingContour {
            layer: self.glyph.layer.name.clone(),
            glyph: self.glyph.glyph.clone(),
            contour: self.idx.get(),
        })
        .map(|g| f(g))))
    }

    fn with_mut<R>(&self, f: impl FnOnce(&mut Contour) -> R) -> Result<R, ProxyError> {
        flatten!(self.glyph.with_mut(|g| match g.contours.get_mut(self.idx.get()) {
            Some(c) if c.py_id == self.py_id => Some(c),
            _ => match g.contours.iter_mut().enumerate().find(|(_, c)| c.py_id == self.py_id) {
                Some((i, c)) => {
                    self.idx.set(i);
                    Some(c)
                }
                None => None,
            },
        }
        .ok_or_else(|| ProxyError::MissingContour {
            layer: self.glyph.layer.name.clone(),
            glyph: self.glyph.glyph.clone(),
            contour: self.idx.get(),
        })
        .map(|g| f(g))))
    }
}

#[pyclass]
#[derive(Debug, Clone)]
pub struct PointsProxy {
    contour: ContourProxy,
}

#[pyclass]
pub struct PointsIter {
    points: PointsProxy,
    //len: usize,
    ix: usize,
}

#[pymethods]
impl PointsProxy {
    fn iter_points(&self) -> PointsIter {
        PointsIter { points: self.clone(), ix: 0 }
    }
}

#[pyproto]
impl PySequenceProtocol for PointsProxy {
    fn __len__(&self) -> usize {
        self.contour.with(|c| c.points.len()).unwrap_or(0)
    }

    fn __getitem__(&'p self, idx: isize) -> PyResult<PointProxy> {
        let idx = python_idx_to_idx(idx, self.__len__())?;
        self.contour
            .with(|c| PointProxy {
                contour: self.contour.clone(),
                idx: Cell::new(idx),
                py_id: c.points[idx].py_id,
            })
            .map_err(Into::into)
    }

    fn __delitem__(&'p mut self, idx: isize) -> PyResult<()> {
        let idx = python_idx_to_idx(idx, self.__len__())?;
        self.contour
            .with_mut(|contour| {
                contour.points.remove(idx);
            })
            .map_err(Into::into)
    }
}

fn python_idx_to_idx(idx: isize, len: usize) -> PyResult<usize> {
    let idx = if idx.is_negative() { len - (idx.abs() as usize % len) } else { idx as usize };

    if idx < len {
        Ok(idx)
    } else {
        Err(exceptions::PyIndexError::new_err(format!(
            "Index {} out of bounds of collection with length {}",
            idx, len
        )))
    }
}

#[pyproto]
impl PyIterProtocol for PointsProxy {
    fn __iter__(slf: PyRef<Self>) -> PointsIter {
        slf.iter_points()
    }
}

#[pyproto]
impl PyIterProtocol for PointsIter {
    fn __iter__(slf: PyRef<'p, Self>) -> PyRef<'p, Self> {
        slf
    }

    fn __next__(mut slf: PyRefMut<Self>) -> Option<PointProxy> {
        let index = slf.ix;
        slf.ix += 1;
        slf.points.__getitem__(index as isize).ok()
    }
}

#[pyclass]
pub struct PointProxy {
    contour: ContourProxy,
    idx: Cell<usize>,
    py_id: PyId,
}

impl PointProxy {
    fn with<R>(&self, f: impl FnOnce(&ContourPoint) -> R) -> Result<R, ProxyError> {
        flatten!(self.contour.with(|c| match c.points.get(self.idx.get()) {
            Some(pt) if pt.py_id == self.py_id => Some(pt),
            _ => match c.points.iter().enumerate().find(|(_, pt)| pt.py_id == self.py_id) {
                Some((i, pt)) => {
                    self.idx.set(i);
                    Some(pt)
                }
                None => None,
            },
        }
        .ok_or_else(|| ProxyError::MissingPoint {
            layer: self.contour.glyph.layer.name.clone(),
            glyph: self.contour.glyph.glyph.clone(),
            contour: self.contour.idx.get(),
            point: self.idx.get()
        })
        .map(|g| f(g))))
    }

    fn with_mut<R>(&self, f: impl FnOnce(&mut ContourPoint) -> R) -> Result<R, ProxyError> {
        flatten!(self.contour.with_mut(|c| match c.points.get_mut(self.idx.get()) {
            Some(pt) if pt.py_id == self.py_id => Some(pt),
            _ => match c.points.iter_mut().enumerate().find(|(_, pt)| pt.py_id == self.py_id) {
                Some((i, pt)) => {
                    self.idx.set(i);
                    Some(pt)
                }
                None => None,
            },
        }
        .ok_or_else(|| ProxyError::MissingPoint {
            layer: self.contour.glyph.layer.name.clone(),
            glyph: self.contour.glyph.glyph.clone(),
            contour: self.contour.idx.get(),
            point: self.idx.get()
        })
        .map(|g| f(g))))
    }
}

#[pymethods]
impl PointProxy {
    #[getter]
    fn get_x(&self) -> PyResult<f32> {
        self.with(|p| p.x).map_err(Into::into)
    }

    #[setter]
    fn set_x(&self, x: f32) -> PyResult<()> {
        self.with_mut(|p| p.x = x).map_err(Into::into)
    }

    #[getter]
    fn get_y(&self) -> PyResult<f32> {
        self.with(|p| p.y).map_err(Into::into)
    }

    #[setter]
    fn set_y(&self, y: f32) -> PyResult<()> {
        self.with_mut(|p| p.y = y).map_err(Into::into)
    }

    fn py_eq(&self, other: PyRef<PointProxy>) -> PyResult<bool> {
        let other: &PointProxy = &*other;
        flatten!(self.with(|p| other.with(|p2| p == p2))).map_err(Into::into)
    }
}

#[pyproto]
impl PyObjectProtocol for PointProxy {
    fn __richcmp__(&'p self, other: PyRef<PointProxy>, op: CompareOp) -> PyResult<bool> {
        let other: &PointProxy = &*other;
        match op {
            CompareOp::Eq => flatten!(self.with(|p| other.with(|p2| p == p2))).map_err(Into::into),
            CompareOp::Ne => flatten!(self.with(|p| other.with(|p2| p != p2))).map_err(Into::into),
            _ => Err(exceptions::PyNotImplementedError::new_err("")),
        }
    }
}