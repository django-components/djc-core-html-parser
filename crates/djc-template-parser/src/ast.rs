//! # Abstract Syntax Tree (AST) for Django Template Tags
//!
//! This module defines the core data structures that represent parsed Django template tags
//! as an Abstract Syntax Tree (AST). These structures are used throughout the template
//! parsing and compilation pipeline.
//!
//! ## Overview
//!
//! The AST represents Django template tags in a structured format that captures:
//! - Tag names and attributes
//! - Values with their types (strings, numbers, variables, template_strings, etc.)
//! - Filter chains and filter arguments
//! - Position information (line/column, start/end indices)
//! - Syntax type (Django `{% %}` vs HTML `< />` tags)
//!
//! ## Core types
//!
//! - **`Tag`**: Represents a complete template tag with name, attributes, and metadata - `{% my_tag ... %}` or `<my_tag ... />`
//! - **`TagAttr`**: Represents a single attribute (key-value pair or flag) - `key=value` or `flag`
//! - **`TagValue`**: Represents a value with type information and optional filters - `'some_val'|upper`
//! - **`TagToken`**: Represents a token with position information
//! - **`TagValueFilter`**: Represents a filter applied to a value
//! - **`ValueKind`**: Enum of supported value types (list, dict, int, float, variable, template_string, translation, string)
//! - **`TagSyntax`**: Enum of supported tag syntaxes (Django vs HTML)
//!
//! All AST types are exposed to Python via PyO3 bindings.
//!
//! ## Example
//!
//! ```rust
//! use crate::djc_template_parser::ast::*;
//!
//! // A Django tag: {% my_tag key=val %}
//! let tag = Tag {
//!     name: TagToken {
//!        token: "my_tag".to_string(),
//!        start_index: 3,
//!        end_index: 9,
//!        line_col: (1, 4),
//!     },
//!     attrs: vec![TagAttr {
//!         key: Some(TagToken {
//!             token: "key".to_string(),
//!             start_index: 10,
//!             end_index: 13,
//!             line_col: (1, 11),
//!         }),
//!         value: TagValue {
//!             token: TagToken {
//!                 token: "val".to_string(),
//!                 start_index: 14,
//!                 end_index: 17,
//!                 line_col: (1, 15),
//!             },
//!             children: vec![],
//!             spread: None,
//!             filters: vec![],
//!             kind: ValueKind::Variable,
//!             start_index: 14,
//!             end_index: 17,
//!             line_col: (1, 15),
//!         },
//!         is_flag: false,
//!         start_index: 10,
//!         end_index: 17,
//!         line_col: (1, 11),
//!     }],
//!     is_self_closing: false,
//!     syntax: TagSyntax::Django,
//!     start_index: 0,
//!     end_index: 20,
//!     line_col: (1, 4),
//! };
//! ```

use pyo3::prelude::*;

/// Top-level tag attribute, e.g. `key=my_var` or without key like `my_var|filter`
#[pyclass]
#[derive(Debug, PartialEq, Clone)]
pub struct TagAttr {
    #[pyo3(get)]
    pub key: Option<TagToken>,
    #[pyo3(get)]
    pub value: TagValue,
    #[pyo3(get)]
    pub is_flag: bool,

    /// Start index (incl. filters)
    #[pyo3(get)]
    pub start_index: usize,
    /// End index (incl. filters)
    #[pyo3(get)]
    pub end_index: usize,
    /// Line and column (incl. filters)
    #[pyo3(get)]
    pub line_col: (usize, usize),
}

#[pymethods]
impl TagAttr {
    // These methods with `[new]` will become constructors (`__init__()`)
    #[new]
    #[pyo3(signature = (key, value, is_flag, start_index, end_index, line_col))]
    fn new(
        key: Option<TagToken>,
        value: TagValue,
        is_flag: bool,
        start_index: usize,
        end_index: usize,
        line_col: (usize, usize),
    ) -> Self {
        Self {
            key,
            value,
            is_flag,
            start_index,
            end_index,
            line_col,
        }
    }

    // Allow to compare objects with `==`
    fn __eq__(&self, other: &TagAttr) -> bool {
        self.key == other.key
            && self.value == other.value
            && self.is_flag == other.is_flag
            && self.start_index == other.start_index
            && self.end_index == other.end_index
            && self.line_col == other.line_col
    }

    fn __repr__(&self) -> String {
        format!("TagAttr(key={:?}, value={:?}, is_flag={}, start_index={}, end_index={}, line_col={:?})", 
                self.key, self.value, self.is_flag, self.start_index, self.end_index, self.line_col)
    }
}

