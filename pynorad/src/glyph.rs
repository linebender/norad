use std::convert::TryFrom;
use std::sync::{Arc, Mutex, RwLock};

use norad::{
    AffineTransform, Anchor, Component, Contour, ContourPoint, Glyph, GlyphName, Guideline, Image,
    Plist, PointType, PyId,
};
use pyo3::{
    exceptions,
    prelude::*,
    types::{PyDict, PyType},
    PyRef, PySequenceProtocol,
};

use super::font::HasObjectLib;
use super::{
    flatten, proxy_eq, proxy_or_concrete, proxy_property, seq_proxy, seq_proxy_iter,
    seq_proxy_member, util, ProxyError, PyGuideline, PyLayer, PyLib,
};

type AffineTuple = (f32, f32, f32, f32, f32, f32);

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
            GlyphProxy::Concrete(glyph) => Ok(f(&glyph.try_read().unwrap())),
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
            GlyphProxy::Concrete(glyph) => Ok(f(&mut glyph.try_write().unwrap())),
        }
    }
}

proxy_property!(PyGlyph, height, f32, get_height, set_height);
proxy_property!(PyGlyph, width, f32, get_width, set_width);

#[pyproto]
impl PySequenceProtocol for PyGlyph {
    fn __len__(&self) -> PyResult<usize> {
        self.with(|g| g.contours.len()).map_err(Into::into)
    }

    fn __getitem__(&'p self, idx: isize) -> PyResult<PyContour> {
        self.contours().__getitem__(idx)
    }

    fn __delitem__(&'p mut self, idx: isize) -> PyResult<()> {
        self.contours().__delitem__(idx)
    }
}

#[pymethods]
impl PyGlyph {
    #[classmethod]
    fn concrete(
        _cls: &PyType,
        name: &str,
        width: f32,
        height: f32,
        unicodes: Vec<u32>,
        contours: Vec<PyRef<PyContour>>,
        components: Vec<PyRef<PyComponent>>,
        anchors: Vec<PyRef<PyAnchor>>,
        guidelines: Vec<PyRef<PyGuideline>>,
    ) -> PyResult<Self> {
        let name: GlyphName = name.into();
        let mut glyph = Glyph::new_named(name.clone());
        glyph.width = width;
        glyph.height = height;
        glyph.codepoints = unicodes
            .into_iter()
            .map(char::try_from)
            .collect::<Result<_, _>>()
            .map_err(|e| exceptions::PyValueError::new_err(e.to_string()))?;
        glyph.contours =
            contours.iter().map(|c| c.with(|c| c.clone())).collect::<Result<_, _>>()?;
        glyph.components =
            components.iter().map(|c| c.with(|c| c.clone())).collect::<Result<_, _>>()?;
        glyph.anchors = anchors.iter().map(|c| c.with(|c| c.clone())).collect::<Result<_, _>>()?;
        glyph.guidelines =
            guidelines.iter().map(|c| c.with(|c| c.clone())).collect::<Result<_, _>>()?;
        let glyph = Arc::new(RwLock::new(glyph));
        Ok(PyGlyph { name, inner: GlyphProxy::Concrete(glyph) })
    }

    #[getter]
    fn contours(&self) -> ContoursProxy {
        ContoursProxy { inner: self.clone() }
    }

    #[setter]
    fn set_contours(&mut self, new: Vec<PyRef<PyContour>>) -> PyResult<()> {
        let contours: Vec<Contour> =
            new.into_iter().map(|g| g.with(|g| g.clone())).collect::<Result<_, _>>()?;
        self.with_mut(|glyph| glyph.contours = contours).map_err(Into::into)
    }

    #[name = "clearContours"]
    fn clear_contours(&mut self) -> PyResult<()> {
        self.with_mut(|g| g.contours.clear()).map_err(Into::into)
    }

    #[getter]
    fn components(&self) -> ComponentsProxy {
        ComponentsProxy { inner: self.clone() }
    }

    #[name = "clearComponents"]
    fn clear_components(&mut self) -> PyResult<()> {
        self.with_mut(|g| g.components.clear()).map_err(Into::into)
    }

