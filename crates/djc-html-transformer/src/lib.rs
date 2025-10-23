use transformer::{transform};

pub mod transformer;

// Re-export the types that users need
pub use transformer::{HtmlTransformerConfig};

/// Transform HTML by adding attributes to the elements.
///
/// This is the pure Rust version that takes a configuration object.
///
/// Args:
///     html: The HTML string to transform. Can be a fragment or full document.
///     config: The HTML transformer configuration.
///
/// Returns:
///     A Result containing either:
///     - Ok((html, captured)): A tuple with the transformed HTML and captured attributes
///     - Err(error): An error if the HTML is malformed or cannot be parsed.
pub fn set_html_attributes(
    html: &str,
    config: &HtmlTransformerConfig,
) -> Result<(String, Vec<(String, Vec<String>)>), Box<dyn std::error::Error>> {
    transform(config, html)
}
