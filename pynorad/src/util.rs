use norad::{Color, Identifier};
use std::str::FromStr;

use pyo3::{
    exceptions::{PyIndexError, PyValueError},
    PyResult,
};

#[macro_export]
macro_rules! flatten {
    ($expr:expr $(,)?) => {
        match $expr {
            Err(e) => Err(e),
            Ok(Err(e)) => Err(e),
            Ok(Ok(fine)) => Ok(fine),
        }
    };
}

pub(crate) fn python_idx_to_idx(idx: isize, len: usize) -> PyResult<usize> {
    let idx = if idx.is_negative() { len - (idx.abs() as usize % len) } else { idx as usize };

    if idx < len {
        Ok(idx)
    } else {
        Err(PyIndexError::new_err(format!(
            "Index {} out of bounds of collection with length {}",
            idx, len
        )))
    }
}

pub(crate) fn to_identifier(s: Option<&str>) -> PyResult<Option<Identifier>> {
    s.map(Identifier::new).transpose().map_err(|_| {
        PyValueError::new_err(
            "Identifier must be between 0 and 100 characters, each in the range 0x20..=0x7E",
        )
    })
}

pub(crate) fn to_color(s: Option<&str>) -> PyResult<Option<Color>> {
    s.map(Color::from_str).transpose().map_err(|_| PyValueError::new_err("Invalid color string"))
}
