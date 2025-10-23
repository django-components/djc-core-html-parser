use djc_html_transformer::{set_html_attributes as set_html_attributes_rust, HtmlTransformerConfig};
use djc_template_parser::{
    compile_ast_to_string as compile_ast_to_string_rust, parse_tag as parse_tag_rust, Tag, TagAttr,
    TagSyntax, TagToken, TagValue, TagValueFilter, ValueKind,
};
use pyo3::exceptions::{PySyntaxError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyList, PyDict, PyTuple};
use std::collections::HashSet;

/// Singular Python API that brings togther all the other Rust crates.
#[pymodule]
fn djc_core(m: &Bound<'_, PyModule>) -> PyResult<()> {
    // HTML transformer
    m.add_function(wrap_pyfunction!(set_html_attributes, m)?)?;

    // Template parser
    m.add_function(wrap_pyfunction!(parse_tag, m)?)?;
    m.add_function(wrap_pyfunction!(compile_ast_to_string, m)?)?;
    m.add_class::<Tag>()?;
    m.add_class::<TagAttr>()?;
    m.add_class::<TagSyntax>()?;
    m.add_class::<TagToken>()?;
    m.add_class::<TagValue>()?;
    m.add_class::<TagValueFilter>()?;
    m.add_class::<ValueKind>()?;

    Ok(())
}

#[pyfunction]
#[pyo3(signature = (input, flags=None))]
fn parse_tag(input: &str, flags: Option<HashSet<String>>) -> PyResult<Tag> {
    parse_tag_rust(input, flags).map_err(|e| PySyntaxError::new_err(e.to_string()))
}

#[pyfunction]
fn compile_ast_to_string(py: Python, attributes: &Bound<PyList>) -> PyResult<String> {
    let attrs: Vec<TagAttr> = attributes.extract()?;
    let result = py.detach(|| compile_ast_to_string_rust(&attrs));
    result.map_err(|e| PySyntaxError::new_err(e.to_string()))
}

/// Transform HTML by adding attributes to the elements.
///
/// Args:
///     html (str): The HTML string to transform. Can be a fragment or full document.
///     root_attributes (List[str]): List of attribute names to add to root elements only.
///     all_attributes (List[str]): List of attribute names to add to all elements.
///     check_end_names (bool, optional): Whether to validate matching of end tags. Defaults to false.
///     watch_on_attribute (str, optional): If set, captures which attributes were added to elements with this attribute.
///
/// Returns:
///     Tuple[str, Dict[str, List[str]]]: A tuple containing:
///         - The transformed HTML string
///         - A dictionary mapping captured attribute values to lists of attributes that were added
///           to those elements. Only returned if watch_on_attribute is set, otherwise empty dict.
///
/// Example:
///     >>> html = '<div data-id="123"><p>Hello</p></div>'
///     >>> html, captured = set_html_attributes(html, ['data-root-id'], ['data-v-123'], watch_on_attribute='data-id')
///     >>> print(captured)
///     {'123': ['data-root-id', 'data-v-123']}
///
/// Raises:
///     ValueError: If the HTML is malformed or cannot be parsed.
#[pyfunction]
#[pyo3(signature = (html, root_attributes, all_attributes, check_end_names=None, watch_on_attribute=None))]
#[pyo3(
    text_signature = "(html, root_attributes, all_attributes, *, check_end_names=False, watch_on_attribute=None)"
)]
pub fn set_html_attributes(
    py: Python,
    html: &str,
    root_attributes: Vec<String>,
    all_attributes: Vec<String>,
    check_end_names: Option<bool>,
    watch_on_attribute: Option<String>,
) -> PyResult<Py<PyAny>> {
    let config = HtmlTransformerConfig::new(
        root_attributes,
        all_attributes,
        check_end_names.unwrap_or(false),
        watch_on_attribute,
    );

    match set_html_attributes_rust(html, &config) {
        Ok((html, captured)) => {
            // Convert captured attributes to a Python dictionary
            let captured_dict = PyDict::new(py);
            for (id, attrs) in captured {
                captured_dict.set_item(id, attrs)?;
            }

            // Convert items to Bound<PyAny> for the tuple
            use pyo3::types::PyString;
            let html_obj = PyString::new(py, &html).as_any().clone();
            let dict_obj = captured_dict.as_any().clone();
            let result = PyTuple::new(py, vec![html_obj, dict_obj])?;
            Ok(result.into_any().unbind())
        }
        Err(e) => Err(PyValueError::new_err(e.to_string())),
    }
}
