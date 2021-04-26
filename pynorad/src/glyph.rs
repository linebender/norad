use std::cell::Cell;
use std::sync::{Arc, Mutex, RwLock};

use norad::{Component, Contour, ContourPoint, Glyph, GlyphName, PointType, PyId};
use pyo3::{
    exceptions,
    prelude::*,
    types::{PyDict, PyType},
    PyRef, PySequenceProtocol,
};

use super::{flatten, proxy_eq, seq_proxy, seq_proxy_iter, util, ProxyError, PyLayer};

#[pyclass]
#[derive(Debug, Clone)]
pub struct PyGlyph {
    inner: GlyphProxy,
    pub(crate) name: GlyphName,
}

#[derive(Debug, Clone)]
enum GlyphProxy {
    Layer { layer: PyLayer, py_id: PyId },
    Concrete(Arc<RwLock<Glyph>>),
}

impl PyGlyph {
    pub(crate) fn proxy(name: GlyphName, py_id: PyId, layer: PyLayer) -> Self {
        PyGlyph { inner: GlyphProxy::Layer { layer, py_id }, name }
    }

    pub(crate) fn layer_name(&self) -> &str {
        match &self.inner {
            GlyphProxy::Layer { layer, .. } => layer.name(),
            _ => "None",
        }
    }

    pub(crate) fn with<R>(&self, f: impl FnOnce(&Glyph) -> R) -> Result<R, ProxyError> {
        match &self.inner {
            GlyphProxy::Layer { layer, py_id } => {
                flatten!(layer.with(|l| match l.get_glyph(&self.name) {
                    Some(g) if g.py_id == *py_id => Some(g),
                    _ => l.iter_contents().find(|g| g.py_id == *py_id),
                }
                .ok_or_else(|| ProxyError::MissingGlyph(self.clone()))
                .map(|g| { f(g) })))
            }
            GlyphProxy::Concrete(glyph) => Ok(f(&glyph.read().unwrap())),
        }
    }

    pub(crate) fn with_mut<R>(&mut self, f: impl FnOnce(&mut Glyph) -> R) -> Result<R, ProxyError> {
        let PyGlyph { inner, name } = self;
        match inner {
            GlyphProxy::Layer { layer, py_id } => {
                let result = layer.with_mut(|l| match l.get_glyph_mut(name) {
                    Some(g) if g.py_id == *py_id => Some(f(g)),
                    _ => match l.iter_contents_mut().find(|g| g.py_id == *py_id) {
                        Some(g) => {
                            *name = g.name.clone();
                            Some(f(g))
                        }
                        None => None,
                    },
                })?;
                match result {
                    Some(thing) => Ok(thing),
                    None => Err(ProxyError::MissingGlyph(self.clone())),
                }
            }
            GlyphProxy::Concrete(glyph) => Ok(f(&mut glyph.write().unwrap())),
        }
    }
}

#[pymethods]
impl PyGlyph {
    #[classmethod]
    fn concrete(_cls: &PyType, name: &str) -> Self {
        let name: GlyphName = name.into();
        let glyph = Arc::new(RwLock::new(Glyph::new_named(name.clone())));
        PyGlyph { name, inner: GlyphProxy::Concrete(glyph) }
    }

    #[getter]
    fn contours(&self) -> ContoursProxy {
        ContoursProxy { inner: self.clone() }
    }

    #[getter]
    fn width(&self) -> PyResult<f32> {
        self.with(|g| g.width).map_err(Into::into)
    }

    #[getter]
    fn height(&self) -> PyResult<f32> {
        self.with(|g| g.height).map_err(Into::into)
    }

    #[getter]
    fn name(&self) -> Option<&str> {
        if self.name.is_empty() {
            None
        } else {
            Some(&self.name)
        }
    }

    #[setter]
    fn _name(&mut self, new_name: &str) -> PyResult<()> {
        let new_name: GlyphName = new_name.into();
        self.with_mut(|g| g.name = new_name.clone())?;
        self.name = new_name;
        Ok(())
    }

    fn py_eq(&self, other: PyRef<PyGlyph>) -> PyResult<bool> {
        let other: &PyGlyph = &*other;
        flatten!(self.with(|p| other.with(|p2| p == p2))).map_err(Into::into)
    }

