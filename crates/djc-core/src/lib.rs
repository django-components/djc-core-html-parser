use djc_html_transformer::set_html_attributes;
use pyo3::prelude::*;

/// A Python module implemented in Rust for high-performance transformations.
#[pymodule]
fn djc_core(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(set_html_attributes, m)?)?;
    Ok(())
}
