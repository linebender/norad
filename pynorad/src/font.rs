use std::collections::HashSet;
use std::ops::{Deref, DerefMut};
use std::sync::{Arc, RwLock};

use norad::{Font, LayerInfo};
use pyo3::{
    exceptions,
    prelude::*,
    types::{PyType, PyUnicode},
    PyRef,
};

use super::{LayerIter, LayerProxy, DEFAULT_LAYER_NAME};

#[pyclass]
#[derive(Clone)]
pub struct PyFont {
    pub(crate) inner: Arc<RwLock<Font>>,
}

impl PyFont {
    pub(crate) fn read<'a>(&'a self) -> impl Deref<Target = Font> + 'a {
        self.inner.read().unwrap()
    }

    pub(crate) fn write<'a>(&'a self) -> impl DerefMut<Target = Font> + 'a {
        self.inner.write().unwrap()
    }
}

impl std::fmt::Debug for PyFont {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "PyFont({:?})", &Arc::as_ptr(&self.inner))?;
        let font = self.read();
        for layer in font.iter_layers() {
            write!(f, "    {}: {} items", &layer.name, layer.layer.len())?;
        }
        Ok(())
    }
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
        Font::load(s).map(Into::into).map_err(super::error_to_py)
    }

    fn save(&self, path: &PyUnicode) -> PyResult<()> {
        let path: String = path.extract()?;
        self.read().save(&path).map_err(super::error_to_py)
    }

    fn py_eq(&self, other: PyRef<PyFont>) -> PyResult<bool> {
        let other: &PyFont = &*other;
        let ptr_eq = Arc::ptr_eq(&self.inner, &other.inner);
        Ok(ptr_eq || other.read().eq(&self.read()))
    }

    fn layer_eq(&self, other: PyRef<PyFont>) -> PyResult<bool> {
        let other: &PyFont = &*other;
        let ptr_eq = Arc::ptr_eq(&self.inner, &other.inner);
        Ok(ptr_eq || other.read().layers.eq(&self.read().layers))
    }

    fn layer_count(&self) -> usize {
        self.read().layers.len()
    }

    fn layer_names(&self) -> HashSet<String> {
        self.read().layers.iter().map(|l| l.name.to_string()).collect()
    }

    fn layer_order(&self) -> Vec<String> {
        self.read().layers.iter().map(|l| l.name.to_string()).collect()
    }

    fn deep_copy(&self) -> Self {
        let inner = Font::deep_clone(&self.read());
        inner.into()
    }

    fn new_layer(&mut self, layer_name: &PyUnicode) -> PyResult<LayerProxy> {
        let layer_name: Arc<str> = layer_name.extract::<String>()?.into();
        let info = LayerInfo::new(layer_name.clone());
        self.write().layers.push(info);
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
        self.read()
            .layers
            .iter()
            .find(|l| l.name.as_ref() == name)
            .map(|l| LayerProxy { name: l.name.clone(), font: self.clone() })
    }
}

impl From<Font> for PyFont {
    fn from(src: Font) -> PyFont {
        PyFont { inner: Arc::new(RwLock::new(src)) }
    }
}