#[pyclass(eq, eq_int)]
#[derive(Debug, PartialEq, Clone)]
pub enum ValueKind {
    List,
    Dict,
    Int,
    Float,
    Variable,
    TemplateString,  // A string that contains a Django template tags, e.g. `"{{ my_var }}"`
    Translation,
    String,
}

#[pymethods]
impl ValueKind {
    #[new]
    fn new(kind: &str) -> PyResult<Self> {
        match kind {
            "list" => Ok(ValueKind::List),
            "dict" => Ok(ValueKind::Dict),
            "int" => Ok(ValueKind::Int),
            "float" => Ok(ValueKind::Float),
            "variable" => Ok(ValueKind::Variable),
            "template_string" => Ok(ValueKind::TemplateString),
            "translation" => Ok(ValueKind::Translation),
            "string" => Ok(ValueKind::String),
            _ => Err(pyo3::exceptions::PyValueError::new_err(format!(
                "Invalid ValueKind: {}",
                kind
            ))),
        }
    }

    fn __str__(&self) -> String {
        match self {
            ValueKind::List => "list".to_string(),
            ValueKind::Dict => "dict".to_string(),
            ValueKind::Int => "int".to_string(),
            ValueKind::Float => "float".to_string(),
            ValueKind::Variable => "variable".to_string(),
            ValueKind::TemplateString => "template_string".to_string(),
            ValueKind::Translation => "translation".to_string(),
            ValueKind::String => "string".to_string(),
        }
    }
}

/// Metadata of the matched token
#[pyclass]
#[derive(Debug, PartialEq, Clone)]
pub struct TagToken {
    /// String value of the token (excl. filters and spread)
    #[pyo3(get)]
    pub token: String,
    /// Start index (excl. filters and spread)
    #[pyo3(get)]
    pub start_index: usize,
    /// End index (excl. filters and spread)
    #[pyo3(get)]
    pub end_index: usize,
    /// Line and column (excl. filters and spread)
    #[pyo3(get)]
    pub line_col: (usize, usize),
}

#[pymethods]
impl TagToken {
    #[new]
    fn new(token: String, start_index: usize, end_index: usize, line_col: (usize, usize)) -> Self {
        Self {
            token,
            start_index,
            end_index,
            line_col,
        }
    }

    fn __eq__(&self, other: &TagToken) -> bool {
        self.token == other.token
            && self.start_index == other.start_index
            && self.end_index == other.end_index
            && self.line_col == other.line_col
    }

    fn __repr__(&self) -> String {
        format!(
            "TagToken(token='{}', start_index={}, end_index={}, line_col={:?})",
            self.token, self.start_index, self.end_index, self.line_col
        )
    }
}

#[pyclass]
#[derive(Debug, PartialEq, Clone)]
pub struct TagValue {
    /// Position and string value of the value (excl. filters and spread)
    ///
    /// NOTE: If this TagValue has NO filters, position and index in `token` are the same
    ///       as `start_index`, `end_index` and `line_col` defined directly on `TagValue`.
    #[pyo3(get)]
    pub token: TagToken,
    /// Children of this TagValue - e.g. list items like `[1, 2, 3]` or dict key-value entries like `{"key": "value"}`
    #[pyo3(get)]
    pub children: Vec<TagValue>,

    #[pyo3(get)]
    pub kind: ValueKind,
    #[pyo3(get)]
    pub spread: Option<String>,
    #[pyo3(get)]
    pub filters: Vec<TagValueFilter>,

    /// Start index (incl. filters and spread)
    #[pyo3(get)]
    pub start_index: usize,
    /// End index (incl. filters and spread)
    #[pyo3(get)]
    pub end_index: usize,
    /// Line and column (incl. filters and spread)
    #[pyo3(get)]
    pub line_col: (usize, usize),
}

#[pymethods]
impl TagValue {
    #[new]
    #[pyo3(signature = (token, children, kind, spread, filters, start_index, end_index, line_col))]
    fn new(
        token: TagToken,
        children: Vec<TagValue>,
        kind: ValueKind,
        spread: Option<String>,
        filters: Vec<TagValueFilter>,
        start_index: usize,
        end_index: usize,
        line_col: (usize, usize),
    ) -> Self {
        Self {
            token,
            children,
            kind,
            spread,
            filters,
            start_index,
            end_index,
            line_col,
        }
    }

    fn __eq__(&self, other: &TagValue) -> bool {
        self.token == other.token
            && self.children == other.children
            && self.kind == other.kind
            && self.spread == other.spread
            && self.filters == other.filters
            && self.start_index == other.start_index
            && self.end_index == other.end_index
            && self.line_col == other.line_col
    }

    fn __repr__(&self) -> String {
        format!("TagValue(token={:?}, children={:?}, kind={:?}, spread={:?}, filters={:?}, start_index={}, end_index={}, line_col={:?})", 
                self.token, self.children, self.kind, self.spread, self.filters, self.start_index, self.end_index, self.line_col)
    }
}

