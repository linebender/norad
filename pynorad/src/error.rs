use pyo3::{exceptions, PyErr};
use std::sync::Arc;

use super::PyGlyph;

/// Errors that can occur when python tries to access a proxy object that
/// no longer exists.
///
/// This could happen, for instance, if there is a python reference to a glyph
/// that was retrieved through a layer, but the layer was subsequently deleted.
#[derive(Debug, Clone)]
pub enum ProxyError {
    MissingLayer(Arc<str>),
    MissingGlyph(PyGlyph),
    MissingContour,
    MissingComponent,
    MissingPoint,
    MissingAnchor,
    MissingGlobalGuideline,
    MissingGlyphGuideline,
    MissingLayerGuideline(Arc<str>),
}

impl std::fmt::Display for ProxyError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            ProxyError::MissingLayer(layer) => write!(f, "Layer '{}' no longer exists.", layer),
            ProxyError::MissingGlyph(glyph) => {
                write!(f, "No glyph '{}' in layer '{}'", glyph.name, glyph.layer_name())
            }
            ProxyError::MissingContour => write!(f, "Missing contour"),
            ProxyError::MissingComponent => write!(f, "Missing component"),
            ProxyError::MissingAnchor => write!(f, "Missing anchor"),
            ProxyError::MissingPoint => write!(f, "Point not found in parent contour"),
            ProxyError::MissingGlobalGuideline => write!(f, "Missing global Guideline"),
            ProxyError::MissingGlyphGuideline => write!(f, "Missing glyph Guideline"),
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

//FIXME: more nuanced error mapping
pub(crate) fn error_to_py(error: norad::Error) -> PyErr {
    match error {
        norad::Error::DuplicateGlyph { .. } => exceptions::PyKeyError::new_err(error.to_string()),
        _ => exceptions::PyRuntimeError::new_err(error.to_string()),
    }
}
