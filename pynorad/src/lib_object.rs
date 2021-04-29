use std::collections::HashMap;

use plist::{Dictionary, Value};
use pyo3::{exceptions::PyValueError, prelude::*, types::PyDict};

use super::PyFont;

#[pyclass]
pub struct LibProxy {
    pub(crate) font: PyFont,
}

#[pymethods]
impl LibProxy {
    fn set_item(&mut self, key: String, value: RawValue) -> PyResult<()> {
        let value = from_python(value)?;
        self.font.write().lib.insert(key, value);
        Ok(())
    }

    fn get_item(&self, name: &str) -> PyResult<Option<PyObject>> {
        self.font.read().lib.get(name).map(to_python).transpose()
    }
}

#[pyproto]
impl pyo3::PyMappingProtocol for LibProxy {
    fn __len__(&self) -> usize {
        self.font.read().lib.len()
    }

    fn __getitem__(&'p self, name: &str) -> pyo3::PyResult<Option<PyObject>> {
        self.font.read().lib.get(name).map(to_python).transpose()
    }

    fn __setitem__(&'p mut self, key: String, value: RawValue) -> PyResult<()> {
        let value = from_python(value)?;
        self.font.write().lib.insert(key, value);
        Ok(())
    }

    fn __delitem__(&'p mut self, name: &str) -> pyo3::PyResult<()> {
        self.font.write().lib.remove(name);
        Ok(())
    }
}

fn to_python(obj: &Value) -> PyResult<PyObject> {
    let gil = Python::acquire_gil();
    let py = gil.python();
    Ok(match obj {
        Value::String(s) => s.to_object(py),
        Value::Real(f) => f.to_object(py),
        Value::Integer(int) => {
            if let Some(uint) = int.as_unsigned() {
                uint.to_object(py)
            } else if let Some(int) = int.as_signed() {
                int.to_object(py)
            } else {
                unreachable!()
            }
        }
        Value::Uid(hmm) => hmm.get().to_object(py),
        Value::Date(date) => format!("{:?}", date).to_object(py),
        Value::Data(data) => data.to_object(py),
        Value::Array(array) => {
            array.iter().map(to_python).collect::<Result<Vec<_>, _>>()?.to_object(py)
        }
        Value::Dictionary(dict) => {
            let d = PyDict::new(py);
            for (key, value) in dict.iter() {
                let value = to_python(value)?;
                d.set_item(key, value)?;
            }
            d.into()
        }
        Value::Boolean(b) => b.to_object(py),
        // should never happen
        _non_exhaustive => return Err(PyValueError::new_err("Unknown plist type")),
    })
}

fn from_python(value: RawValue) -> PyResult<Value> {
    match value {
        RawValue::Int(int) => Ok(int.into()),
        RawValue::Float(float) => Ok(float.into()),
        RawValue::String(string) => Ok(string.into()),
        RawValue::Array(array) => {
            array.into_iter().map(from_python).collect::<Result<Vec<_>, _>>().map(Into::into)
        }
        RawValue::Dict(dict) => {
            let mut plist = Dictionary::new();
            for (key, value) in dict.into_iter() {
                let value = from_python(value)?;
                plist.insert(key, value);
            }
            Ok(plist.into())
        }
    }
}

#[derive(FromPyObject)]
pub enum RawValue {
    #[pyo3(annotation = "str")]
    String(String),
    #[pyo3(annotation = "int")]
    Int(i64),
    #[pyo3(annotation = "float")]
    Float(f64),
    Array(Vec<RawValue>),
    Dict(HashMap<String, RawValue>),
}