    #[allow(non_snake_case)]
    fn drawPoints(&self, pen: PyObject) -> PyResult<()> {
        self.with(|glyph| {
            let gil = Python::acquire_gil();
            let py = gil.python();

            for c in &glyph.contours {
                if let Err(e) = pen.call_method0(py, "beginPath") {
                    e.restore(py);
                    return;
                }
                for p in &c.points {
                    let coord = (p.x, p.y).to_object(py);
                    let d = PyDict::new(py);
                    d.set_item("segmentType", point_to_str(p.typ)).unwrap();
                    d.set_item("smooth", Some(p.smooth)).unwrap();
                    d.set_item("name", p.name.as_ref()).unwrap();
                    d.set_item("identifier", p.identifier().as_ref().map(|id| id.as_str()))
                        .unwrap();
                    pen.call_method(py, "addPoint", (coord,), Some(d)).unwrap();
                }
                pen.call_method0(py, "endPath").unwrap();
            }
            for c in &glyph.components {
                let transform: kurbo::Affine = c.transform.into();
                let transform = transform.as_coeffs();
                pen.call_method1(
                    py,
                    "addComponent",
                    (c.base.to_object(py), transform.to_object(py)),
                )
                .unwrap();
            }
        })
        .map_err(Into::into)
    }

    fn point_pen(&self) -> PyPointPen {
        PyPointPen { glyph: self.clone(), contour: None }
    }
}

#[pyclass]
pub struct PyPointPen {
    glyph: PyGlyph,
    contour: Option<Arc<Mutex<Contour>>>,
}

#[pymethods]
impl PyPointPen {
    fn begin_path(&mut self, identifier: Option<&str>) -> PyResult<()> {
        let identifier = util::to_identifier(identifier)?;
        self.contour = Some(Arc::new(Mutex::new(Contour::new(Vec::new(), identifier, None))));

        Ok(())
    }

    fn end_path(&mut self) -> PyResult<()> {
        let contour = match self.contour.take().map(Arc::try_unwrap) {
            Some(Ok(contour)) => contour.into_inner().unwrap(),
            Some(Err(arc)) => arc.lock().unwrap().clone(),
            None => return Err(exceptions::PyValueError::new_err("Call beginPath first.")),
        };
        self.glyph.with_mut(|g| g.contours.push(contour)).map_err(Into::into)
    }

    fn add_point(
        &mut self,
        pt: (f32, f32),
        typ: u8,
        smooth: bool,
        name: Option<String>,
        identifier: Option<&str>,
    ) -> PyResult<()> {
        if self.contour.is_none() {
            return Err(exceptions::PyValueError::new_err("Call beginPath first."));
        }
        let identifier = util::to_identifier(identifier)?;
        let typ = match typ {
            0 => PointType::Move,
            1 => PointType::Line,
            2 => PointType::OffCurve,
            3 => PointType::Curve,
            4 => PointType::QCurve,
            _ => unreachable!("values in the range 0..=4 only please"),
        };

        let point = ContourPoint::new(pt.0, pt.1, typ, smooth, name, identifier, None);
        self.contour.as_mut().unwrap().lock().unwrap().points.push(point);
        Ok(())
    }

    fn add_component(
        &mut self,
        name: &str,
        xform: (f64, f64, f64, f64, f64, f64),
        identifier: Option<&str>,
    ) -> PyResult<()> {
        let identifier = util::to_identifier(identifier)?;
        let transform = kurbo::Affine::new([xform.0, xform.1, xform.2, xform.3, xform.4, xform.5]);

        let component = Component::new(name.into(), transform.into(), identifier, None);
        self.glyph.with_mut(|g| g.components.push(component)).map_err(Into::into)
    }
}

fn point_to_str(p: PointType) -> Option<&'static str> {
    match p {
        PointType::Move => Some("move"),
        PointType::Line => Some("line"),
        PointType::OffCurve => None,
        PointType::Curve => Some("curve"),
        PointType::QCurve => Some("qcurve"),
    }
}

seq_proxy!(ContoursProxy, PyGlyph, ContourProxy, contours, Contour);
proxy_eq!(ContoursProxy);
seq_proxy_iter!(ContoursIter, ContoursProxy, ContourProxy);

#[pyclass]
#[derive(Debug, Clone)]
pub struct ContourProxy {
    pub(crate) inner: ContoursProxy,
    pub(crate) idx: Cell<usize>,
    py_id: PyId,
}

#[pymethods]
impl ContourProxy {
    #[getter]
    fn points(&self) -> PointsProxy {
        PointsProxy { inner: self.clone() }
    }
}

