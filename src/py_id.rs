//! Supplementary ID type used by pynorad.
//!
//! Pynorad generally represents a reference to a given object in a font as
//! a reference to the global font, plus a path to the specific object. In the
//! case of objects stored in a vec (such as points in a contour) this path uses
//! the position in the vec as the id. This means that adding or removing items
//! to the vec can invalidate this index, causing existing references to point
//! at the wrong object. To guard against this we store an additional identifier
//! alongside these types that is also stored in the reference; we make sure that
//! these ids match when retreiving the object.

use std::sync::atomic::{AtomicU64, Ordering};

/// A fallback identifier for types that are referenced by index.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PyId(u64);

impl PyId {
    pub fn next() -> Self {
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let next = COUNTER.fetch_add(1, Ordering::Relaxed);
        PyId(next)
    }

    pub fn duplicate(&self) -> Self {
        PyId(self.0)
    }
}

impl Default for PyId {
    fn default() -> Self {
        PyId::next()
    }
}
