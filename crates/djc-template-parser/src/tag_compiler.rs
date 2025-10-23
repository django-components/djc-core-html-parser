//! # Django Template Tag Compiler
//!
//! This module translates parsed AST representations of Django template tags (e.g. `{% component %}`)
//! into a code definition of a callable Python function (e.g. `def func(context, ...):\n    ...`)
//!
//! The generated function takes a `context` object and returns a tuple of arguments and keyword arguments.
//!
//! ## Features
//!
//! - **Argument ordering**: Maintains Python-like behavior with positional args before keyword args
//! - **Spread operators**: Handles `...list` and `**dict` spread syntax
//! - **Filter compilation**: Converts Django filter chains to Python expressions
//! - **Value type handling**: Properly handles strings, numbers, variables, template_strings, etc.
//! - **Error detection**: Compile-time detection of invalid argument ordering
//! - **Indentation**: Properly indents generated code for readability
//!
//! ## Error handling
//!
//! The compiler returns `Result<String, String>` where errors include:
//! - Invalid argument ordering (positional after keyword)
//! - Compilation failures for complex values
//! - Filter chain compilation errors
//!

pub use crate::ast::{TagAttr, TagValue, ValueKind};
use crate::error::CompileError;

pub fn compile_ast_to_string(attributes: &[TagAttr]) -> Result<String, CompileError> {
    let mut body = String::new();
    // We want to keep Python-like behaviour with args having to come before kwargs.
    // When we have only args and kwargs, we can check at compile-time whether
    // there are any args after kwargs, and raise an error if so.
    // But if there is a spread (`...var`) then this has to be handled at runtime,
    // because we don't know if `var` is a mapping or an iterable.
    //
    // So what we do is that we preferentially raise an error at compile-time.
    // And once we come across a spread (`...var`), then we set `kwarg_seen` also in Python,
    // and run the checks in both Python and Rust.
    let mut has_spread = false;
    let mut kwarg_seen = false;

    for attr in attributes {
        if attr.is_flag {
            continue;
        }

        if let Some(key) = &attr.key {
            // It's a kwarg: key=value
            let value_str = compile_value(&attr.value)?;
            body.push_str(&format!(
                "kwargs.append(('{}', {}))\n",
                key.token, value_str
            ));
            // Spreads have to be handled at runtime. But before we come across a spread,
            // we can check solely at compile-time whether there are any args after kwargs,
            // which should raise an error.
            if !kwarg_seen {
                if has_spread {
                    body.push_str("kwarg_seen = True\n");
                }
                kwarg_seen = true;
            }
        } else if attr.value.spread.is_some() {
            // It's a spread: ...value
            if !has_spread {
                has_spread = true;
                // First time we come across a spread,
                // start tracking arg/kwarg orders at run time,
                // because we need the Context to know if this spread is a dict or an iterable.
                body.push_str(&format!(
                    "kwarg_seen = {}\n",
                    if kwarg_seen { "True" } else { "False" }
                ));
            }
            let value_str = compile_value(&attr.value)?;
            let raw_token_str = attr.value.token.token.replace("\"", "\\\"");
            body.push_str(&format!(
                // NOTE: We wrap the raw token in triple quotes because it may contain newlines
                "kwarg_seen = _handle_spread({}, \"\"\"{}\"\"\", args, kwargs, kwarg_seen)\n",
                value_str, raw_token_str
            ));
        } else {
            // This is a positional arg: value
            // Capture args after kwargs at compile time
            if kwarg_seen {
                return Err(CompileError::from(
                    "positional argument follows keyword argument",
                ));
            }
            // Capture args after kwargs at run time
            if has_spread {
                body.push_str("if kwarg_seen:\n");
                body.push_str(
                    "    raise SyntaxError(\"positional argument follows keyword argument\")\n",
                );
            }
            let value_str = compile_value(&attr.value)?;
            body.push_str(&format!("args.append({})\n", value_str));
        }
    }

    let mut final_code = String::new();
    let signature = "def compiled_func(context, *, template_string, translation, variable, filter):";
    final_code.push_str(signature);
    final_code.push_str("\n");

    // The top-level `...var` spread may be handled either as a list of args or a dict of kwargs
    // depending on the value of `var`.
    // So we check if `var` is a mapping by checking for `.keys()` or if it's an iterable.
    // We use this helper function to handle this.
    if has_spread {
        let helper_func = r#"def _handle_spread(value, raw_token_str, args, kwargs, kwarg_seen):
    if hasattr(value, "keys"):
        kwargs.extend(value.items())
        return True
    else:
        if kwarg_seen:
            raise SyntaxError("positional argument follows keyword argument")
        try:
            args.extend(value)
        except TypeError:
            raise TypeError(
                f"Value of '...{raw_token_str}' must be a mapping or an iterable, "
                f"not {type(value).__name__}."
            )
        return False

"#;
        final_code.push_str(&indent_body(&helper_func, 4));
        final_code.push_str("\n");
    }

    final_code.push_str("    args = []\n");
    final_code.push_str("    kwargs = []\n");
    if !body.trim().is_empty() {
        final_code.push_str(&indent_body(&body, 4));
        final_code.push_str("\n");
    }
    final_code.push_str("    return args, kwargs");

    Ok(final_code)
}

