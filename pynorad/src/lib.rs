//! UfoLib2 compatable API based on top of norad.

mod error;
mod font;
mod fontinfo;
mod glyph;
mod guideline;
mod layer;
#[macro_use]
mod util;
mod lib_object;

pub use error::ProxyError;
pub use font::PyFont;
pub use fontinfo::PyFontInfo;
pub use glyph::{
    PointsIter, PyAnchor, PyComponent, PyContour, PyGlyph, PyImage, PyPoint, PyPointPen,
};
pub use guideline::PyGuideline;
pub use layer::{GlyphIter, LayerIter, PyLayer};
pub use lib_object::PyLib;

use pyo3::prelude::*;

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
    m.add_class::<PyImage>()?;
    m.add_class::<PyGuideline>()?;
    m.add_class::<PyFontInfo>()?;
    Ok(())
}