    #[setter]
    fn set_components(&mut self, new: Vec<PyRef<PyComponent>>) -> PyResult<()> {
        let components: Vec<Component> =
            new.into_iter().map(|g| g.with(|g| g.clone())).collect::<Result<_, _>>()?;
        self.with_mut(|glyph| glyph.components = components).map_err(Into::into)
    }

    #[getter]
    fn anchors(&self) -> AnchorsProxy {
        AnchorsProxy { inner: self.clone() }
    }

    #[setter]
    fn set_anchors(&mut self, new: Vec<PyRef<PyAnchor>>) -> PyResult<()> {
        let anchors: Vec<Anchor> =
            new.into_iter().map(|g| g.with(|g| g.clone())).collect::<Result<_, _>>()?;
        self.with_mut(|glyph| glyph.anchors = anchors).map_err(Into::into)
    }

    #[name = "clearAnchors"]
    fn clear_anchors(&mut self) -> PyResult<()> {
        self.with_mut(|g| g.anchors.clear()).map_err(Into::into)
    }

    #[getter]
    fn guidelines(&self) -> GlyphGuidelinesProxy {
        GlyphGuidelinesProxy { inner: self.clone() }
    }

    #[setter]
    fn set_guidelines(&mut self, new: Vec<PyRef<PyGuideline>>) -> PyResult<()> {
        let guidelines: Vec<Guideline> =
            new.into_iter().map(|g| g.with(|g| g.clone())).collect::<Result<_, _>>()?;
        self.with_mut(|glyph| glyph.guidelines = guidelines).map_err(Into::into)
    }

    #[name = "clearGuidelines"]
    fn clear_guidelines(&mut self) -> PyResult<()> {
        self.with_mut(|g| g.guidelines.clear()).map_err(Into::into)
    }

    #[getter]
    fn unicodes(&self) -> PyResult<Vec<u32>> {
        self.with(|g| g.codepoints.iter().map(|c| *c as u32).collect()).map_err(Into::into)
    }

    #[setter]
    fn set_unicodes(&mut self, vals: Vec<u32>) -> PyResult<()> {
        let codepoints = vals
            .into_iter()
            .map(char::try_from)
            .collect::<Result<_, _>>()
            .map_err(|e| exceptions::PyValueError::new_err(e.to_string()))?;
        self.with_mut(|g| g.codepoints = codepoints).map_err(Into::into)
    }

    #[getter]
    fn name(&self) -> PyResult<Option<String>> {
        self.with(|g| if g.name.is_empty() { None } else { Some(g.name.to_string()) })
            .map_err(Into::into)
    }

    fn set_name(&mut self, new_name: &str) -> PyResult<()> {
        let new_name: GlyphName = new_name.into();
        self.with_mut(|g| g.name = new_name.clone())?;
        self.name = new_name;
        Ok(())
    }

    #[getter]
    fn image(&self) -> PyResult<Option<PyImage>> {
        if self.with(|g| g.image.is_some())? {
            Ok(Some(PyImage::proxy(self.clone())))
        } else {
            Ok(None)
        }
    }

    #[setter]
    fn set_image(&mut self, image: Option<PyRef<PyImage>>) -> PyResult<()> {
        let image: Option<Image> =
            image.map(|img| img.with(|img| img.clone())).transpose()?.flatten();
        self.with_mut(|g| g.image = image).map_err(Into::into)
    }

    #[getter(verticalOrigin)]
    fn get_vertical_origin(&self) -> PyResult<Option<f64>> {
        self.with(|g| g.lib.get("public.verticalOrigin").and_then(|v| v.as_real()))
            .map_err(Into::into)
    }

    #[setter(verticalOrigin)]
    fn set_vertical_origin(&mut self, val: Option<f64>) -> PyResult<()> {
        match val {
            Some(v) => {
                self.with_mut(|g| g.lib.insert("public.verticalOrigin".to_string(), v.into()))
            }
            None => self.with_mut(|g| g.lib.remove("public.verticalOrigin")),
        }?;
        Ok(())
    }

    #[getter]
    fn lib(&self) -> PyLib {
        self.clone().into()
    }

    #[setter]
    fn set_lib(&mut self, lib: crate::lib_object::RawValue) -> PyResult<()> {
        let lib: Plist = crate::lib_object::from_python(lib)?
            .into_dictionary()
            .ok_or_else(|| exceptions::PyValueError::new_err("lib must be a dictionary"))?;
        self.with_mut(|g| g.lib = lib).map_err(Into::into)
    }

