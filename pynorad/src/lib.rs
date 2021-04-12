use std::collections::HashSet;
use std::sync::{Arc, RwLock};

use norad::{Contour, ContourPoint, Font, Glyph, GlyphName, Layer, LayerInfo};
use pyo3::{
    exceptions,
    prelude::*,
    types::{PyType, PyUnicode},
    PyErr, PyIterProtocol, PyRef, PySequenceProtocol,
};

static DEFAULT_LAYER_NAME: &str = "public.default";

#[pymodule]
fn pynorad(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_class::<PyFont>()?;
    m.add_class::<LayerProxy>()?;
    m.add_class::<GlyphProxy>()?;
    Ok(())
}

#[pyclass]
#[derive(Debug, Clone)]
pub struct PyFont {
    inner: Arc<RwLock<Font>>,
}

#[pymethods]
impl PyFont {
    #[new]
    fn new() -> Self {
        Font::default().into()
    }

    #[classmethod]
    fn load(_cls: &PyType, path: &PyUnicode) -> PyResult<Self> {
        let s: String = path.extract()?;
        //FIXME: not the right exception type
        Font::load(s).map(Into::into).map_err(error_to_py)
    }

    fn save(&self, path: &PyUnicode) -> PyResult<()> {
        let path: String = path.extract()?;
        self.inner.read().unwrap().save(&path).map_err(error_to_py)
    }

    fn py_eq(&self, other: PyRef<PyFont>) -> PyResult<bool> {
        let other: &PyFont = &*other;
        let ptr_eq = Arc::ptr_eq(&self.inner, &other.inner);
        Ok(ptr_eq || other.inner.read().unwrap().eq(&self.inner.read().unwrap()))
    }

    fn layer_eq(&self, other: PyRef<PyFont>) -> PyResult<bool> {
        let other: &PyFont = &*other;
        let ptr_eq = Arc::ptr_eq(&self.inner, &other.inner);
        Ok(ptr_eq || other.inner.read().unwrap().layers.eq(&self.inner.read().unwrap().layers))
    }

    fn layer_count(&self) -> usize {
        self.inner.read().unwrap().layers.len()
    }

    fn layer_names(&self) -> HashSet<String> {
        self.inner.read().unwrap().layers.iter().map(|l| l.name.to_string()).collect()
    }

    fn layer_order(&self) -> Vec<String> {
        self.inner.read().unwrap().layers.iter().map(|l| l.name.to_string()).collect()
    }

    fn deep_copy(&self) -> Self {
        let inner = Font::clone(&self.inner.read().unwrap());
        inner.into()
    }

    fn new_layer(&mut self, layer_name: &PyUnicode) -> PyResult<LayerProxy> {
        let layer_name: Arc<str> = layer_name.extract::<String>()?.into();
        let info = LayerInfo::new(layer_name.clone());
        self.inner.write().unwrap().layers.push(info);
        Ok(LayerProxy { font: self.clone(), name: layer_name })
    }

    fn iter_layers(&self) -> LayerIter {
        LayerIter { font: self.clone(), ix: 0 }
    }

    fn default_layer(&self) -> PyResult<LayerProxy> {
        self.get_layer(DEFAULT_LAYER_NAME)
            .ok_or_else(|| exceptions::PyRuntimeError::new_err("Missing default layer"))
    }

    fn get_layer(&self, name: &str) -> Option<LayerProxy> {
        let font = self.inner.read().unwrap();
        font.layers
            .iter()
            .find(|l| l.name.as_ref() == name)
            .map(|l| LayerProxy { name: l.name.clone(), font: self.clone() })
    }
}

#[pyclass]
pub struct LayerIter {
    font: PyFont,
    ix: usize,
}

