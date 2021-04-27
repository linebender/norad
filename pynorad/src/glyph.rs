use std::sync::{Arc, Mutex, RwLock};

use norad::{Component, Contour, ContourPoint, Glyph, GlyphName, PointType, PyId};
use pyo3::{
    exceptions,
    prelude::*,
    types::{PyDict, PyType},
    PyRef, PySequenceProtocol,
};

use super::{
    flatten, proxy_eq, proxy_or_concrete, seq_proxy, seq_proxy_iter, seq_proxy_member, util,
    ProxyError, PyLayer,
};

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
        let typ = util::decode_point_type(typ);
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

seq_proxy!(PyGlyph, components, ComponentsProxy, PyComponent, Component);
seq_proxy_member!(ComponentsProxy, PyComponent, ComponentProxy, Component, MissingComponent);
seq_proxy_iter!(ComponentsIter, ComponentsProxy, PyComponent);
proxy_eq!(ComponentsProxy);

seq_proxy!(PyGlyph, contours, ContoursProxy, PyContour, Contour);
seq_proxy_member!(ContoursProxy, PyContour, ContourProxy, Contour, MissingContour);
seq_proxy_iter!(ContoursIter, ContoursProxy, PyContour);
proxy_eq!(ContoursProxy);

#[pymethods]
impl PyContour {
    #[getter]
    fn points(&self) -> PointsProxy {
        PointsProxy { inner: self.clone() }
    }
}

seq_proxy!(PyContour, points, PointsProxy, PyPoint, ContourPoint);
seq_proxy_member!(PointsProxy, PyPoint, PointProxy, ContourPoint, MissingPoint);
seq_proxy_iter!(PointsIter, PointsProxy, PyPoint);
proxy_eq!(PointsProxy);

#[pymethods]
impl PyPoint {
    #[classmethod]
    fn concrete(
        _cls: &PyType,
        x: f32,
        y: f32,
        typ: u8,
        smooth: bool,
        name: Option<String>,
        identifier: Option<&str>,
    ) -> PyResult<Self> {
        let identifier = util::to_identifier(identifier)?;
        let typ = util::decode_point_type(typ);
        let point = ContourPoint::new(x, y, typ, smooth, name, identifier, None);
        Ok(point.into())
    }
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

    fn py_eq(&self, other: PyRef<PyPoint>) -> PyResult<bool> {
        let other: &PyPoint = &*other;
        flatten!(self.with(|p| other.with(|p2| p == p2))).map_err(Into::into)
    }
}

proxy_eq!(PyPoint);