    #[getter]
    fn note(&self) -> PyResult<Option<String>> {
        self.with(|g| g.note.clone()).map_err(Into::into)
    }

    #[setter]
    fn set_note(&mut self, note: Option<String>) -> PyResult<()> {
        self.with_mut(|g| g.note = note).map_err(Into::into)
    }

    #[name = "objectLib"]
    fn object_lib(&self, obj: HasObjectLib) -> PyResult<PyLib> {
        match obj {
            HasObjectLib::Point(p) => Ok(p.into()),
            HasObjectLib::Contour(p) => Ok(p.into()),
            HasObjectLib::Component(p) => Ok(p.into()),
            HasObjectLib::Anchor(p) => Ok(p.into()),
            HasObjectLib::Guideline(p) => Ok(p.into()),
        }
    }

    fn append_anchor(&mut self, anchor: PyRef<PyAnchor>) -> PyResult<()> {
        let anchor = anchor.with(|a| a.to_owned())?;
        self.with_mut(|g| g.anchors.push(anchor)).map_err(Into::into)
    }

    fn append_contour(&mut self, contour: PyRef<PyContour>) -> PyResult<()> {
        let contour = contour.with(|a| a.to_owned())?;
        self.with_mut(|g| g.contours.push(contour)).map_err(Into::into)
    }

    fn append_component(&mut self, component: PyRef<PyComponent>) -> PyResult<()> {
        let component = component.with(|a| a.to_owned())?;
        self.with_mut(|g| g.components.push(component)).map_err(Into::into)
    }

    fn append_guideline(&mut self, guideline: PyRef<PyGuideline>) -> PyResult<()> {
        let guideline = guideline.with(|a| a.to_owned())?;
        self.with_mut(|g| g.guidelines.push(guideline)).map_err(Into::into)
    }

    #[name = "r#move"]
    fn move_(&mut self, delta: (f32, f32)) -> PyResult<()> {
        self.with_mut(|g| {
            g.contours.iter_mut().for_each(|c| {
                c.points.iter_mut().for_each(|pt| {
                    pt.x += delta.0;
                    pt.y += delta.1;
                });
            });
            g.anchors.iter_mut().for_each(|a| {
                a.x += delta.0;
                a.y += delta.1;
            });
            g.components.iter_mut().for_each(|c| {
                c.transform.x_offset += delta.0;
                c.transform.y_offset += delta.1;
            });
        })
        .map_err(Into::into)
    }

    fn py_eq(&self, other: PyRef<PyGlyph>) -> PyResult<bool> {
        let other: &PyGlyph = &*other;
        flatten!(self.with(|p| other.with(|p2| p == p2))).map_err(Into::into)
    }

    #[name = "drawPoints"]
    fn draw_points(&self, pen: PyObject) -> PyResult<()> {
        flatten!(self
            .with(|glyph| {
                let gil = Python::acquire_gil();
                let py = gil.python();

                for c in &glyph.contours {
                    pen.call_method0(py, "beginPath")?;
                    for p in &c.points {
                        let coord = (p.x, p.y).to_object(py);
                        let d = PyDict::new(py);
                        d.set_item("segmentType", point_to_str(p.typ))?;
                        d.set_item("smooth", Some(p.smooth))?;
                        d.set_item("name", p.name.as_ref())?;
                        d.set_item("identifier", p.identifier().as_ref().map(|id| id.as_str()))?;
                        pen.call_method(py, "addPoint", (coord,), Some(d))?;
                    }
                    pen.call_method0(py, "endPath")?;
                }
                for c in &glyph.components {
                    let transform: kurbo::Affine = c.transform.into();
                    let transform = transform.as_coeffs();
                    pen.call_method1(
                        py,
                        "addComponent",
                        (c.base.to_object(py), transform.to_object(py)),
                    )?;
                }
                Ok(())
            })
            .map_err(Into::into))
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
            Some(Err(arc)) => arc.try_lock().unwrap().clone(),
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
        self.contour.as_mut().unwrap().try_lock().unwrap().points.push(point);
        Ok(())
    }

