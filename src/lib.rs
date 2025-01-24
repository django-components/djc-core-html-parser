use pyo3::prelude::*;
use pyo3::types::PyModule;

mod html_parser;

/// A Python module implemented in Rust for high-performance HTML transformation.
#[pymodule]
fn djc_core_html_parser(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(html_parser::set_html_attributes, m)?)?;
    Ok(())
}
