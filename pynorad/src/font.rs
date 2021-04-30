use std::collections::HashSet;
use std::ops::{Deref, DerefMut};
use std::sync::{Arc, RwLock};

use super::guideline::GuidelinesProxy;
use super::PyLib;
use norad::Font;
use pyo3::{
    prelude::*,
    types::{PyType, PyUnicode},
    PyRef,
};

use super::{
    LayerIter, PyAnchor, PyComponent, PyContour, PyFontInfo, PyGuideline, PyLayer, PyPoint,
};

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
            write!(f, "    {}: {} items", layer.name(), layer.len())?;
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
    fn from_layers(_cls: &PyType, layers: Vec<PyLayer>) -> PyResult<Self> {
        let layers = layers
            .into_iter()
            .map(|l| l.with(|layer| layer.to_owned()))
            .collect::<Result<_, _>>()?;
        Ok(Font::from_layers(layers).into())
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
        self.read().layers.iter().map(|l| l.name().to_string()).collect()
    }

    fn layer_order(&self) -> Vec<String> {
        self.read().layers.iter().map(|l| l.name().to_string()).collect()
    }

    fn deep_copy(&self) -> Self {
        let inner = Font::deep_clone(&self.read());
        inner.into()
    }

    fn new_layer(&mut self, layer_name: &str) -> PyResult<PyLayer> {
        let layer_name: Arc<str> = layer_name.into();
        self.write().layers.new_layer(&layer_name).map_err(super::error_to_py)?;
        Ok(PyLayer::proxy(self.clone(), layer_name))
    }

    fn rename_layer(&mut self, old: &str, new: &str, overwrite: bool) -> PyResult<()> {
        self.write().layers.rename_layer(old, new, overwrite).map_err(super::error_to_py)
    }

    fn iter_layers(&self) -> LayerIter {
        LayerIter { font: self.clone(), ix: 0 }
    }

    fn default_layer(&self) -> PyLayer {
        let layer_name = self.read().default_layer().name().clone();
        PyLayer::proxy(self.clone(), layer_name)
    }

    fn get_layer(&self, name: &str) -> Option<PyLayer> {
        self.read().layers.get(name).map(|l| PyLayer::proxy(self.clone(), l.name().clone()))
    }

    fn contains(&self, layer_name: &str) -> bool {
        self.read().layers.get(layer_name).is_some()
    }

    fn append_guideline(&mut self, guideline: PyRef<PyGuideline>) -> PyResult<()> {
        let guideline = (&*guideline).with(|g| g.to_owned())?;
        self.write().guidelines_mut().push(guideline);
        Ok(())
    }

    fn guidelines(&self) -> GuidelinesProxy {
        self.fontinfo().get_guidelines()
    }

    fn lib(&self) -> PyLib {
        self.clone().into()
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

    fn replace_guidelines(&mut self, guidelines: Vec<PyRefMut<PyGuideline>>) -> PyResult<()> {
        self.fontinfo().set_guidelines(guidelines)
    }

    fn fontinfo(&self) -> PyFontInfo {
        PyFontInfo::proxy(self.clone())
    }
}

impl From<Font> for PyFont {
    fn from(src: Font) -> PyFont {
        PyFont { inner: Arc::new(RwLock::new(src)) }
    }
}

#[derive(FromPyObject)]
pub enum HasObjectLib {
    Contour(PyContour),
    Component(PyComponent),
    Anchor(PyAnchor),
    Guideline(PyGuideline),
    Point(PyPoint),
}
