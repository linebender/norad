use std::cell::Cell;
use std::collections::HashSet;
use std::sync::{Arc, RwLock};

use norad::{Contour, ContourPoint, Font, Glyph, GlyphName, Layer, LayerInfo, PyId};
use pyo3::{
    class::basic::CompareOp,
    exceptions,
    prelude::*,
    types::{PyType, PyUnicode},
    PyErr, PyIterProtocol, PyObjectProtocol, PyRef, PySequenceProtocol,
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
