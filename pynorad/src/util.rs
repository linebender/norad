use pyo3::{exceptions::PyIndexError, PyResult};

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
