use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyTuple};
use quick_xml::events::{BytesStart, Event};
use quick_xml::reader::Reader;
use quick_xml::writer::Writer;
use std::collections::HashSet;
use std::io::Cursor;

// List of HTML5 void elements. These can be written as `<tag>` or `<tag />`,
//e.g. `<br />`, `<link />`, `<img />`, etc.
const VOID_ELEMENTS: [&str; 14] = [
    "area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "param", "source",
    "track", "wbr",
];

/// Transform HTML by adding attributes to the elements.
///
/// Args:
///     html (str): The HTML string to transform. Can be a fragment or full document.
///     root_attributes (List[str]): List of attribute names to add to root elements only.
///     all_attributes (List[str]): List of attribute names to add to all elements.
///     expand_empty_elements (bool, optional): Whether to expand self-closing tags into open/close pairs. Defaults to true.
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
///     >>> html, captured = transform_html(html, ['data-root-id'], ['data-v-123'], watch_on_attribute='data-id')
///     >>> print(captured)
///     {'123': ['data-root-id', 'data-v-123']}
///
/// Raises:
///     ValueError: If the HTML is malformed or cannot be parsed.
#[pyfunction]
#[pyo3(
    text_signature = "(html, root_attributes, all_attributes, *, expand_empty_elements=True, check_end_names=False, watch_on_attribute=None)"
)]
pub fn transform_html(
    py: Python,
    html: &str,
    root_attributes: Vec<String>,
    all_attributes: Vec<String>,
    expand_empty_elements: Option<bool>,
    check_end_names: Option<bool>,
    watch_on_attribute: Option<String>,
) -> PyResult<PyObject> {
    let config = HtmlTransformerConfig::new(
        root_attributes,
        all_attributes,
        expand_empty_elements.unwrap_or(true),
        check_end_names.unwrap_or(false),
        watch_on_attribute,
    );

    match transform(&config, html) {
        Ok((html, captured)) => {
            // Convert captured attributes to a Python dictionary
            let captured_dict = PyDict::new(py);
            for (id, attrs) in captured {
                captured_dict.set_item(id, attrs)?;
            }

            let result = PyTuple::new(py, &[html.into_py(py), captured_dict.into_py(py)]);
            Ok(result.into())
        }
        Err(e) => Err(PyValueError::new_err(e.to_string())),
    }
}

/// Configuration for HTML transformation
pub struct HtmlTransformerConfig {
    root_attributes: Vec<String>,
    all_attributes: Vec<String>,
    void_elements: HashSet<String>,
    expand_empty_elements: bool,
    check_end_names: bool,
    watch_on_attribute: Option<String>,
}

impl HtmlTransformerConfig {
    pub fn new(
        root_attributes: Vec<String>,
        all_attributes: Vec<String>,
        expand_empty_elements: bool,
        check_end_names: bool,
        watch_on_attribute: Option<String>,
    ) -> Self {
        let void_elements = VOID_ELEMENTS.iter().map(|&s| s.to_string()).collect();

        HtmlTransformerConfig {
            root_attributes,
            all_attributes,
            void_elements,
            expand_empty_elements,
            check_end_names,
            watch_on_attribute,
        }
    }
}

/// Add attributes to a HTML start tag (e.g. `<div>`) based on the configuration
fn add_attributes(
    config: &HtmlTransformerConfig,
    element: &mut BytesStart,
    is_root: bool,
    captured_attributes: &mut Vec<(String, Vec<String>)>,
) {
    let mut added_attrs = Vec::new();

    // Add root attributes if this is a root element
    if is_root {
        for attr in &config.root_attributes {
            element.push_attribute((attr.as_str(), ""));
            added_attrs.push(attr.clone());
        }
    }

    // Add attributes that should be applied to all elements
    for attr in &config.all_attributes {
        element.push_attribute((attr.as_str(), ""));
        added_attrs.push(attr.clone());
    }

    // If we're watching for a specific attribute, check if this element has it
    if let Some(watch_attr) = &config.watch_on_attribute {
        if let Some(attr_value) = element
            .attributes()
            .find(|a| {
                if let Ok(attr) = a {
                    String::from_utf8_lossy(attr.key.as_ref()) == *watch_attr
                } else {
                    false
                }
            })
            .and_then(|a| a.ok())
            .map(|a| String::from_utf8_lossy(a.value.as_ref()).into_owned())
        {
            captured_attributes.push((attr_value, added_attrs));
        }
    }
}

