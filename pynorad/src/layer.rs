use crate::font::PyFont;
use crate::glyph::GlyphProxy;
use crate::ProxyError;

use std::sync::Arc;

use norad::{GlyphName, Layer};
use pyo3::{prelude::*, PyIterProtocol, PyRef};

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

    fn __next__(mut slf: PyRefMut<Self>) -> Option<LayerProxy> {
        let index = slf.ix;
        slf.ix += 1;
        match slf.font.read().layers.get(index).map(|l| l.name.clone()) {
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
    pub font: PyFont,
    pub name: Arc<str>,
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
        let layer_same = self.font.read().find_layer(|info| info.name == self.name)
            == other.font.read().find_layer(|info| info.name == other.name);

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
    pub fn with<R>(&self, f: impl FnOnce(&Layer) -> R) -> Result<R, ProxyError> {
        self.font
            .read()
            .find_layer(|l| l.name == self.name)
            .map(f)
            .ok_or_else(|| ProxyError::MissingLayer(self.name.clone()))
    }

    pub fn with_mut<R>(&self, f: impl FnOnce(&mut Layer) -> R) -> Result<R, ProxyError> {
        self.font
            .write()
            .find_layer_mut(|l| l.name == self.name)
            .map(f)
            .ok_or_else(|| ProxyError::MissingLayer(self.name.clone()))
    }
}