#[pyclass]
#[derive(Debug, PartialEq, Clone)]
pub struct TagValueFilter {
    /// Token of the filter, e.g. `filter`
    #[pyo3(get)]
    pub token: TagToken,
    /// Argument of the filter, e.g. `my_var`
    #[pyo3(get)]
    pub arg: Option<TagValue>,

    /// Start index (incl. `|`)
    #[pyo3(get)]
    pub start_index: usize,
    /// End index (incl. `|`)
    #[pyo3(get)]
    pub end_index: usize,
    /// Line and column (incl. `|`)
    #[pyo3(get)]
    pub line_col: (usize, usize),
}

#[pymethods]
impl TagValueFilter {
    #[new]
    #[pyo3(signature = (token, arg, start_index, end_index, line_col))]
    fn new(
        token: TagToken,
        arg: Option<TagValue>,
        start_index: usize,
        end_index: usize,
        line_col: (usize, usize),
    ) -> Self {
        Self {
            token,
            arg,
            start_index,
            end_index,
            line_col,
        }
    }

    fn __eq__(&self, other: &TagValueFilter) -> bool {
        self.token == other.token
            && self.arg == other.arg
            && self.start_index == other.start_index
            && self.end_index == other.end_index
            && self.line_col == other.line_col
    }

    fn __repr__(&self) -> String {
        format!(
            "TagValueFilter(token={:?}, arg={:?}, start_index={}, end_index={}, line_col={:?})",
            self.token, self.arg, self.start_index, self.end_index, self.line_col
        )
    }
}

#[pyclass(eq, eq_int)]
#[derive(Debug, PartialEq, Clone)]
pub enum TagSyntax {
    Django, // For tags like {% my_tag ... %}
    Html,   // For tags like <my_tag ... />
}

#[pymethods]
impl TagSyntax {
    #[new]
    fn new(syntax: &str) -> PyResult<Self> {
        match syntax {
            "django" => Ok(TagSyntax::Django),
            "html" => Ok(TagSyntax::Html),
            _ => Err(pyo3::exceptions::PyValueError::new_err(format!(
                "Invalid TagSyntax: {}",
                syntax
            ))),
        }
    }

    fn __str__(&self) -> String {
        match self {
            TagSyntax::Django => "django".to_string(),
            TagSyntax::Html => "html".to_string(),
        }
    }
}

/// Represents a full template tag, including its name, attributes, and other metadata.
/// E.g. `{% slot key=val key2=val2 %}` or `<slot key=val key2=val2>`
#[pyclass]
#[derive(Debug, PartialEq, Clone)]
pub struct Tag {
    /// The name of the tag, e.g., 'slot' in `{% slot ... %}`.
    /// This is a `TagToken` to include positional data.
    #[pyo3(get)]
    pub name: TagToken,

    /// A list of attributes passed to the tag.
    #[pyo3(get)]
    pub attrs: Vec<TagAttr>,

    /// Whether the tag is self-closing.
    /// E.g. `{% my_tag / %}` or `<my_tag />`.
    #[pyo3(get)]
    pub is_self_closing: bool,

    /// The syntax of the tag:
    /// - `django` for `{% my_tag / %}`
    /// - `html` for `<my_tag />`.
    #[pyo3(get)]
    pub syntax: TagSyntax,

    /// Start index of the tag in the original input string.
    #[pyo3(get)]
    pub start_index: usize,

    /// End index of the tag in the original input string.
    #[pyo3(get)]
    pub end_index: usize,

    /// Line and column number of the start of the tag.
    #[pyo3(get)]
    pub line_col: (usize, usize),
}

#[pymethods]
impl Tag {
    #[new]
    fn new(
        name: TagToken,
        attrs: Vec<TagAttr>,
        is_self_closing: bool,
        syntax: TagSyntax,
        start_index: usize,
        end_index: usize,
        line_col: (usize, usize),
    ) -> Self {
        Self {
            name,
            attrs,
            is_self_closing,
            syntax,
            start_index,
            end_index,
            line_col,
        }
    }

    fn __eq__(&self, other: &Tag) -> bool {
        self.name == other.name
            && self.attrs == other.attrs
            && self.is_self_closing == other.is_self_closing
            && self.syntax == other.syntax
            && self.start_index == other.start_index
            && self.end_index == other.end_index
            && self.line_col == other.line_col
    }

    fn __repr__(&self) -> String {
        format!("Tag(name={:?}, attrs={:?}, is_self_closing={}, syntax={:?}, start_index={}, end_index={}, line_col={:?})", 
                self.name, self.attrs, self.is_self_closing, self.syntax, self.start_index, self.end_index, self.line_col)
    }
}