fn indent_body(body: &str, indent_level: usize) -> String {
    let indent = " ".repeat(indent_level);
    body.lines()
        .map(|line| {
            if line.trim().is_empty() {
                String::new()
            } else {
                format!("{}{}", indent, line)
            }
        })
        .collect::<Vec<String>>()
        .join("\n")
}

fn compile_value(value: &TagValue) -> Result<String, CompileError> {
    let compiled_value = match value.kind {
        ValueKind::Int | ValueKind::Float => Ok(value.token.token.clone()),
        ValueKind::String => {
            // The token includes quotes, which is what we want for a Python string literal
            Ok(value.token.token.clone())
        }
        ValueKind::Variable => Ok(format!("variable(context, '{}')", value.token.token)),
        ValueKind::TemplateString => Ok(format!("template_string(context, {})", value.token.token)),
        ValueKind::Translation => {
            let inner_string_start = value.token.token.find('(').map(|i| i + 1).unwrap_or(0);
            let inner_string_end = value
                .token
                .token
                .rfind(')')
                .unwrap_or(value.token.token.len());
            if inner_string_start > 0 && inner_string_end > inner_string_start {
                let inner_string = &value.token.token[inner_string_start..inner_string_end];
                Ok(format!("translation(context, {})", inner_string))
            } else {
                Err(CompileError::from(format!(
                    "Invalid translation string format: {}",
                    value.token.token
                )))
            }
        }
        ValueKind::List => {
            let mut items = Vec::new();
            for item in &value.children {
                let compiled_item = compile_value(item)?;
                if item.spread.is_some() {
                    items.push(format!("*{}", compiled_item));
                } else {
                    items.push(compiled_item);
                }
            }
            Ok(format!("[{}]", items.join(", ")))
        }
        ValueKind::Dict => {
            let mut items = Vec::new();
            let mut children_iter = value.children.iter();
            while let Some(child) = children_iter.next() {
                if child.spread.is_some() {
                    items.push(format!("**{}", compile_value(child)?));
                } else {
                    // This is a key, next must be value
                    let key = child;
                    let value = children_iter.next().ok_or_else(|| {
                        CompileError::from("Dict AST has uneven number of key-value children")
                    })?;
                    let compiled_key = compile_value(key)?;
                    let compiled_value = compile_value(value)?;
                    items.push(format!("{}: {}", compiled_key, compiled_value));
                }
            }
            Ok(format!("{{{}}}", items.join(", ")))
        }
    };

    let mut result = compiled_value?;

    // Apply filters
    for filter in &value.filters {
        let filter_name = &filter.token.token;
        if let Some(arg) = &filter.arg {
            let compiled_arg = compile_value(arg)?;
            result = format!(
                "filter(context, '{}', {}, {})",
                filter_name, result, compiled_arg
            );
        } else {
            result = format!("filter(context, '{}', {}, None)", filter_name, result);
        }
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::{TagAttr, TagToken, TagValue, TagValueFilter, ValueKind};
    use crate::tag_parser::TagParser;
    use std::collections::HashSet;

    fn create_tag_token(token: &str) -> TagToken {
        TagToken {
            token: token.to_string(),
            start_index: 0,
            end_index: token.len(),
            line_col: (1, 1),
        }
    }

    fn create_var_tag_value(token: &str) -> TagValue {
        TagValue {
            token: create_tag_token(token),
            children: vec![],
            kind: ValueKind::Variable,
            spread: None,
            filters: vec![],
            start_index: 0,
            end_index: token.len(),
            line_col: (1, 1),
        }
    }

    fn create_string_tag_value(token: &str) -> TagValue {
        let quoted_token = format!("\"{}\"", token);
        TagValue {
            token: create_tag_token(&quoted_token),
            children: vec![],
            kind: ValueKind::String,
            spread: None,
            filters: vec![],
            start_index: 0,
            end_index: quoted_token.len(),
            line_col: (1, 1),
        }
    }

    fn create_expr_tag_value(token: &str) -> TagValue {
        let quoted_token = format!("{}", token);
        TagValue {
            token: create_tag_token(&quoted_token),
            children: vec![],
            kind: ValueKind::TemplateString,
            spread: None,
            filters: vec![],
            start_index: 0,
            end_index: quoted_token.len(),
            line_col: (1, 1),
        }
    }

    fn create_trans_tag_value(token: &str) -> TagValue {
        let trans_token = format!("_({})", token);
        TagValue {
            token: create_tag_token(&trans_token),
            children: vec![],
            kind: ValueKind::Translation,
            spread: None,
            filters: vec![],
            start_index: 0,
            end_index: trans_token.len(),
            line_col: (1, 1),
        }
    }

    fn create_int_tag_value(val: i32) -> TagValue {
        let token = val.to_string();
        TagValue {
            token: create_tag_token(&token),
            children: vec![],
            kind: ValueKind::Int,
            spread: None,
            filters: vec![],
            start_index: 0,
            end_index: token.len(),
            line_col: (1, 1),
        }
    }

    fn create_arg_attr(value: TagValue) -> TagAttr {
        TagAttr {
            key: None,
            value,
            is_flag: false,
            start_index: 0,
            end_index: 0, // not important for these tests
            line_col: (1, 1),
        }
    }

    fn create_kwarg_attr(key: &str, value: TagValue) -> TagAttr {
        TagAttr {
            key: Some(create_tag_token(key)),
            value,
            is_flag: false,
            start_index: 0,
            end_index: 0, // not important for these tests
            line_col: (1, 1),
        }
    }

    #[test]
    fn test_no_attributes() {
        let ast = vec![];
        let result = compile_ast_to_string(&ast).unwrap();
        let expected = r#"def compiled_func(context, *, template_string, translation, variable, filter):
    args = []
    kwargs = []
    return args, kwargs"#;
        assert_eq!(result, expected.to_string());
    }

    #[test]
    fn test_single_arg() {
        let ast = vec![create_arg_attr(create_var_tag_value("my_var"))];
        let result = compile_ast_to_string(&ast).unwrap();
        let expected = r#"def compiled_func(context, *, template_string, translation, variable, filter):
    args = []
    kwargs = []
    args.append(variable(context, 'my_var'))
    return args, kwargs"#;
        assert_eq!(result, expected.to_string());
    }

    #[test]
    fn test_multiple_args() {
        let ast = vec![
            create_arg_attr(create_var_tag_value("my_var")),
            create_arg_attr(create_string_tag_value("hello")),
            create_arg_attr(create_int_tag_value(123)),
        ];
        let result = compile_ast_to_string(&ast).unwrap();
        let expected = r#"def compiled_func(context, *, template_string, translation, variable, filter):
    args = []
    kwargs = []
    args.append(variable(context, 'my_var'))
    args.append("hello")
    args.append(123)
    return args, kwargs"#;
        assert_eq!(result, expected.to_string());
    }

    #[test]
    fn test_single_kwarg() {
        let ast = vec![create_kwarg_attr("key", create_var_tag_value("my_var"))];
        let result = compile_ast_to_string(&ast).unwrap();
        let expected = r#"def compiled_func(context, *, template_string, translation, variable, filter):
    args = []
    kwargs = []
    kwargs.append(('key', variable(context, 'my_var')))
    return args, kwargs"#;
        assert_eq!(result, expected.to_string());
    }

    #[test]
    fn test_multiple_kwargs() {
        let ast = vec![
            create_kwarg_attr("key1", create_var_tag_value("my_var")),
            create_kwarg_attr("key2", create_string_tag_value("hello")),
        ];
        let result = compile_ast_to_string(&ast).unwrap();
        let expected = r#"def compiled_func(context, *, template_string, translation, variable, filter):
    args = []
    kwargs = []
    kwargs.append(('key1', variable(context, 'my_var')))
    kwargs.append(('key2', "hello"))
    return args, kwargs"#;
        assert_eq!(result, expected.to_string());
    }

    #[test]
    fn test_mixed_args_and_kwargs() {
        let ast = vec![
            create_arg_attr(create_int_tag_value(42)),
            create_kwarg_attr("key", create_string_tag_value("value")),
        ];
        let result = compile_ast_to_string(&ast).unwrap();
        let expected = r#"def compiled_func(context, *, template_string, translation, variable, filter):
    args = []
    kwargs = []
    args.append(42)
    kwargs.append(('key', "value"))
    return args, kwargs"#;
        assert_eq!(result, expected.to_string());
    }

    #[test]
    fn test_spread_kwargs() {
        let mut spread_value = create_var_tag_value("options");
        spread_value.spread = Some("...".to_string());
        let ast = vec![
            create_arg_attr(spread_value),
            create_kwarg_attr("key", create_string_tag_value("value")),
        ];
        let result = compile_ast_to_string(&ast).unwrap();
        let expected = r#"def compiled_func(context, *, template_string, translation, variable, filter):
    def _handle_spread(value, raw_token_str, args, kwargs, kwarg_seen):
        if hasattr(value, "keys"):
            kwargs.extend(value.items())
            return True
        else:
            if kwarg_seen:
                raise SyntaxError("positional argument follows keyword argument")
            try:
                args.extend(value)
            except TypeError:
                raise TypeError(
                    f"Value of '...{raw_token_str}' must be a mapping or an iterable, "
                    f"not {type(value).__name__}."
                )
            return False

    args = []
    kwargs = []
    kwarg_seen = False
    kwarg_seen = _handle_spread(variable(context, 'options'), """options""", args, kwargs, kwarg_seen)
    kwargs.append(('key', "value"))
    kwarg_seen = True
    return args, kwargs"#;
        assert_eq!(result, expected.to_string());
    }

    #[test]
    fn test_spread_kwargs_order_preserved() {
        let mut spread_value = create_var_tag_value("options");
        spread_value.spread = Some("...".to_string());
        let ast = vec![
            create_kwarg_attr("key1", create_string_tag_value("value1")),
            create_arg_attr(spread_value),
            create_kwarg_attr("key2", create_string_tag_value("value2")),
        ];
        let result = compile_ast_to_string(&ast).unwrap();
        let expected = r#"def compiled_func(context, *, template_string, translation, variable, filter):
    def _handle_spread(value, raw_token_str, args, kwargs, kwarg_seen):
        if hasattr(value, "keys"):
            kwargs.extend(value.items())
            return True
        else:
            if kwarg_seen:
                raise SyntaxError("positional argument follows keyword argument")
            try:
                args.extend(value)
            except TypeError:
                raise TypeError(
                    f"Value of '...{raw_token_str}' must be a mapping or an iterable, "
                    f"not {type(value).__name__}."
                )
            return False

    args = []
    kwargs = []
    kwargs.append(('key1', "value1"))
    kwarg_seen = True
    kwarg_seen = _handle_spread(variable(context, 'options'), """options""", args, kwargs, kwarg_seen)
    kwargs.append(('key2', "value2"))
    return args, kwargs"#;
        assert_eq!(result, expected.to_string());
    }

    #[test]
    fn test_template_string_arg() {
        let ast = vec![create_arg_attr(create_expr_tag_value("\"{{ my_var }}\""))];
        let result = compile_ast_to_string(&ast).unwrap();
        let expected = r#"def compiled_func(context, *, template_string, translation, variable, filter):
    args = []
    kwargs = []
    args.append(template_string(context, "{{ my_var }}"))
    return args, kwargs"#;
        assert_eq!(result, expected.to_string());
    }

    #[test]
    fn test_translation_arg() {
        let ast = vec![create_arg_attr(create_trans_tag_value("\"hello world\""))];
        let result = compile_ast_to_string(&ast).unwrap();
        let expected = r#"def compiled_func(context, *, template_string, translation, variable, filter):
    args = []
    kwargs = []
    args.append(translation(context, "hello world"))
    return args, kwargs"#;
        assert_eq!(result, expected.to_string());
    }

    #[test]
    fn test_filter() {
        let mut value = create_var_tag_value("my_var");
        value.filters.push(TagValueFilter {
            token: create_tag_token("upper"),
            arg: None,
            start_index: 0,
            end_index: 0,
            line_col: (1, 1),
        });
        let ast = vec![create_arg_attr(value)];
        let result = compile_ast_to_string(&ast).unwrap();
        let expected = r#"def compiled_func(context, *, template_string, translation, variable, filter):
    args = []
    kwargs = []
    args.append(filter(context, 'upper', variable(context, 'my_var'), None))
    return args, kwargs"#;
        assert_eq!(result, expected.to_string());
    }

    #[test]
    fn test_filter_with_arg() {
        let mut value = create_var_tag_value("my_var");
        value.filters.push(TagValueFilter {
            token: create_tag_token("default"),
            arg: Some(create_string_tag_value("none")),
            start_index: 0,
            end_index: 0,
            line_col: (1, 1),
        });
        let ast = vec![create_arg_attr(value)];
        let result = compile_ast_to_string(&ast).unwrap();
        let expected = r#"def compiled_func(context, *, template_string, translation, variable, filter):
    args = []
    kwargs = []
    args.append(filter(context, 'default', variable(context, 'my_var'), "none"))
    return args, kwargs"#;
        assert_eq!(result, expected.to_string());
    }

    #[test]
    fn test_multiple_filters() {
        let mut value = create_var_tag_value("my_var");
        value.filters.push(TagValueFilter {
            token: create_tag_token("upper"),
            arg: None,
            start_index: 0,
            end_index: 0,
            line_col: (1, 1),
        });
        value.filters.push(TagValueFilter {
            token: create_tag_token("default"),
            arg: Some(create_string_tag_value("none")),
            start_index: 0,
            end_index: 0,
            line_col: (1, 1),
        });
        let ast = vec![create_arg_attr(value)];
        let result = compile_ast_to_string(&ast).unwrap();
        let expected = r#"def compiled_func(context, *, template_string, translation, variable, filter):
    args = []
    kwargs = []
    args.append(filter(context, 'default', filter(context, 'upper', variable(context, 'my_var'), None), "none"))
    return args, kwargs"#;
        assert_eq!(result, expected.to_string());
    }

    #[test]
    fn test_list_value() {
        let list_value = TagValue {
            token: create_tag_token("[1, my_var]"),
            children: vec![create_int_tag_value(1), create_var_tag_value("my_var")],
            kind: ValueKind::List,
            spread: None,
            filters: vec![],
            start_index: 0,
            end_index: 0,
            line_col: (1, 1),
        };
        let ast = vec![create_arg_attr(list_value)];
        let result = compile_ast_to_string(&ast).unwrap();
        let expected = r#"def compiled_func(context, *, template_string, translation, variable, filter):
    args = []
    kwargs = []
    args.append([1, variable(context, 'my_var')])
    return args, kwargs"#;
        assert_eq!(result, expected.to_string());
    }

    #[test]
    fn test_dict_value() {
        let dict_value = TagValue {
            token: create_tag_token("{'key': my_var}"),
            children: vec![
                create_string_tag_value("key"),
                create_var_tag_value("my_var"),
            ],
            kind: ValueKind::Dict,
            spread: None,
            filters: vec![],
            start_index: 0,
            end_index: 0,
            line_col: (1, 1),
        };
        let ast = vec![create_kwarg_attr("data", dict_value)];
        let result = compile_ast_to_string(&ast).unwrap();
        let expected = r#"def compiled_func(context, *, template_string, translation, variable, filter):
    args = []
    kwargs = []
    kwargs.append(('data', {"key": variable(context, 'my_var')}))
    return args, kwargs"#;
        assert_eq!(result, expected.to_string());
    }

    #[test]
    fn test_multiple_spreads() {
        let mut spread_value_1 = create_var_tag_value("options1");
        spread_value_1.spread = Some("...".to_string());
        let mut spread_value_2 = create_var_tag_value("options2");
        spread_value_2.spread = Some("...".to_string());

        let ast = vec![
            create_kwarg_attr("key1", create_string_tag_value("value1")),
            create_arg_attr(spread_value_1),
            create_kwarg_attr("key2", create_string_tag_value("value2")),
            create_arg_attr(spread_value_2),
        ];
        let result = compile_ast_to_string(&ast).unwrap();

        let expected = r#"def compiled_func(context, *, template_string, translation, variable, filter):
    def _handle_spread(value, raw_token_str, args, kwargs, kwarg_seen):
        if hasattr(value, "keys"):
            kwargs.extend(value.items())
            return True
        else:
            if kwarg_seen:
                raise SyntaxError("positional argument follows keyword argument")
            try:
                args.extend(value)
            except TypeError:
                raise TypeError(
                    f"Value of '...{raw_token_str}' must be a mapping or an iterable, "
                    f"not {type(value).__name__}."
                )
            return False

    args = []
    kwargs = []
    kwargs.append(('key1', "value1"))
    kwarg_seen = True
    kwarg_seen = _handle_spread(variable(context, 'options1'), """options1""", args, kwargs, kwarg_seen)
    kwargs.append(('key2', "value2"))
    kwarg_seen = _handle_spread(variable(context, 'options2'), """options2""", args, kwargs, kwarg_seen)
    return args, kwargs"#;
        assert_eq!(result, expected.to_string());
    }

    #[test]
    fn test_compiler_skips_flags() {
        let mut flag_attr = create_arg_attr(create_var_tag_value("disabled"));
        flag_attr.is_flag = true;

        let ast = vec![
            create_kwarg_attr("key", create_string_tag_value("value")),
            flag_attr,
        ];
        let result = compile_ast_to_string(&ast).unwrap();
        let expected = r#"def compiled_func(context, *, template_string, translation, variable, filter):
    args = []
    kwargs = []
    kwargs.append(('key', "value"))
    return args, kwargs"#;
        assert_eq!(result, expected.to_string());
    }

    #[test]
    fn test_positional_after_keyword_error() {
        let ast = vec![
            create_kwarg_attr("key", create_string_tag_value("value")),
            create_arg_attr(create_int_tag_value(42)),
        ];
        let result = compile_ast_to_string(&ast);
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            CompileError::from("positional argument follows keyword argument")
        );
    }

    // ###########################################
    // PARAMETER ORDERING TESTS
    // ###########################################

    #[test]
    fn test_arg_after_kwarg_parses_but_compiler_should_error() {
        // The parser should allow this syntax, but the compiler should raise an error
        let input = r#"{% component key="value" positional_arg %}"#;
        let tag = TagParser::parse_tag(input, &HashSet::new()).unwrap();
        let result = compile_ast_to_string(&tag.attrs);

        assert!(result.is_err());
        let error = result.unwrap_err();
        assert_eq!(error, CompileError::from("positional argument follows keyword argument"));
    }

    #[test]
    fn test_arg_after_spread_parses_and_compiles() {
        // Altho we can see that we're spreading a literal dict,
        // spreads are evaluated only at runtime, so this should parse and compile.
        let input = r#"{% component ...{"key": "value"} positional_arg %}"#;
        let result = TagParser::parse_tag(input, &HashSet::new());

        assert!(!result.is_err());
    }

    #[test]
    fn test_kwarg_after_spread_parse_and_compiles() {
        // This is totally fine
        let input = r#"{% component ...[1, 2, 3] key="value" %}"#;
        let tag = TagParser::parse_tag(input, &HashSet::new()).unwrap();
        let result = compile_ast_to_string(&tag.attrs);

        assert!(!result.is_err());
    }
}