    fn add_component(
        &mut self,
        name: &str,
        xform: AffineTuple,
        identifier: Option<&str>,
    ) -> PyResult<()> {
        let identifier = util::to_identifier(identifier)?;
        let transform: kurbo::Affine = norad::AffineTransform::from(xform).into();
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

seq_proxy!(PyGlyph, contours, ContoursProxy, PyContour, Contour);
seq_proxy_member!(ContoursProxy, PyContour, ContourProxy, Contour, MissingContour);
seq_proxy_iter!(ContoursIter, ContoursProxy, PyContour);
proxy_eq!(PyContour);

#[pymethods]
impl PyContour {
    #[classmethod]
    fn concrete(
        _cls: &PyType,
        points: Vec<PyRef<PyPoint>>,
        identifier: Option<&str>,
    ) -> PyResult<Self> {
        let identifier = util::to_identifier(identifier)?;
        let points: Vec<_> =
            points.iter().map(|p| p.with(|pt| pt.to_owned())).collect::<Result<_, _>>()?;
        let contour = Contour::new(points, identifier, None);
        Ok(contour.into())
    }

    #[getter]
    fn points(&self) -> PointsProxy {
        PointsProxy { inner: self.clone() }
    }

    #[name = "r#move"]
    fn move_(&mut self, delta: (f32, f32)) -> PyResult<()> {
        self.with_mut(|c| {
            c.points.iter_mut().for_each(|pt| {
                pt.x += delta.0;
                pt.y += delta.1;
            })
        })
        .map_err(Into::into)
    }

    #[name = "drawPoints"]
    fn draw_points(&self, pen: PyObject) -> PyResult<()> {
        let c = self.with(|c| c.clone())?;
        let gil = Python::acquire_gil();
        let py = gil.python();
        pen.call_method0(py, "beginPath")?;
        for p in &c.points {
            let coord = (p.x, p.y).to_object(py);
            let d = PyDict::new(py);
            d.set_item("segmentType", point_to_str(p.typ))?;
            d.set_item("smooth", Some(p.smooth))?;
            d.set_item("name", p.name.as_ref())?;
            d.set_item("identifier", p.identifier().as_ref().map(|id| id.as_str()))?;
            pen.call_method(py, "addPoint", (coord,), Some(d))?;
        }
        pen.call_method0(py, "endPath")?;
        Ok(())
    }
}

// guidelines exist in multiple places so the code is a bit different.
seq_proxy!(PyGlyph, guidelines, GlyphGuidelinesProxy, PyGuideline, Guideline);
seq_proxy_member!(GlyphGuidelinesProxy, GlyphGuidelineProxy, Guideline, MissingGlyphGuideline);
seq_proxy_iter!(GuidelinesIter, GlyphGuidelinesProxy, PyGuideline);

#[pymethods]
impl PyGuideline {
    #[getter]
    fn get_name(&self) -> PyResult<Option<String>> {
        self.with(|g| g.name.clone()).map_err(Into::into)
    }

    #[setter]
    fn set_name(&mut self, name: Option<String>) -> PyResult<()> {
        self.with_mut(|g| g.name = name).map_err(Into::into)
    }
}

seq_proxy!(PyGlyph, components, ComponentsProxy, PyComponent, Component);
seq_proxy_member!(ComponentsProxy, PyComponent, ComponentProxy, Component, MissingComponent);
seq_proxy_iter!(ComponentsIter, ComponentsProxy, PyComponent);
proxy_eq!(PyComponent);
proxy_property!(PyComponent, transform, AffineTuple, get_transformation, set_transformation);
//proxy_property!(PyComponent, base, &str, get_baseGlyph, set_baseGlyph);

#[pymethods]
impl PyComponent {
    #[classmethod]
    fn concrete(
        _cls: &PyType,
        base: &str,
        xform: Option<AffineTuple>,
        identifier: Option<&str>,
    ) -> PyResult<Self> {
        let identifier = util::to_identifier(identifier)?;
        let transform = xform.map(AffineTransform::from).unwrap_or_default();
        let component = Component::new(base.into(), transform, identifier, None);
        Ok(component.into())
    }

    #[name = "r#move"]
    fn move_(&mut self, delta: (f32, f32)) -> PyResult<()> {
        self.with_mut(|c| {
            c.transform.x_offset += delta.0;
            c.transform.y_offset += delta.1;
        })
        .map_err(Into::into)
    }

    #[getter]
    fn get_base(&self) -> PyResult<String> {
        self.with(|c| c.base.to_string()).map_err(Into::into)
    }

    fn set_base(&mut self, name: &str) -> PyResult<()> {
        self.with_mut(|c| c.base = name.into()).map_err(Into::into)
    }

    #[getter]
    fn identifier(&self) -> PyResult<Option<String>> {
        self.with(|c| c.identifier().map(|id| id.as_str().to_owned())).map_err(Into::into)
    }
}

seq_proxy!(PyGlyph, anchors, AnchorsProxy, PyAnchor, Anchor);
seq_proxy_member!(AnchorsProxy, PyAnchor, AnchorProxy, Anchor, MissingAnchor);
seq_proxy_iter!(AnchorsIter, AnchorsProxy, PyAnchor);
proxy_eq!(PyAnchor);
proxy_property!(PyAnchor, x, f32, get_x, set_x);
proxy_property!(PyAnchor, y, f32, get_y, set_y);

#[pymethods]
impl PyAnchor {
    #[classmethod]
    fn concrete(
        _cls: &PyType,
        x: f32,
        y: f32,
        name: Option<String>,
        color: Option<&str>,
        identifier: Option<&str>,
    ) -> PyResult<Self> {
        let identifier = util::to_identifier(identifier)?;
        let color = util::to_color(color)?;
        let anchor = Anchor::new(x, y, name, color, identifier, None);
        Ok(anchor.into())
    }

    #[name = "r#move"]
    fn move_(&mut self, delta: (f32, f32)) -> PyResult<()> {
        self.with_mut(|c| {
            c.x += delta.0;
            c.y += delta.1;
        })
        .map_err(Into::into)
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
}

seq_proxy!(PyContour, points, PointsProxy, PyPoint, ContourPoint);
seq_proxy_member!(PointsProxy, PyPoint, PointProxy, ContourPoint, MissingPoint);
seq_proxy_iter!(PointsIter, PointsProxy, PyPoint);
proxy_eq!(PointsProxy);
proxy_eq!(PyPoint);
proxy_property!(PyPoint, x, f32, get_x, set_x);
proxy_property!(PyPoint, y, f32, get_y, set_y);

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

    fn py_eq(&self, other: PyRef<PyPoint>) -> PyResult<bool> {
        let other: &PyPoint = &*other;
        flatten!(self.with(|p| other.with(|p2| p == p2))).map_err(Into::into)
    }
}

#[pyclass]
#[derive(Debug, Clone)]
pub struct PyImage {
    inner: ImageProxy,
}

#[derive(Debug, Clone)]
enum ImageProxy {
    Glyph(PyGlyph),
    Concrete(Arc<Mutex<Image>>),
}

#[pymethods]
impl PyImage {
    #[classmethod]
    fn concrete(
        _cls: &PyType,
        file_name: Option<String>,
        color: Option<&str>,
        transform: Option<AffineTuple>,
    ) -> PyResult<Self> {
        let image = Image {
            file_name: file_name.unwrap_or_default().into(),
            color: util::to_color(color)?,
            transform: transform.map(AffineTransform::from).unwrap_or_default(),
        };

        Ok(PyImage { inner: ImageProxy::Concrete(Arc::new(Mutex::new(image))) })
    }
}

impl PyImage {
    pub(crate) fn proxy(glyph: PyGlyph) -> Self {
        PyImage { inner: ImageProxy::Glyph(glyph) }
    }

    pub(crate) fn with<R>(&self, f: impl FnOnce(&Image) -> R) -> Result<Option<R>, ProxyError> {
        match &self.inner {
            ImageProxy::Glyph(g) => g.with(|g| g.image.as_ref().map(f)).map_err(Into::into),
            ImageProxy::Concrete(img) => Ok(Some(f(&img.try_lock().unwrap()))),
        }
    }

    #[allow(dead_code)]
    pub(crate) fn with_mut<R>(
        &mut self,
        f: impl FnOnce(&mut Image) -> R,
    ) -> Result<Option<R>, ProxyError> {
        match &mut self.inner {
            ImageProxy::Glyph(g) => g.with_mut(|g| g.image.as_mut().map(f)).map_err(Into::into),
            ImageProxy::Concrete(img) => Ok(Some(f(&mut img.try_lock().unwrap()))),
        }
    }
}