impl ContourProxy {
    fn new(inner: ContoursProxy, idx: usize, py_id: PyId) -> Self {
        ContourProxy { inner, idx: Cell::new(idx), py_id }
    }

    fn with<R>(&self, f: impl FnOnce(&Contour) -> R) -> Result<R, ProxyError> {
        flatten!(self.inner.with(|contours| match contours.get(self.idx.get()) {
            Some(c) if c.py_id == self.py_id => Some(c),
            //NOTE: if we don't find the item or the id doesn't match, we do
            // a linear search for the id; if we find it we update our index.
            _ => match contours.iter().enumerate().find(|(_, c)| c.py_id == self.py_id) {
                Some((i, c)) => {
                    self.idx.set(i);
                    Some(c)
                }
                None => None,
            },
        }
        .ok_or_else(|| ProxyError::MissingContour(self.clone()))
        .map(|g| f(g))))
    }

    fn with_mut<R>(&mut self, f: impl FnOnce(&mut Contour) -> R) -> Result<R, ProxyError> {
        let ContourProxy { inner, idx, py_id } = self;
        let result = inner.with_mut(|contours| match contours.get_mut(idx.get()) {
            Some(c) if c.py_id == *py_id => Some(f(c)),
            _ => match contours.iter_mut().enumerate().find(|(_, c)| c.py_id == *py_id) {
                Some((i, c)) => {
                    idx.set(i);
                    Some(f(c))
                }
                None => None,
            },
        })?;

        match result {
            Some(thing) => Ok(thing),
            None => Err(ProxyError::MissingContour(self.clone())),
        }
    }
}

seq_proxy!(PointsProxy, ContourProxy, PointProxy, points, ContourPoint);
seq_proxy_iter!(PointsIter, PointsProxy, PointProxy);
proxy_eq!(PointsProxy);

#[pyclass]
#[derive(Debug, Clone)]
pub struct PointProxy {
    pub(crate) inner: ContourProxy,
    pub(crate) idx: Cell<usize>,
    py_id: PyId,
}

impl PointProxy {
    fn new(inner: PointsProxy, idx: usize, py_id: PyId) -> Self {
        PointProxy { inner: inner.inner, idx: Cell::new(idx), py_id }
    }

    fn with<R>(&self, f: impl FnOnce(&ContourPoint) -> R) -> Result<R, ProxyError> {
        flatten!(self.inner.with(|c| match c.points.get(self.idx.get()) {
            Some(pt) if pt.py_id == self.py_id => Some(pt),
            _ => match c.points.iter().enumerate().find(|(_, pt)| pt.py_id == self.py_id) {
                Some((i, pt)) => {
                    self.idx.set(i);
                    Some(pt)
                }
                None => None,
            },
        }
        .ok_or_else(|| ProxyError::MissingPoint(self.clone()))
        .map(|g| f(g))))
    }

    fn with_mut<R>(&mut self, f: impl FnOnce(&mut ContourPoint) -> R) -> Result<R, ProxyError> {
        let PointProxy { inner: contour, py_id, idx } = self;
        let result = contour.with_mut(|c| match c.points.get_mut(idx.get()) {
            Some(pt) if pt.py_id == *py_id => Some(f(pt)),
            _ => match c.points.iter_mut().enumerate().find(|(_, pt)| pt.py_id == *py_id) {
                Some((i, pt)) => {
                    idx.set(i);
                    Some(f(pt))
                }
                None => None,
            },
        })?;

        match result {
            Some(thing) => Ok(thing),
            None => Err(ProxyError::MissingPoint(self.clone())),
        }
    }
}

#[pymethods]
impl PointProxy {
    #[getter]
    fn get_x(&self) -> PyResult<f32> {
        self.with(|p| p.x).map_err(Into::into)
    }

    #[setter]
    fn set_x(&mut self, x: f32) -> PyResult<()> {
        self.with_mut(|p| p.x = x).map_err(Into::into)
    }

    #[getter]
    fn get_y(&self) -> PyResult<f32> {
        self.with(|p| p.y).map_err(Into::into)
    }

    #[setter]
    fn set_y(&mut self, y: f32) -> PyResult<()> {
        self.with_mut(|p| p.y = y).map_err(Into::into)
    }

    fn py_eq(&self, other: PyRef<PointProxy>) -> PyResult<bool> {
        let other: &PointProxy = &*other;
        flatten!(self.with(|p| other.with(|p2| p == p2))).map_err(Into::into)
    }
}

proxy_eq!(PointProxy);
