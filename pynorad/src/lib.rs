use pyo3::{exceptions, prelude::*, PyErr};
use std::sync::Arc;

mod font;
mod fontinfo;
mod glyph;
mod guideline;
mod layer;
#[macro_use]
mod util;
mod lib_object;

pub use font::PyFont;
pub use fontinfo::PyFontInfo;
pub use glyph::{
    AnchorProxy, ComponentProxy, ContourProxy, GlyphGuidelineProxy, PointProxy, PointsIter,
    PointsProxy, PyAnchor, PyComponent, PyContour, PyGlyph, PyPoint, PyPointPen,
};
pub use guideline::PyGuideline;
pub use layer::{GlyphIter, LayerIter, PyLayer};
pub use lib_object::PyLib;

pub(crate) static DEFAULT_LAYER_NAME: &str = "public.default";

#[pymodule]
fn pynorad(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_class::<PyFont>()?;
    m.add_class::<PyLayer>()?;
    m.add_class::<PyGlyph>()?;
    m.add_class::<PyAnchor>()?;
    m.add_class::<PyPoint>()?;
    m.add_class::<PyContour>()?;
    m.add_class::<PyComponent>()?;
    m.add_class::<PyPointPen>()?;
    m.add_class::<PyGuideline>()?;
    m.add_class::<PyFontInfo>()?;
    Ok(())
}

//FIXME: more nuanced error mapping
pub(crate) fn error_to_py(error: norad::Error) -> PyErr {
    match error {
        norad::Error::DuplicateGlyph { .. } => exceptions::PyKeyError::new_err(error.to_string()),
        _ => exceptions::PyRuntimeError::new_err(error.to_string()),
    }
}

#[derive(Debug, Clone)]
pub enum ProxyError {
    MissingLayer(Arc<str>),
    MissingGlyph(PyGlyph),
    MissingContour(ContourProxy),
    MissingComponent(ComponentProxy),
    MissingPoint(PointProxy),
    MissingAnchor(AnchorProxy),
    MissingGlobalGuideline,
    MissingGlyphGuideline(GlyphGuidelineProxy),
    MissingLayerGuideline(Arc<str>),
}

impl std::fmt::Display for ProxyError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            ProxyError::MissingLayer(layer) => write!(f, "Layer '{}' no longer exists.", layer),
            ProxyError::MissingGlyph(glyph) => {
                write!(f, "No glyph '{}' in layer '{}'", glyph.name, glyph.layer_name())
            }
            ProxyError::MissingContour(contour) => {
                write!(
                    f,
                    "No contour {} in glyph '{}', layer '{}'",
                    contour.idx.get(),
                    contour.inner.inner.name,
                    contour.inner.inner.layer_name()
                )
            }
            ProxyError::MissingComponent(_) => write!(f, "Missing component"),
            ProxyError::MissingAnchor(_) => write!(f, "Missing anchor"),
            //FIXME: figure out these errors again
            ProxyError::MissingPoint(point) => write!(
                f,
                "Missing point",
                //"No point {} in contour {}, glyph '{}', layer '{}'",
                //point.idx.get(),
                //point.inner.inner.idx.get(),
                //point.inner.inner.inner.inner.name,
                //point.inner.inner.inner.inner.layer_name(),
            ),
            ProxyError::MissingGlobalGuideline => write!(f, "Missing global Guideline"),
            ProxyError::MissingGlyphGuideline(_) => write!(f, "Missing glyph Guideline"),
            ProxyError::MissingLayerGuideline(layer) => {
                write!(f, "Missing Guideline in layer '{}'", layer)
            }
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