#[pyproto]
impl PyIterProtocol for LayerIter {
    fn __iter__(slf: PyRef<'p, Self>) -> PyRef<'p, Self> {
        slf
    }

    fn __next__(mut slf: PyRefMut<Self>) -> Option<LayerProxy> {
        let index = slf.ix;
        slf.ix += 1;
        match slf.font.inner.read().unwrap().layers.get(index).map(|l| l.name.clone()) {
            Some(layer_name) => Some(LayerProxy { font: slf.font.clone(), name: layer_name }),
            None => None,
        }
    }
}

#[pyclass]
pub struct GlyphIter {
    layer: LayerProxy,
    glyphs: Vec<GlyphName>,
    ix: usize,
}

#[pyproto]
impl PyIterProtocol for GlyphIter {
    fn __iter__(slf: PyRef<'p, Self>) -> PyRef<'p, Self> {
        slf
    }

    fn __next__(mut slf: PyRefMut<Self>) -> Option<GlyphProxy> {
        let index = slf.ix;
        slf.ix += 1;
        slf.glyphs.get(index).cloned().map(|glyph| GlyphProxy { layer: slf.layer.clone(), glyph })
    }
}

#[pyclass]
#[derive(Clone)]
pub struct LayerProxy {
    font: PyFont,
    name: Arc<str>,
}

impl std::fmt::Debug for LayerProxy {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.debug_struct("LayerProxy")
            .field("font", &Arc::as_ptr(&self.font.inner))
            .field("name", &self.name)
            .finish()
    }
}

#[pymethods]
impl LayerProxy {
    #[getter]
    fn name(&self) -> &str {
        &self.name
    }

    fn len(&self) -> usize {
        self.with(|layer| layer.len()).unwrap_or(0)
    }

    fn py_eq(&self, other: PyRef<LayerProxy>) -> PyResult<bool> {
        let other: &LayerProxy = &*other;
        if Arc::ptr_eq(&self.font.inner, &other.font.inner) && Arc::ptr_eq(&self.name, &other.name)
        {
            return Ok(true);
        } else if other.name != self.name {
            return Ok(false);
        }
        let font1 = self.font.inner.read().unwrap();
        let font2 = other.font.inner.read().unwrap();
        let layer_same = font1.find_layer(|info| info.name == self.name)
            == font2.find_layer(|info| info.name == other.name);

        Ok(layer_same)
    }

    fn iter_glyphs(&self) -> PyResult<GlyphIter> {
        self.with(|layer| layer.iter_contents().map(|glyph| glyph.name.clone()).collect::<Vec<_>>())
            .map_err(Into::into)
            .map(|glyphs| GlyphIter { glyphs, layer: self.clone(), ix: 0 })
    }

    fn glyph(&self, name: &str) -> PyResult<Option<GlyphProxy>> {
        self.with(|layer| {
            layer
                .get_glyph(name) //.map(
                //let font = self.font.inner.read().unwrap();
                //font.find_layer(|l| l.name == self.name).and_then(|l| l.get_glyph(name))
                .map(|glyph| GlyphProxy { layer: self.clone(), glyph: glyph.name.clone() })
        })
        .map_err(Into::into)
    }
}

impl LayerProxy {
    fn with<R>(&self, f: impl FnOnce(&Layer) -> R) -> Result<R, ProxyError> {
        let lock = self.font.inner.read().unwrap();
        lock.find_layer(|l| l.name == self.name)
            .map(f)
            .ok_or_else(|| ProxyError::MissingLayer(self.name.clone()))
    }

    fn with_mut<R>(&self, f: impl FnOnce(&mut Layer) -> R) -> Result<R, ProxyError> {
        let mut lock = self.font.inner.write().unwrap();
        lock.find_layer_mut(|l| l.name == self.name)
            .map(f)
            .ok_or_else(|| ProxyError::MissingLayer(self.name.clone()))
    }
}

#[pyclass]
#[derive(Debug, Clone)]
pub struct GlyphProxy {
    layer: LayerProxy,
    //layer: Arc<str>,
    glyph: GlyphName,
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

        if self.glyph.with(|g| g.contours.get(idx).is_some()).unwrap_or(false) {
            Some(ContourProxy { glyph: self.glyph.clone(), contour: idx })
        } else {
            None
        }
    }
}

