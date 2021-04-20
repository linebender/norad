use crate::font::PyFont;
use crate::glyph::GlyphProxy;
use crate::ProxyError;

use std::sync::{Arc, RwLock};

use norad::{GlyphName, Layer};
use pyo3::{prelude::*, types::PyType, PyIterProtocol, PyRef};

#[pyclass]
#[derive(Clone, Debug)]
pub struct PyLayer {
    inner: LayerProxy,
}

#[derive(Clone)]
enum LayerProxy {
    Font { font: PyFont, layer_name: Arc<str> },
    Concrete { layer: Arc<RwLock<Layer>>, layer_name: Arc<str> },
}

impl std::fmt::Debug for LayerProxy {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            LayerProxy::Font { font, layer_name } => f
                .debug_struct("LayerProxy::Font")
                .field("font", &Arc::as_ptr(&font.inner))
                .field("name", &layer_name)
                .finish(),
            LayerProxy::Concrete { layer_name, .. } => {
                f.debug_struct("LayerProxy::Concrete").field("layer", &layer_name).finish()
            }
        }
    }
}

#[pymethods]
impl PyLayer {
    #[classmethod]
    fn concrete(_cls: &PyType, name: &str) -> PyResult<Self> {
        let layer_name: Arc<str> = name.into();
        let layer = Arc::new(RwLock::new(Layer::new(layer_name.clone(), None)));
        Ok(PyLayer { inner: LayerProxy::Concrete { layer, layer_name } })
    }

    #[getter]
    pub fn name(&self) -> &str {
        match &self.inner {
            LayerProxy::Font { layer_name, .. } => &layer_name,
            LayerProxy::Concrete { layer_name, .. } => &layer_name,
        }
    }

    fn len(&self) -> usize {
        self.with(|layer| layer.len()).unwrap_or(0)
    }

    fn py_eq(&self, other: PyRef<PyLayer>) -> PyResult<bool> {
        let other: &PyLayer = &*other;
        if let Some(eq) = self.inner.ptr_eq(&other.inner) {
            return Ok(eq);
        }

        super::flatten!(self.with(|l1| other.with(|l2| l1 == l2))).map_err(Into::into)
    }

    fn iter_glyphs(&self) -> PyResult<GlyphIter> {
        self.with(|layer| layer.iter_contents().map(|glyph| glyph.name.clone()).collect::<Vec<_>>())
            .map_err(Into::into)
            .map(|glyphs| GlyphIter { glyphs, layer: self.clone(), ix: 0 })
    }

    fn glyph(&self, name: &str) -> PyResult<Option<GlyphProxy>> {
        self.with(|layer| {
            layer
                .get_glyph(name)
                .map(|glyph| GlyphProxy { layer: self.clone(), glyph: glyph.name.clone() })
        })
        .map_err(Into::into)
    }
}

impl PyLayer {
    pub fn proxy(font: PyFont, layer_name: Arc<str>) -> Self {
        PyLayer { inner: LayerProxy::Font { font, layer_name } }
    }

    pub fn with<R>(&self, f: impl FnOnce(&Layer) -> R) -> Result<R, ProxyError> {
        match &self.inner {
            LayerProxy::Font { font, layer_name } => font
                .read()
                .layers
                .get(&layer_name)
                .map(f)
                .ok_or_else(|| ProxyError::MissingLayer(layer_name.clone())),
            LayerProxy::Concrete { layer, .. } => Ok(f(&layer.read().unwrap())),
        }
    }

    pub fn with_mut<R>(&self, f: impl FnOnce(&mut Layer) -> R) -> Result<R, ProxyError> {
        match &self.inner {
            LayerProxy::Font { font, layer_name } => font
                .write()
                .layers
                .get_mut(&layer_name)
                .map(f)
                .ok_or_else(|| ProxyError::MissingLayer(layer_name.clone())),
            LayerProxy::Concrete { layer, .. } => Ok(f(&mut layer.write().unwrap())),
        }
    }
}

impl LayerProxy {
    fn ptr_eq(&self, other: &LayerProxy) -> Option<bool> {
        match (self, other) {
            (
                LayerProxy::Concrete { layer: layer1, .. },
                LayerProxy::Concrete { layer: layer2, .. },
            ) => {
                if Arc::ptr_eq(layer1, layer2) {
                    return Some(true);
                }
            }
            (
                LayerProxy::Font { font: font1, layer_name: name1 },
                LayerProxy::Font { font: font2, layer_name: name2 },
            ) => {
                if Arc::ptr_eq(&font1.inner, &font2.inner) && Arc::ptr_eq(&name1, &name2) {
                    return Some(true);
                } else if name1 != name2 {
                    return Some(false);
                }
            }
            _ => (),
        };
        None
    }
}

#[pyclass]
pub struct LayerIter {
    pub(crate) font: PyFont,
    pub(crate) ix: usize,
}

#[pyproto]
impl PyIterProtocol for LayerIter {
    fn __iter__(slf: PyRef<'p, Self>) -> PyRef<'p, Self> {
        slf
    }

    fn __next__(mut slf: PyRefMut<Self>) -> Option<PyLayer> {
        let index = slf.ix;
        slf.ix += 1;
        match slf.font.read().layers.layers().get(index).map(|l| l.name().clone()) {
            Some(layer_name) => {
                Some(PyLayer { inner: LayerProxy::Font { font: slf.font.clone(), layer_name } })
            }
            None => None,
        }
    }
}

#[pyclass]
pub struct GlyphIter {
    layer: PyLayer,
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