/// Main entrypoint. Transform HTML by adding attributes to the elements.
pub fn transform(
    config: &HtmlTransformerConfig,
    html: &str,
) -> Result<(String, Vec<(String, Vec<String>)>), Box<dyn std::error::Error>> {
    let mut reader = Reader::from_str(html);
    let reader_config = reader.config_mut();
    reader_config.expand_empty_elements = config.expand_empty_elements;
    reader_config.check_end_names = config.check_end_names;

    // We transform the HTML by reading it and writing it simultaneously
    let mut writer = Writer::new(Cursor::new(Vec::new()));
    let mut captured_attributes = Vec::new();

    // Track the nesting depth of elements to identify root elements (depth == 0)
    let mut depth: i32 = 0;

    // Read the HTML event by event
    loop {
        match reader.read_event() {
            // Start tag
            Ok(Event::Start(e)) => {
                let tag_name = String::from_utf8_lossy(e.name().as_ref())
                    .to_string()
                    .to_lowercase();
                let mut elem = e.into_owned();
                add_attributes(config, &mut elem, depth == 0, &mut captured_attributes);

                // For void elements, write as Empty event
                if config.void_elements.contains(&tag_name) {
                    writer.write_event(Event::Empty(elem))?;
                } else {
                    writer.write_event(Event::Start(elem))?;
                    depth += 1;
                }
            }

            // End tag
            Ok(Event::End(e)) => {
                let tag_name = String::from_utf8_lossy(e.name().as_ref())
                    .to_string()
                    .to_lowercase();

                // Skip end tags for void elements
                if !config.void_elements.contains(&tag_name) {
                    writer.write_event(Event::End(e))?;
                    depth -= 1;
                }
            }

            // Empty element (AKA void or self-closing tag, e.g. `<br />`)
            Ok(Event::Empty(e)) => {
                let mut elem = e.into_owned();
                add_attributes(config, &mut elem, depth == 0, &mut captured_attributes);
                writer.write_event(Event::Empty(elem))?;
            }

            // End of file
            Ok(Event::Eof) => break,
            // Other events (e.g. comments, processing instructions, etc.)
            Ok(e) => writer.write_event(e)?,
            Err(e) => return Err(Box::new(e)),
        }
    }

    // Convert the transformed HTML to a string
    let result = String::from_utf8(writer.into_inner().into_inner())?;
    Ok((result, captured_attributes))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_transformation() {
        let config = HtmlTransformerConfig::new(
            vec!["data-root".to_string()],
            vec!["data-all".to_string()],
            true,
            false,
            None,
        );

        let input = "<div><p>Hello</p></div>";
        let (result, _) = transform(&config, input).unwrap();

        assert!(result.contains("data-root"));
        assert!(result.contains("data-all"));
    }

    #[test]
    fn test_multiple_roots() {
        let config = HtmlTransformerConfig::new(
            vec!["data-root".to_string()],
            vec!["data-all".to_string()],
            true,
            false,
            None,
        );

        let input = "<div>First</div><span>Second</span>";
        let (result, _) = transform(&config, input).unwrap();

        // Both root elements should have data-root
        assert_eq!(result.matches("data-root").count(), 2);
        // All elements should have data-all
        assert_eq!(result.matches("data-all").count(), 2);
    }

    #[test]
    fn test_complex_html() {
        let config = HtmlTransformerConfig::new(
            vec!["data-root".to_string()],
            vec!["data-all".to_string(), "data-v-123".to_string()],
            true,
            false,
            None,
        );

        let input = r#"
            <div class="container" id="main">
                <header class="flex">
                    <h1 title="Main Title">Hello & Welcome</h1>
                    <nav data-existing="true">
                        <a href="/home">Home</a>
                        <a href="/about" class="active">About</a>
                    </nav>
                </header>
                <main>
                    <article data-existing="true">
                        <h2>Article 1</h2>
                        <p>Some text with <strong>bold</strong> and <em>emphasis</em></p>
                        <img src="test.jpg" alt="Test Image"/>
                    </article>
                </main>
            </div>
            <footer id="footer">
                <p>&copy; 2024</p>
            </footer>
        "#;

        let (result, _) = transform(&config, input).unwrap();

        // Check root elements have root attributes
        assert!(result.contains(
            r#"<div class="container" id="main" data-root="" data-all="" data-v-123="">"#
        ));
        assert!(result.contains(r#"<footer id="footer" data-root="" data-all="" data-v-123="">"#));

        // Check nested elements have all_attributes but not root_attributes
        assert!(result.contains(r#"<h1 title="Main Title" data-all="" data-v-123="">"#));
        assert!(result.contains(r#"<nav data-existing="true" data-all="" data-v-123="">"#));
        assert!(
            result.contains(r#"<img src="test.jpg" alt="Test Image" data-all="" data-v-123=""/>"#)
        );

        // Verify we didn't mess up the content or structure
        assert!(result.contains("Hello & Welcome"));
        assert!(result.contains("&copy; 2024"));
        assert!(result.contains(r#"<strong data-all="" data-v-123="">bold</strong>"#));
    }

    #[test]
    fn test_void_elements() {
        let config = HtmlTransformerConfig::new(
            vec!["data-root".to_string()],
            vec!["data-v-123".to_string()],
            true,
            false,
            None,
        );

        // Test various formats of void elements
        let test_cases = [
            (
                "<meta charset=\"utf-8\">",
                "<meta charset=\"utf-8\" data-root=\"\" data-v-123=\"\"/>"
            ),
            (
                "<meta charset=\"utf-8\"/>",
                "<meta charset=\"utf-8\" data-root=\"\" data-v-123=\"\"/>"
            ),
            (
                "<div><br><hr></div>",
                "<div data-root=\"\" data-v-123=\"\"><br data-v-123=\"\"/><hr data-v-123=\"\"/></div>"
            ),
            (
                "<img src=\"test.jpg\" alt=\"Test\">",
                "<img src=\"test.jpg\" alt=\"Test\" data-root=\"\" data-v-123=\"\"/>"
            ),
        ];

        for (input, expected) in test_cases {
            let (result, _) = transform(&config, input).unwrap();
            assert_eq!(result, expected);
        }

        // Test multiple void elements in a complex structure
        let input = r#"<div>
            <link rel="stylesheet" href="style.css">
            <img src="test.jpg">
            <p>Text with<br>break</p>
        </div>"#;

        let (result, _) = transform(&config, input).unwrap();

        // Verify void elements have attributes but no closing tags
        assert!(result.contains(r#"<link rel="stylesheet" href="style.css" data-v-123=""/>"#));
        assert!(result.contains(r#"<img src="test.jpg" data-v-123=""/>"#));
        assert!(result.contains(r#"<br data-v-123=""/>"#));

        // Verify non-void elements still have proper closing tags
        assert!(result.contains("</p>"));
        assert!(result.contains("</div>"));
    }

    #[test]
    fn test_html_head_with_meta() {
        let config = HtmlTransformerConfig::new(
            vec!["data-root".to_string()],
            vec!["data-v-123".to_string()],
            true,
            false,
            None,
        );

        let input = r#"
            <head>
                <meta charset="utf-8">
                <title>Test Page</title>
                <link rel="stylesheet" href="style.css">
                <meta name="description" content="Test">
            </head>"#;

        let (result, _) = transform(&config, input).unwrap();

        // Check that it parsed successfully
        assert!(result.contains(r#"<meta charset="utf-8""#));
        assert!(result.contains(r#"<title data-v-123="">Test Page</title>"#));
        assert!(result.contains(r#"<link rel="stylesheet" href="style.css""#));

        // Verify void elements are properly handled
        assert!(!result.contains("</meta>"));
        assert!(!result.contains("</link>"));
        assert!(result.contains("/>"));
    }

    #[test]
    fn test_config_expand_empty_elements() {
        // Test with expand_empty_elements = false
        let config = HtmlTransformerConfig::new(
            vec!["data-root".to_string()],
            vec!["data-v-123".to_string()],
            false, // Don't expand empty elements
            false,
            None,
        );

        let test_cases = [
            // Non-void elements should stay self-closing when expand_empty_elements is false
            (
                "<div/>",
                "<div data-root=\"\" data-v-123=\"\"/>"
            ),
            (
                "<p/>",
                "<p data-root=\"\" data-v-123=\"\"/>"
            ),
            (
                "<div><span/></div>",
                "<div data-root=\"\" data-v-123=\"\"><span data-v-123=\"\"/></div>"
            ),
            // Void elements should always be self-closing regardless of config
            (
                "<div><img/><br/></div>",
                "<div data-root=\"\" data-v-123=\"\"><img data-v-123=\"\"/><br data-v-123=\"\"/></div>"
            ),
        ];

        for (input, expected) in test_cases {
            let (result, _) = transform(&config, input).unwrap();
            assert_eq!(result, expected);
        }

        // Compare with expand_empty_elements = true
        let config = HtmlTransformerConfig::new(
            vec!["data-root".to_string()],
            vec!["data-v-123".to_string()],
            true, // Expand empty elements
            false,
            None,
        );

        let expanded_cases = [
            ("<div/>", "<div data-root=\"\" data-v-123=\"\"></div>"),
            ("<p/>", "<p data-root=\"\" data-v-123=\"\"></p>"),
            // Void elements should still be self-closing
            (
                "<div><img/></div>",
                "<div data-root=\"\" data-v-123=\"\"><img data-v-123=\"\"/></div>",
            ),
        ];

        for (input, expected) in expanded_cases {
            let (result, _) = transform(&config, input).unwrap();
            assert_eq!(result, expected);
        }
    }

    #[test]
    fn test_config_check_end_names() {
        // Test with check_end_names = false (lenient mode)
        let config = HtmlTransformerConfig::new(
            vec!["data-root".to_string()],
            vec!["data-v-123".to_string()],
            true,
            false, // Don't check end names
            None,
        );

        // These should parse successfully with check_end_names = false
        let lenient_cases = [
            "<div><p>Hello</div></p>", // Mismatched nesting
            "<div>Text</span>",        // Wrong closing tag
            "<p>Text</wrong>",         // Non-matching end tag
        ];

        for input in lenient_cases {
            assert!(transform(&config, input).is_ok());
        }

        // Test with check_end_names = true (strict mode)
        let config = HtmlTransformerConfig::new(
            vec!["data-root".to_string()],
            vec!["data-v-123".to_string()],
            true,
            true, // Check end names
            None,
        );

        // These should fail with check_end_names = true
        for input in lenient_cases {
            assert!(transform(&config, input).is_err());
        }

        // But well-formed HTML should still work
        let valid_input = "<div><p>Hello</p></div>";
        assert!(transform(&config, valid_input).is_ok());
    }

    #[test]
    fn test_watch_attribute() {
        let config = HtmlTransformerConfig::new(
            vec!["data-root".to_string()],
            vec!["data-v-123".to_string()],
            true,
            false,
            Some("data-id".to_string()),
        );

        let input = r#"
            <div data-id="123">
                <p>Regular element</p>
                <span data-id="456">Nested element</span>
                <img data-id="789" src="test.jpg"/>
            </div>"#;

        let (result, captured) = transform(&config, input).unwrap();

        println!("result: {}", result);
        println!("captured: {:?}", captured);

        // Verify HTML transformation worked
        assert!(result.contains(r#"<div data-id="123" data-root="" data-v-123="">"#));
        assert!(result.contains(r#"<span data-id="456" data-v-123="">"#));
        assert!(result.contains(r#"<img data-id="789" src="test.jpg" data-v-123=""/>"#));

        // Verify attribute capturing
        assert_eq!(captured.len(), 3);
        assert!(captured.iter().any(|(id, attrs)| id == "123"
            && attrs.contains(&"data-root".to_string())
            && attrs.contains(&"data-v-123".to_string())));
        assert!(captured
            .iter()
            .any(|(id, attrs)| id == "456" && attrs.contains(&"data-v-123".to_string())));
        assert!(captured
            .iter()
            .any(|(id, attrs)| id == "789" && attrs.contains(&"data-v-123".to_string())));
    }
}