#[pyclass]
#[derive(Debug, Clone)]
pub struct ContourProxy {
    glyph: GlyphProxy,
    contour: usize,
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
        flatten!(self.glyph.with(|g| g
            .contours
            .get(self.contour)
            .ok_or_else(|| ProxyError::MissingContour {
                layer: self.glyph.layer.name.clone(),
                glyph: self.glyph.glyph.clone(),
                contour: self.contour,
            })
            .map(|g| f(g))))
    }

    fn with_mut<R>(&self, f: impl FnOnce(&mut Contour) -> R) -> Result<R, ProxyError> {
        flatten!(self.glyph.with_mut(|g| g
            .contours
            .get_mut(self.contour)
            .ok_or_else(|| ProxyError::MissingContour {
                layer: self.glyph.layer.name.clone(),
                glyph: self.glyph.glyph.clone(),
                contour: self.contour,
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
    contour: ContourProxy,
    len: usize,
    ix: usize,
}

#[pymethods]
impl PointsProxy {
    fn iter_points(&self) -> PointsIter {
        PointsIter { contour: self.contour.clone(), len: self.__len__(), ix: 0 }
    }
}

#[pyproto]
impl PySequenceProtocol for PointsProxy {
    fn __len__(&self) -> usize {
        self.contour.with(|c| c.points.len()).unwrap_or(0)
    }

    fn __getitem__(&'p self, idx: isize) -> Option<PointProxy> {
        let idx: usize = if idx.is_negative() {
            self.__len__().checked_sub(idx.abs() as usize)?
        } else {
            idx as usize
        };

        if self.contour.with(|c| c.points.get(idx).is_some()).unwrap_or(false) {
            Some(PointProxy { contour: self.contour.clone(), point: idx })
        } else {
            None
        }
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
        if index < slf.len {
            Some(PointProxy { contour: slf.contour.clone(), point: index })
        } else {
            None
        }
    }
}

#[pyclass]
pub struct PointProxy {
    contour: ContourProxy,
    point: usize,
}

impl PointProxy {
    fn with<R>(&self, f: impl FnOnce(&ContourPoint) -> R) -> Result<R, ProxyError> {
        flatten!(self.contour.with(|c| c
            .points
            .get(self.point)
            .ok_or_else(|| ProxyError::MissingPoint {
                layer: self.contour.glyph.layer.name.clone(),
                glyph: self.contour.glyph.glyph.clone(),
                contour: self.contour.contour,
                point: self.point
            })
            .map(|g| f(g))))
    }

    fn with_mut<R>(&self, f: impl FnOnce(&mut ContourPoint) -> R) -> Result<R, ProxyError> {
        flatten!(self.contour.with_mut(|c| c
            .points
            .get_mut(self.point)
            .ok_or_else(|| ProxyError::MissingPoint {
                layer: self.contour.glyph.layer.name.clone(),
                glyph: self.contour.glyph.glyph.clone(),
                contour: self.contour.contour,
                point: self.point
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
}

impl From<Font> for PyFont {
    fn from(src: Font) -> PyFont {
        PyFont { inner: Arc::new(RwLock::new(src)) }
    }
}

//FIXME: more nuanced error mapping
fn error_to_py(error: norad::Error) -> PyErr {
    exceptions::PyRuntimeError::new_err(error.to_string())
}

#[derive(Debug, Clone)]
enum ProxyError {
    MissingLayer(Arc<str>),
    MissingGlyph { layer: Arc<str>, glyph: Arc<str> },
    MissingContour { layer: Arc<str>, glyph: Arc<str>, contour: usize },
    MissingPoint { layer: Arc<str>, glyph: Arc<str>, contour: usize, point: usize },
}

impl std::fmt::Display for ProxyError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            ProxyError::MissingLayer(layer) => write!(f, "Layer '{}' no longer exists.", layer),
            ProxyError::MissingGlyph { layer, glyph } => {
                write!(f, "No glyph '{}' in layer '{}'", glyph, layer)
            }
            ProxyError::MissingContour { layer, glyph, contour } => {
                write!(f, "No contour {} in glyph '{}', layer '{}'", contour, glyph, layer)
            }
            ProxyError::MissingPoint { layer, glyph, contour, point } => write!(
                f,
                "No point {} in contour {}, glyph '{}', layer '{}'",
                point, contour, glyph, layer
            ),
        }
    }
}

impl From<ProxyError> for PyErr {
    fn from(src: ProxyError) -> PyErr {
        exceptions::PyRuntimeError::new_err(src.to_string())
    }
}

// acts like a dictionary of str: layer
// len()
// iterator over layers
// __contains__
// __get__
// __del__
// .defaultLayer
// newLayer(name, **kwargs) create and return a layer
// renameGlyph(name, newName, overwrite) rename across all glyphs. if 'overwrite' is false,
// raises an exception if the new name already exists
