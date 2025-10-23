//! # Django Template Tag Parser
//!
//! This module converts Django template tag strings (e.g. `{% component %}`) into an Abstract Syntax Tree (AST).
//! using [Pest](https://pest.rs/) parsing library.
//!
//! The parsing grammar is defined in `grammar.pest` and supports:
//!
//! ## Features
//!
//! - **Complex value types**: strings, numbers, variables, template_strings, translations, lists, dicts
//! - **Filter chains**: `value|filter1|filter2:arg`
//! - **Spread operators**: `...list` and `**dict`
//! - **Comments**: `{# comment #}` within tag content
//! - **Position tracking**: line/column information for error reporting
//! - **Template string detection**: identifies strings with Django template tags inside them
//! - Can be easily extended to support HTML syntax `<my_tag key=value />`
//!
//! ## Error Handling
//!
//! The parser returns `ParseError` for invalid input, which includes:
//! - Pest parsing errors (syntax violations)
//! - Invalid key errors (for malformed attributes)
//! - Automatic conversion to Python `ValueError` for PyO3 integration

use crate::ast::{Tag, TagAttr, TagSyntax, TagToken, TagValue, TagValueFilter, ValueKind};
use lazy_static;
use pest::Parser;
use pest_derive::Parser;
use regex;
use std::collections::HashSet;
use thiserror::Error;

#[derive(Parser)]
#[grammar = "grammar.pest"]
pub struct TagParser;

#[derive(Error, Debug)]
pub enum ParseError {
    #[error("Pest parser error: {0}")]
    PestError(#[from] pest::error::Error<Rule>),
    #[error("Invalid key: {0}")]
    InvalidKey(String),
}

impl TagParser {
    pub fn parse_tag(input: &str, flags: &HashSet<String>) -> Result<Tag, ParseError> {
        let wrapper_pair = Self::parse(Rule::tag_wrapper, input)?
            .next()
            .ok_or_else(|| {
                ParseError::PestError(pest::error::Error::new_from_span(
                    pest::error::ErrorVariant::CustomError {
                        message: "Empty tag content".to_string(),
                    },
                    pest::Span::new(input, 0, 0).unwrap(),
                ))
            })?;

        // Get the span from wrapper_pair before moving it
        let span = wrapper_pair.as_span();
        let start_index = span.start();
        let end_index = span.end();

        // Descend into tag_wrapper -> (django_tag | html_tag)
        let tag_pair = wrapper_pair.into_inner().next().unwrap();

        let syntax = match tag_pair.as_rule() {
            Rule::django_tag => TagSyntax::Django,
            // Rule::html_tag => TagSyntax::Html, // Uncomment to enable HTML syntax `<my_tag key=value />`
            _ => unreachable!("Expected django_tag"),
        };

        // Descend into (django_tag | html_tag) -> tag_content
        let tag_content_pair = tag_pair.into_inner().next().unwrap();

        let line_col = tag_content_pair.line_col();

        let mut inner_pairs = tag_content_pair.into_inner();

        // First item in a tag is always the tag name
        let name_pair = inner_pairs
            .next()
            .ok_or_else(|| ParseError::InvalidKey("Tag is empty".to_string()))?;
        if name_pair.as_rule() != Rule::tag_name {
            return Err(ParseError::InvalidKey(format!(
                "Expected tag_name, found rule {:?}",
                name_pair.as_rule()
            )));
        }

        let name_span = name_pair.as_span();
        let name = TagToken {
            token: name_pair.as_str().to_string(),
            start_index: name_span.start(),
            end_index: name_span.end(),
            line_col: name_pair.line_col(),
        };

        let mut attributes = Vec::new();
        let mut seen_flags = HashSet::new();
        let mut is_self_closing = false;

        // Parse the attributes
        for pair in inner_pairs {
            match pair.as_rule() {
                Rule::attribute => {
                    let mut attr = Self::process_attribute(pair)?;

                    // Check if this is a flag
                    if attr.key.is_none() && attr.value.spread.is_none() {
                        let token = &attr.value.token.token;
                        if flags.contains(token) {
                            attr.is_flag = true;
                            if !seen_flags.insert(token.clone()) {
                                return Err(ParseError::InvalidKey(format!(
                                    "Flag '{}' may be specified only once.",
                                    token
                                )));
                            }
                        }
                    }

                    attributes.push(attr);
                }
                Rule::self_closing_slash => {
                    is_self_closing = true;
                }
                _ => { /* Spacing and comments are silent and won't appear here */ }
            }
        }

        Ok(Tag {
            name,
            attrs: attributes,
            is_self_closing,
            syntax,
            start_index,
            end_index,
            line_col,
        })
    }

    fn process_attribute(attr_pair: pest::iterators::Pair<Rule>) -> Result<TagAttr, ParseError> {
        let start_index = attr_pair.as_span().start();
        let line_col = attr_pair.line_col();

        let _attr_str = attr_pair.as_str().to_string(); // Clone the string before moving the pair
        let mut inner_pairs = attr_pair.into_inner().peekable();

        // println!("Processing attribute: {:?}", attr_str);
        // if let Some(next_rule) = inner_pairs.peek() {
        //     println!("Next rule: {:?}", next_rule.as_rule());
        // }

        // Check if this is a key-value pair or just a value
        match inner_pairs.peek().map(|p| p.as_rule()) {
            Some(Rule::key) => {
                // println!("Found key-value pair");

                // Key
                let key_pair = inner_pairs.next().unwrap();
                let key_value = key_pair.as_str().to_string();
                let key_end_index = key_pair.as_span().end();

                // Value
                let value_pair = inner_pairs
                    .filter(|p| p.as_rule() == Rule::filtered_value)
                    .next()
                    .ok_or_else(|| {
                        ParseError::InvalidKey(format!("Missing value for key: {}", key_value))
                    })?;

                let value = Self::process_filtered_value(value_pair)?;
                let value_end_index = value.end_index;

                Ok(TagAttr {
                    key: Some(TagToken {
                        token: key_value,
                        start_index,
                        end_index: key_end_index,
                        line_col,
                    }),
                    value,
                    is_flag: false,
                    start_index,
                    end_index: value_end_index,
                    line_col,
                })
            }
            Some(Rule::spread_value) => {
                // println!("Found spread value");

                // Spread value form
                let spread_value = inner_pairs.next().unwrap();

                // println!("Spread value: {:?}", spread_value.as_str());
                // println!("Spread value rule: {:?}", spread_value.as_rule());

                // Get the value part after the ... operator
                let mut value_pairs = spread_value.into_inner();
                let value_pair = value_pairs.next().unwrap();

                // println!("Value pair: {:?}", value_pair.as_str());
                // println!("Value pair rule: {:?}", value_pair.as_rule());

                // Process the value part
                let mut value = match value_pair.as_rule() {
                    Rule::filtered_value => Self::process_filtered_value(value_pair)?,
                    other => {
                        return Err(ParseError::InvalidKey(format!(
                            "Expected filtered_value after spread operator, got {:?}",
                            other
                        )))
                    }
                };

                // Update indices
                value.spread = Some("...".to_string());
                value.start_index -= 3;
                value.line_col = (value.line_col.0, value.line_col.1 - 3);

                let end_index = value.end_index;

                Ok(TagAttr {
                    key: None,
                    value,
                    is_flag: false,
                    start_index,
                    end_index,
                    line_col,
                })
            }
            Some(Rule::filtered_value) => {
                // println!("Found filtered value");

                let value_pair = inner_pairs.next().unwrap();
                let value = Self::process_filtered_value(value_pair)?;
                let end_index = value.end_index;

                Ok(TagAttr {
                    key: None,
                    value,
                    is_flag: false,
                    start_index,
                    end_index,
                    line_col,
                })
            }
            _ => unreachable!("Invalid attribute structure"),
        }
    }

    // Filtered value means that:
    // 1. It is "value" - meaning that it is the same as "basic value" + list and dict
    // 2. It may have a filter chain after it
    //
    // E.g. `my_var`, `my_var|filter`, `[1, 2, 3]|filter1|filter2` are all filtered values
    fn process_filtered_value(
        value_pair: pest::iterators::Pair<Rule>,
    ) -> Result<TagValue, ParseError> {
        // println!("Processing value: {:?}", value_pair.as_str());
        // println!("Rule: {:?}", value_pair.as_rule());

        let total_span = value_pair.as_span();
        let total_start_index = total_span.start();
        let total_end_index = total_span.end();
        let total_line_col = value_pair.line_col();

        let mut inner_pairs = value_pair.into_inner();

        // Get the main value part
        let value_part = inner_pairs.next().unwrap();

        // println!("Value part rule: {:?}", value_part.as_rule());
        // println!("Value part text: {:?}", value_part.as_str());
        // println!("Inner pairs of value_part:");
        // for pair in value_part.clone().into_inner() {
        //     println!("  Rule: {:?}, Text: {:?}", pair.as_rule(), pair.as_str());
        // }

        let mut result = match value_part.as_rule() {
            Rule::value => {
                // Get the actual value (stripping the * if present)
                let mut inner_pairs = value_part.clone().into_inner();
                let inner_value = inner_pairs.next().unwrap();

                // println!(
                //     "  Inner value rule: {:?}, Text: {:?}",
                //     inner_value.as_rule(),
                //     inner_value.as_str()
                // );

                // Process the value
                match inner_value.as_rule() {
                    Rule::list => {
                        let list_str = inner_value.as_str().to_string();

                        // println!("  Processing list: {:?}", list_str);

                        let span = inner_value.as_span();
                        let token_start_index = span.start();
                        let token_end_index = span.end();
                        let token_line_col = inner_value.line_col();

                        let children = Self::process_list(inner_value)?;

                        Ok(TagValue {
                            token: TagToken {
                                token: list_str,
                                start_index: token_start_index,
                                end_index: token_end_index,
                                line_col: token_line_col,
                            },
                            spread: None,
                            filters: vec![],
                            kind: ValueKind::List,
                            children,
                            start_index: total_start_index,
                            end_index: total_end_index,
                            line_col: total_line_col,
                        })
                    }
                    Rule::dict => {
                        let dict_str = inner_value.as_str().to_string();

                        // println!("  Processing dict: {:?}", dict_str);

                        let span = inner_value.as_span();
                        let token_start_index = span.start();
                        let token_end_index = span.end();
                        let token_line_col = inner_value.line_col();

                        let children = Self::process_dict(inner_value)?;

                        Ok(TagValue {
                            token: TagToken {
                                token: dict_str,
                                start_index: token_start_index,
                                end_index: token_end_index,
                                line_col: token_line_col,
                            },
                            spread: None,
                            filters: vec![],
                            kind: ValueKind::Dict,
                            children,
                            start_index: total_start_index,
                            end_index: total_end_index,
                            line_col: total_line_col,
                        })
                    }
                    _ => {
                        let mut result = Self::process_dict_key_inner(inner_value);

                        // Update indices
                        result = result.map(|mut tag_value| {
                            tag_value.start_index = total_start_index;
                            tag_value.end_index = total_end_index;
                            tag_value.line_col = total_line_col;
                            tag_value
                        });

                        result
                    }
                }
            }
            other => Err(ParseError::InvalidKey(format!(
                "Expected value, got {:?}",
                other
            ))),
        };

        // Process any filters
        if let Some(filter_chain) = inner_pairs.next() {
            result = result.and_then(|mut tag_value| {
                tag_value.filters = Self::process_filters(filter_chain)?;
                Ok(tag_value)
            });
        }

        result
    }

    // The value of a dict key is a string, number, or i18n string.
    // It cannot be dicts nor lists because keys must be hashable.
    //
    // NOTE: Basic value is NOT a filtered value
    //
    // E.g. `my_var`, `42`, `"hello world"`, `_("hello world")` are all basic values
    fn process_dict_key_inner(
        value_pair: pest::iterators::Pair<Rule>,
    ) -> Result<TagValue, ParseError> {
        // println!(
        //     "Processing basic value: Rule={:?}, Text={:?}",
        //     value_pair.as_rule(),
        //     value_pair.as_str()
        // );

        let start_index = value_pair.as_span().start();
        let end_index = value_pair.as_span().end();
        let line_col = value_pair.line_col();

        // Determine the value kind, so that downstream processing doesn't need to
        let text = value_pair.as_str();
        let kind = match value_pair.as_rule() {
            Rule::i18n_string => ValueKind::Translation,
            Rule::string_literal => {
                if Self::has_template_string(text) {
                    ValueKind::TemplateString
                } else {
                    ValueKind::String
                }
            }
            Rule::int => ValueKind::Int,
            Rule::float => ValueKind::Float,
            Rule::variable => ValueKind::Variable,
            _ => unreachable!("Invalid basic value {:?}", value_pair.as_rule()),
        };

        // If this is an i18n string, remove the whitespace between `_()` and the text
        let mut text = text.to_string();
        if kind == ValueKind::Translation {
            // Find the first occurrence of either quote type
            let single_quote_pos = text.find('\'');
            let double_quote_pos = text.find('"');

            // Select the quote char that appears first
            let quote_char = match (single_quote_pos, double_quote_pos) {
                // If both quotes are present, use the one that appears first
                (Some(s), Some(d)) if s < d => '\'',
                (Some(_), Some(_)) => '"',
                // If only one quote is present, use it
                (Some(_), None) => '\'',
                (None, Some(_)) => '"',
                // If no quotes are present, return an error
                (None, None) => {
                    return Err(ParseError::InvalidKey(
                        "No quotes found in i18n string".to_string(),
                    ))
                }
            };

            let start = text.find(quote_char).unwrap();
            let end = text.rfind(quote_char).unwrap();
            let quoted_part = &text[start..=end];
            text = format!("_({})", quoted_part);
        }

        Ok(TagValue {
            token: TagToken {
                token: text.to_string(),
                start_index,
                end_index,
                line_col,
            },
            spread: None,
            filters: vec![],
            kind,
            children: vec![],
            line_col,
            start_index,
            end_index,
        })
    }

    // Process a key in a dict that may have filters
    fn process_filtered_dict_key(
        value_pair: pest::iterators::Pair<Rule>,
    ) -> Result<TagValue, ParseError> {
        // println!(
        //     "Processing filtered basic value: Rule={:?}, Text={:?}",
        //     value_pair.as_rule(),
        //     value_pair.as_str()
        // );

        let total_span = value_pair.as_span();
        let total_start_index = total_span.start();
        let total_end_index = total_span.end();
        let total_line_col = value_pair.line_col();

        let mut inner_pairs = value_pair.into_inner();
        let dict_key_inner = inner_pairs.next().unwrap();
        let mut result = Self::process_dict_key_inner(dict_key_inner);

        // Update indices
        result = result.map(|mut tag_value| {
            tag_value.start_index = total_start_index;
            tag_value.end_index = total_end_index;
            tag_value.line_col = total_line_col;
            tag_value
        });

        // Process any filters
        if let Some(filter_chain) = inner_pairs.next() {
            result = result.and_then(|mut tag_value| {
                tag_value.filters = Self::process_filters(filter_chain)?;
                Ok(tag_value)
            });
        }

        result
    }

    fn process_list(inner_value: pest::iterators::Pair<Rule>) -> Result<Vec<TagValue>, ParseError> {
        let mut items = Vec::new();
        for item in inner_value.into_inner() {
            // println!(
            //     "    ALL list tokens: Rule={:?}, Text={:?}",
            //     item.as_rule(),
            //     item.as_str()
            // );

            if item.as_rule() == Rule::list_item {
                let has_spread = item.as_str().starts_with('*');

                // println!("      List item inner tokens:");

                for inner in item.clone().into_inner() {
                    // println!(
                    //     "        Rule={:?}, Text={:?}",
                    //     inner.as_rule(),
                    //     inner.as_str()
                    // );

                    if inner.as_rule() == Rule::filtered_value {
                        let mut tag_value = Self::process_filtered_value(inner)?;

                        // Update indices
                        if has_spread {
                            tag_value.spread = Some("*".to_string());
                            tag_value.start_index -= 1;
                            tag_value.line_col = (tag_value.line_col.0, tag_value.line_col.1 - 1);
                        }
                        items.push(tag_value);
                    }
                }
            }
        }
        Ok(items)
    }

    fn process_dict(dict_pair: pest::iterators::Pair<Rule>) -> Result<Vec<TagValue>, ParseError> {
        let mut items = Vec::new();
        for item in dict_pair.into_inner() {
            // println!(
            //     "    ALL dict tokens: Rule={:?}, Text={:?}",
            //     item.as_rule(),
            //     item.as_str()
            // );

            match item.as_rule() {
                Rule::dict_item_pair => {
                    let mut inner = item.into_inner();
                    let key_pair = inner.next().unwrap();
                    let mut value_pair = inner.next().unwrap();

                    // Skip comments in dict items
                    while value_pair.as_rule() == Rule::COMMENT {
                        value_pair = inner.next().unwrap();
                    }

                    // println!(
                    //     "    dict_item_pair: Key={:?}, Value={:?}",
                    //     key_pair.as_str(),
                    //     value_pair.as_str()
                    // );

                    let key = Self::process_filtered_dict_key(key_pair)?;
                    let value = Self::process_filtered_value(value_pair)?;

                    // println!(
                    //     "    dict_item_pair(parsed): Key={:?}, Value={:?}",
                    //     key.token, value.token
                    // );

                    // Check that key is not a list or dict
                    match key.kind {
                        ValueKind::List | ValueKind::Dict => {
                            return Err(ParseError::InvalidKey(
                                "Dictionary keys cannot be lists or dictionaries".to_string(),
                            ));
                        }
                        _ => {}
                    }
                    items.push(key);
                    items.push(value);
                }
                Rule::dict_item_spread => {
                    let mut inner = item.into_inner();
                    let mut value_pair = inner.next().unwrap();

                    // println!("    dict_item_spread: Value={:?}", inner.as_str());

                    // Skip comments in dict items
                    while value_pair.as_rule() == Rule::COMMENT {
                        value_pair = inner.next().unwrap();
                    }

                    let mut value = Self::process_filtered_value(value_pair)?;

                    // Update indices
                    value.spread = Some("**".to_string());
                    value.start_index -= 2;
                    value.line_col = (value.line_col.0, value.line_col.1 - 2);

                    // println!("    dict_item_spread(parsed): Value={:?}", value.token);

                    items.push(value);
                }
                Rule::COMMENT => {}
                _ => unreachable!("Invalid dictionary item {:?}", item.as_rule()),
            }
        }
        Ok(items)
    }
    fn process_filters(
        filter_chain: pest::iterators::Pair<Rule>,
    ) -> Result<Vec<TagValueFilter>, ParseError> {
        // Return error if not a filter chain rule
        if filter_chain.as_rule() != Rule::filter_chain
            && filter_chain.as_rule() != Rule::filter_chain_noarg
        {
            return Err(ParseError::InvalidKey(format!(
                "Expected filter chain, got {:?}",
                filter_chain.as_rule()
            )));
        }

        let mut filters = Vec::new();

        // println!(
        //     "Found rule {:?}, processing filters...",
        //     filter_chain.as_rule()
        // );

        for filter in filter_chain.into_inner() {
            // Skip comments
            if filter.as_rule() == Rule::COMMENT {
                continue;
            }

            // println!("Processing filter: {:?}", filter.as_str());

            if filter.as_rule() != Rule::filter && filter.as_rule() != Rule::filter_noarg {
                return Err(ParseError::InvalidKey(format!(
                    "Expected filter, got {:?}",
                    filter.as_rule()
                )));
            }

            let filter_span = filter.as_span();
            let filter_start_index = filter_span.start();
            let filter_end_index = filter_span.end();
            let filter_line_col = filter.line_col();

            // Find the filter name (skipping the pipe token)
            let mut filter_parts = filter.into_inner();
            let filter_pair = filter_parts
                .find(|p| p.as_rule() == Rule::filter_name)
                .unwrap();
            let filter_name = filter_pair.as_str().to_string();
            let token_start_index = filter_pair.as_span().start();
            let token_end_index = filter_pair.as_span().end();
            let token_line_col = filter_pair.line_col();

            // println!("Found filter name: {:?}", filter_name);

            let filter_arg = if let Some(arg_part) =
                filter_parts.find(|p| p.as_rule() == Rule::filter_arg_part)
            {
                // Position, includeing the `:`
                let arg_span = arg_part.as_span();
                let arg_start_index = arg_span.start();
                let arg_end_index = arg_span.end();
                let arg_line_col = arg_part.line_col();

                let arg_value_pair: pest::iterators::Pair<'_, Rule> = arg_part
                    .into_inner()
                    .find(|p| p.as_rule() == Rule::filter_arg)
                    .unwrap();

                // Process the filter argument as a TagValue
                let mut result = Self::process_filtered_value(arg_value_pair)?;

                // Update indices
                result.start_index = arg_start_index;
                result.end_index = arg_end_index;
                result.line_col = arg_line_col;
                Some(result)
            } else {
                None
            };

            filters.push(TagValueFilter {
                arg: filter_arg,
                token: TagToken {
                    token: filter_name,
                    start_index: token_start_index,
                    end_index: token_end_index,
                    line_col: token_line_col,
                },
                start_index: filter_start_index,
                end_index: filter_end_index,
                line_col: filter_line_col,
            });

            // println!("Added filter to chain: {:?}", filters.last().unwrap());
        }

        // println!(
        //     "Completed processing filter chain, returning {:?} filters",
        //     filters.len()
        // );

        Ok(filters)
    }

    fn has_template_string(s: &str) -> bool {
        // Don't check for template strings in i18n strings
        if s.starts_with("_(") {
            return false;
        }

        // Check for any of the Django template tags with their closing tags
        // The pattern ensures that:
        // 1. Opening and closing tags are properly paired
        // 2. Tags are in the correct order (no closing before opening)
        lazy_static::lazy_static! {
            static ref VAR_TAG: regex::Regex = regex::Regex::new(r"\{\{.*?\}\}").unwrap();
            static ref BLOCK_TAG: regex::Regex = regex::Regex::new(r"\{%.*?%\}").unwrap();
            static ref COMMENT_TAG: regex::Regex = regex::Regex::new(r"\{#.*?#\}").unwrap();
        }

        VAR_TAG.is_match(s) || BLOCK_TAG.is_match(s) || COMMENT_TAG.is_match(s)
    }
}

#[cfg(test)]
mod tests {
    use std::vec;

    use super::*;

    #[test]
    fn test_arg_single_variable() {
        // Test simple variable name
        let input = "{% my_tag val %}";
        let result = TagParser::parse_tag(input, &HashSet::new()).unwrap();
        assert_eq!(
            result,
            Tag {
                name: TagToken {
                    token: "my_tag".to_string(),
                    start_index: 3,
                    end_index: 9,
                    line_col: (1, 4),
                },
                attrs: vec![TagAttr {
                    key: None,
                    value: TagValue {
                        token: TagToken {
                            token: "val".to_string(),
                            start_index: 10,
                            end_index: 13,
                            line_col: (1, 11),
                        },
                        children: vec![],
                        spread: None,
                        filters: vec![],
                        kind: ValueKind::Variable,
                        start_index: 10,
                        end_index: 13,
                        line_col: (1, 11),
                    },
                    is_flag: false,
                    start_index: 10,
                    end_index: 13,
                    line_col: (1, 11),
                },],
                is_self_closing: false,
                syntax: TagSyntax::Django,
                start_index: 0,
                end_index: 16,
                line_col: (1, 4),
            }
        );
    }

    #[test]
    fn test_arg_single_variable_with_dots() {
        // Test variable with dots
        let input = "{% my_tag my.nested.value %}";
        let result = TagParser::parse_tag(input, &HashSet::new()).unwrap();
        assert_eq!(
            result,
            Tag {
                name: TagToken {
                    token: "my_tag".to_string(),
                    start_index: 3,
                    end_index: 9,
                    line_col: (1, 4),
                },
                attrs: vec![TagAttr {
                    key: None,
                    value: TagValue {
                        token: TagToken {
                            token: "my.nested.value".to_string(),
                            start_index: 10,
                            end_index: 25,
                            line_col: (1, 11),
                        },
                        children: vec![],
                        spread: None,
                        filters: vec![],
                        kind: ValueKind::Variable,
                        start_index: 10,
                        end_index: 25,
                        line_col: (1, 11),
                    },
                    is_flag: false,
                    start_index: 10,
                    end_index: 25,
                    line_col: (1, 11),
                },],
                is_self_closing: false,
                syntax: TagSyntax::Django,
                start_index: 0,
                end_index: 28,
                line_col: (1, 4),
            }
        );
    }

    #[test]
    fn test_arg_single_number_1() {
        let input = "{% my_tag 42 %}";
        let result = TagParser::parse_tag(input, &HashSet::new()).unwrap();
        assert_eq!(
            result,
            Tag {
                name: TagToken {
                    token: "my_tag".to_string(),
                    start_index: 3,
                    end_index: 9,
                    line_col: (1, 4),
                },
                attrs: vec![TagAttr {
                    key: None,
                    value: TagValue {
                        token: TagToken {
                            token: "42".to_string(),
                            start_index: 10,
                            end_index: 12,
                            line_col: (1, 11),
                        },
                        children: vec![],
                        spread: None,
                        filters: vec![],
                        kind: ValueKind::Int,
                        start_index: 10,
                        end_index: 12,
                        line_col: (1, 11),
                    },
                    is_flag: false,
                    start_index: 10,
                    end_index: 12,
                    line_col: (1, 11),
                }],
                is_self_closing: false,
                syntax: TagSyntax::Django,
                start_index: 0,
                end_index: 15,
                line_col: (1, 4),
            }
        );
    }

    #[test]
    fn test_arg_single_number_2() {
        let input = "{% my_tag 001 %}";
        let result = TagParser::parse_tag(input, &HashSet::new()).unwrap();
        assert_eq!(
            result,
            Tag {
                name: TagToken {
                    token: "my_tag".to_string(),
                    start_index: 3,
                    end_index: 9,
                    line_col: (1, 4),
                },
                attrs: vec![TagAttr {
                    key: None,
                    value: TagValue {
                        token: TagToken {
                            token: "001".to_string(),
                            start_index: 10,
                            end_index: 13,
                            line_col: (1, 11),
                        },
                        children: vec![],
                        spread: None,
                        filters: vec![],
                        kind: ValueKind::Int,
                        start_index: 10,
                        end_index: 13,
                        line_col: (1, 11),
                    },
                    is_flag: false,
                    start_index: 10,
                    end_index: 13,
                    line_col: (1, 11),
                }],
                is_self_closing: false,
                syntax: TagSyntax::Django,
                start_index: 0,
                end_index: 16,
                line_col: (1, 4),
            }
        );
    }

    #[test]
    fn test_arg_single_number_with_decimal_1() {
        let input = "{% my_tag -1.5 %}";
        let result = TagParser::parse_tag(input, &HashSet::new()).unwrap();
        assert_eq!(
            result,
            Tag {
                name: TagToken {
                    token: "my_tag".to_string(),
                    start_index: 3,
                    end_index: 9,
                    line_col: (1, 4),
                },
                attrs: vec![TagAttr {
                    key: None,
                    value: TagValue {
                        token: TagToken {
                            token: "-1.5".to_string(),
                            start_index: 10,
                            end_index: 14,
                            line_col: (1, 11),
                        },
                        children: vec![],
                        spread: None,
                        filters: vec![],
                        kind: ValueKind::Float,
                        start_index: 10,
                        end_index: 14,
                        line_col: (1, 11),
                    },
                    is_flag: false,
                    start_index: 10,
                    end_index: 14,
                    line_col: (1, 11),
                }],
                is_self_closing: false,
                syntax: TagSyntax::Django,
                start_index: 0,
                end_index: 17,
                line_col: (1, 4),
            }
        );
    }

    #[test]
    fn test_arg_single_number_with_decimal_2() {
        let input = "{% my_tag +2. %}";
        let result = TagParser::parse_tag(input, &HashSet::new()).unwrap();
        assert_eq!(
            result,
            Tag {
                name: TagToken {
                    token: "my_tag".to_string(),
                    start_index: 3,
                    end_index: 9,
                    line_col: (1, 4),
                },
                attrs: vec![TagAttr {
                    key: None,
                    value: TagValue {
                        token: TagToken {
                            token: "+2.".to_string(),
                            start_index: 10,
                            end_index: 13,
                            line_col: (1, 11),
                        },
                        children: vec![],
                        spread: None,
                        filters: vec![],
                        kind: ValueKind::Float,
                        start_index: 10,
                        end_index: 13,
                        line_col: (1, 11),
                    },
                    is_flag: false,
                    start_index: 10,
                    end_index: 13,
                    line_col: (1, 11),
                }],
                is_self_closing: false,
                syntax: TagSyntax::Django,
                start_index: 0,
                end_index: 16,
                line_col: (1, 4),
            }
        );
    }

    #[test]
    fn test_arg_single_number_with_decimal_3() {
        let input = "{% my_tag .3 %}";
        let result = TagParser::parse_tag(input, &HashSet::new()).unwrap();
        assert_eq!(
            result,
            Tag {
                name: TagToken {
                    token: "my_tag".to_string(),
                    start_index: 3,
                    end_index: 9,
                    line_col: (1, 4),
                },
                attrs: vec![TagAttr {
                    key: None,
                    value: TagValue {
                        token: TagToken {
                            token: ".3".to_string(),
                            start_index: 10,
                            end_index: 12,
                            line_col: (1, 11),
                        },
                        children: vec![],
                        spread: None,
                        filters: vec![],
                        kind: ValueKind::Float,
                        start_index: 10,
                        end_index: 12,
                        line_col: (1, 11),
                    },
                    is_flag: false,
                    start_index: 10,
                    end_index: 12,
                    line_col: (1, 11),
                }],
                is_self_closing: false,
                syntax: TagSyntax::Django,
                start_index: 0,
                end_index: 15,
                line_col: (1, 4),
            }
        );
    }

    #[test]
    fn test_arg_single_number_scientific_1() {
        let input = "{% my_tag -1.2e2 %}";
        let result = TagParser::parse_tag(input, &HashSet::new()).unwrap();
        assert_eq!(
            result,
            Tag {
                name: TagToken {
                    token: "my_tag".to_string(),
                    start_index: 3,
                    end_index: 9,
                    line_col: (1, 4),
                },
                attrs: vec![TagAttr {
                    key: None,
                    value: TagValue {
                        token: TagToken {
                            token: "-1.2e2".to_string(),
                            start_index: 10,
                            end_index: 16,
                            line_col: (1, 11),
                        },
                        children: vec![],
                        spread: None,
                        filters: vec![],
                        kind: ValueKind::Float,
                        start_index: 10,
                        end_index: 16,
                        line_col: (1, 11),
                    },
                    is_flag: false,
                    start_index: 10,
                    end_index: 16,
                    line_col: (1, 11),
                }],
                is_self_closing: false,
                syntax: TagSyntax::Django,
                start_index: 0,
                end_index: 19,
                line_col: (1, 4),
            }
        );
    }

    #[test]
    fn test_arg_single_number_scientific_2() {
        let input = "{% my_tag .2e-02 %}";
        let result = TagParser::parse_tag(input, &HashSet::new()).unwrap();
        assert_eq!(
            result,
            Tag {
                name: TagToken {
                    token: "my_tag".to_string(),
                    start_index: 3,
                    end_index: 9,
                    line_col: (1, 4),
                },
                attrs: vec![TagAttr {
                    key: None,
                    value: TagValue {
                        token: TagToken {
                            token: ".2e-02".to_string(),
                            start_index: 10,
                            end_index: 16,
                            line_col: (1, 11),
                        },
                        children: vec![],
                        spread: None,
                        filters: vec![],
                        kind: ValueKind::Float,
                        start_index: 10,
                        end_index: 16,
                        line_col: (1, 11),
                    },
                    is_flag: false,
                    start_index: 10,
                    end_index: 16,
                    line_col: (1, 11),
                }],
                is_self_closing: false,
                syntax: TagSyntax::Django,
                start_index: 0,
                end_index: 19,
                line_col: (1, 4),
            }
        );
    }

    #[test]
    fn test_arg_single_number_scientific_3() {
        let input = "{% my_tag 20.e+02 %}";
        let result = TagParser::parse_tag(input, &HashSet::new()).unwrap();
        assert_eq!(
            result,
            Tag {
                name: TagToken {
                    token: "my_tag".to_string(),
                    start_index: 3,
                    end_index: 9,
                    line_col: (1, 4),
                },
                attrs: vec![TagAttr {
                    key: None,
                    value: TagValue {
                        token: TagToken {
                            token: "20.e+02".to_string(),
                            start_index: 10,
                            end_index: 17,
                            line_col: (1, 11),
                        },
                        children: vec![],
                        spread: None,
                        filters: vec![],
                        kind: ValueKind::Float,
                        start_index: 10,
                        end_index: 17,
                        line_col: (1, 11),
                    },
                    is_flag: false,
                    start_index: 10,
                    end_index: 17,
                    line_col: (1, 11),
                }],
                is_self_closing: false,
                syntax: TagSyntax::Django,
                start_index: 0,
                end_index: 20,
                line_col: (1, 4),
            }
        );
    }

    #[test]
    fn test_arg_single_quoted_string() {
        // Test single quoted string
        let input = r#"{% my_tag 'hello world' %}"#;
        let result = TagParser::parse_tag(input, &HashSet::new()).unwrap();
        assert_eq!(
            result,
            Tag {
                name: TagToken {
                    token: "my_tag".to_string(),
                    start_index: 3,
                    end_index: 9,
                    line_col: (1, 4),
                },
                attrs: vec![TagAttr {
                    key: None,
                    value: TagValue {
                        token: TagToken {
                            token: "'hello world'".to_string(),
                            start_index: 10,
                            end_index: 23,
                            line_col: (1, 11),
                        },
                        children: vec![],
                        spread: None,
                        filters: vec![],
                        kind: ValueKind::String,
                        start_index: 10,
                        end_index: 23,
                        line_col: (1, 11),
                    },
                    is_flag: false,
                    start_index: 10,
                    end_index: 23,
                    line_col: (1, 11),
                }],
                is_self_closing: false,
                syntax: TagSyntax::Django,
                start_index: 0,
                end_index: 26,
                line_col: (1, 4),
            }
        );
    }

    #[test]
    fn test_arg_single_double_quoted_string() {
        // Test double quoted string
        let input = r#"{% my_tag "hello world" %}"#;
        let result = TagParser::parse_tag(input, &HashSet::new()).unwrap();
        assert_eq!(
            result,
            Tag {
                name: TagToken {
                    token: "my_tag".to_string(),
                    start_index: 3,
                    end_index: 9,
                    line_col: (1, 4),
                },
                attrs: vec![TagAttr {
                    key: None,
                    value: TagValue {
                        token: TagToken {
                            token: r#""hello world""#.to_string(),
                            start_index: 10,
                            end_index: 23,
                            line_col: (1, 11),
                        },
                        children: vec![],
                        spread: None,
                        filters: vec![],
                        kind: ValueKind::String,
                        start_index: 10,
                        end_index: 23,
                        line_col: (1, 11),
                    },
                    is_flag: false,
                    start_index: 10,
                    end_index: 23,
                    line_col: (1, 11),
                }],
                is_self_closing: false,
                syntax: TagSyntax::Django,
                start_index: 0,
                end_index: 26,
                line_col: (1, 4),
            }
        );
    }

    #[test]
    fn test_arg_single_i18n_string() {
        let input = r#"{% my_tag _('hello world') %}"#;
        let result = TagParser::parse_tag(input, &HashSet::new()).unwrap();
        assert_eq!(
            result,
            Tag {
                name: TagToken {
                    token: "my_tag".to_string(),
                    start_index: 3,
                    end_index: 9,
                    line_col: (1, 4),
                },
                attrs: vec![TagAttr {
                    key: None,
                    value: TagValue {
                        token: TagToken {
                            token: "_('hello world')".to_string(),
                            start_index: 10,
                            end_index: 26,
                            line_col: (1, 11),
                        },
                        children: vec![],
                        spread: None,
                        filters: vec![],
                        kind: ValueKind::Translation,
                        start_index: 10,
                        end_index: 26,
                        line_col: (1, 11),
                    },
                    is_flag: false,
                    start_index: 10,
                    end_index: 26,
                    line_col: (1, 11),
                }],
                is_self_closing: false,
                syntax: TagSyntax::Django,
                start_index: 0,
                end_index: 29,
                line_col: (1, 4),
            }
        );
    }

    #[test]
    fn test_arg_single_i18n_string_with_double_quotes() {
        let input = r#"{% my_tag _("hello world") %}"#;
        let result = TagParser::parse_tag(input, &HashSet::new()).unwrap();
        assert_eq!(
            result,
            Tag {
                name: TagToken {
                    token: "my_tag".to_string(),
                    start_index: 3,
                    end_index: 9,
                    line_col: (1, 4),
                },
                attrs: vec![TagAttr {
                    key: None,
                    value: TagValue {
                        token: TagToken {
                            token: r#"_("hello world")"#.to_string(),
                            start_index: 10,
                            end_index: 26,
                            line_col: (1, 11),
                        },
                        children: vec![],
                        kind: ValueKind::Translation,
                        spread: None,
                        filters: vec![],
                        start_index: 10,
                        end_index: 26,
                        line_col: (1, 11),
                    },
                    is_flag: false,
                    start_index: 10,
                    end_index: 26,
                    line_col: (1, 11),
                }],
                is_self_closing: false,
                syntax: TagSyntax::Django,
                start_index: 0,
                end_index: 29,
                line_col: (1, 4),
            }
        );
    }

    #[test]
    fn test_arg_single_whitespace() {
        let input = "{% my_tag val %}";
        let result = TagParser::parse_tag(input, &HashSet::new()).unwrap();
        assert_eq!(
            result,
            Tag {
                name: TagToken {
                    token: "my_tag".to_string(),
                    start_index: 3,
                    end_index: 9,
                    line_col: (1, 4),
                },
                attrs: vec![TagAttr {
                    key: None,
                    value: TagValue {
                        token: TagToken {
                            token: "val".to_string(),
                            start_index: 10,
                            end_index: 13,
                            line_col: (1, 11),
                        },
                        children: vec![],
                        spread: None,
                        filters: vec![],
                        kind: ValueKind::Variable,
                        start_index: 10,
                        end_index: 13,
                        line_col: (1, 11),
                    },
                    is_flag: false,
                    start_index: 10,
                    end_index: 13,
                    line_col: (1, 11),
                }],
                is_self_closing: false,
                syntax: TagSyntax::Django,
                start_index: 0,
                end_index: 16,
                line_col: (1, 4),
            }
        );
    }
    #[test]
    fn test_arg_multiple() {
        let input = r#"{% my_tag component value1 value2 %}"#;
        let result = TagParser::parse_tag(input, &HashSet::new()).unwrap();
        assert_eq!(
            result,
            Tag {
                name: TagToken {
                    token: "my_tag".to_string(),
                    start_index: 3,
                    end_index: 9,
                    line_col: (1, 4),
                },
                attrs: vec![
                    TagAttr {
                        key: None,
                        value: TagValue {
                            token: TagToken {
                                token: "component".to_string(),
                                start_index: 10,
                                end_index: 19,
                                line_col: (1, 11),
                            },
                            children: vec![],
                            spread: None,
                            filters: vec![],
                            kind: ValueKind::Variable,
                            start_index: 10,
                            end_index: 19,
                            line_col: (1, 11),
                        },
                        is_flag: false,
                        start_index: 10,
                        end_index: 19,
                        line_col: (1, 11),
                    },
                    TagAttr {
                        key: None,
                        value: TagValue {
                            token: TagToken {
                                token: "value1".to_string(),
                                start_index: 20,
                                end_index: 26,
                                line_col: (1, 21),
                            },
                            children: vec![],
                            spread: None,
                            filters: vec![],
                            kind: ValueKind::Variable,
                            start_index: 20,
                            end_index: 26,
                            line_col: (1, 21),
                        },
                        is_flag: false,
                        start_index: 20,
                        end_index: 26,
                        line_col: (1, 21),
                    },
                    TagAttr {
                        key: None,
                        value: TagValue {
                            token: TagToken {
                                token: "value2".to_string(),
                                start_index: 27,
                                end_index: 33,
                                line_col: (1, 28),
                            },
                            children: vec![],
                            spread: None,
                            filters: vec![],
                            kind: ValueKind::Variable,
                            start_index: 27,
                            end_index: 33,
                            line_col: (1, 28),
                        },
                        is_flag: false,
                        start_index: 27,
                        end_index: 33,
                        line_col: (1, 28),
                    }
                ],
                is_self_closing: false,
                syntax: TagSyntax::Django,
                start_index: 0,
                end_index: 36,
                line_col: (1, 4),
            }
        );
    }

    #[test]
    fn test_kwarg_single() {
        let input = r#"{% my_tag key=val %}"#;
        let result = TagParser::parse_tag(input, &HashSet::new()).unwrap();
        assert_eq!(
            result,
            Tag {
                name: TagToken {
                    token: "my_tag".to_string(),
                    start_index: 3,
                    end_index: 9,
                    line_col: (1, 4),
                },
                attrs: vec![TagAttr {
                    key: Some(TagToken {
                        token: "key".to_string(),
                        start_index: 10,
                        end_index: 13,
                        line_col: (1, 11),
                    }),
                    value: TagValue {
                        token: TagToken {
                            token: "val".to_string(),
                            start_index: 14,
                            end_index: 17,
                            line_col: (1, 15),
                        },
                        children: vec![],
                        spread: None,
                        filters: vec![],
                        kind: ValueKind::Variable,
                        start_index: 14,
                        end_index: 17,
                        line_col: (1, 15),
                    },
                    is_flag: false,
                    start_index: 10,
                    end_index: 17,
                    line_col: (1, 11),
                },],
                is_self_closing: false,
                syntax: TagSyntax::Django,
                start_index: 0,
                end_index: 20,
                line_col: (1, 4),
            }
        );
    }

    #[test]
    fn test_kwarg_single_whitespace() {
        let input = r#"{% my_tag key=val %}"#;
        let result = TagParser::parse_tag(input, &HashSet::new()).unwrap();
        assert_eq!(
            result,
            Tag {
                name: TagToken {
                    token: "my_tag".to_string(),
                    start_index: 3,
                    end_index: 9,
                    line_col: (1, 4),
                },
                attrs: vec![TagAttr {
                    key: Some(TagToken {
                        token: "key".to_string(),
                        start_index: 10,
                        end_index: 13,
                        line_col: (1, 11),
                    }),
                    value: TagValue {
                        token: TagToken {
                            token: "val".to_string(),
                            start_index: 14,
                            end_index: 17,
                            line_col: (1, 15),
                        },
                        children: vec![],
                        spread: None,
                        filters: vec![],
                        kind: ValueKind::Variable,
                        start_index: 14,
                        end_index: 17,
                        line_col: (1, 15),
                    },
                    is_flag: false,
                    start_index: 10,
                    end_index: 17,
                    line_col: (1, 11),
                },],
                is_self_closing: false,
                syntax: TagSyntax::Django,
                start_index: 0,
                end_index: 20,
                line_col: (1, 4),
            }
        );
    }

    #[test]
    fn test_kwarg_multiple() {
        let input = r#"{% my_tag key=val key2=val2 %}"#;
        let result = TagParser::parse_tag(input, &HashSet::new()).unwrap();
        assert_eq!(
            result,
            Tag {
                name: TagToken {
                    token: "my_tag".to_string(),
                    start_index: 3,
                    end_index: 9,
                    line_col: (1, 4),
                },
                attrs: vec![
                    TagAttr {
                        key: Some(TagToken {
                            token: "key".to_string(),
                            start_index: 10,
                            end_index: 13,
                            line_col: (1, 11),
                        }),
                        value: TagValue {
                            token: TagToken {
                                token: "val".to_string(),
                                start_index: 14,
                                end_index: 17,
                                line_col: (1, 15),
                            },
                            children: vec![],
                            spread: None,
                            filters: vec![],
                            kind: ValueKind::Variable,
                            start_index: 14,
                            end_index: 17,
                            line_col: (1, 15),
                        },
                        is_flag: false,
                        start_index: 10,
                        end_index: 17,
                        line_col: (1, 11),
                    },
                    TagAttr {
                        key: Some(TagToken {
                            token: "key2".to_string(),
                            start_index: 18,
                            end_index: 22,
                            line_col: (1, 19),
                        }),
                        value: TagValue {
                            token: TagToken {
                                token: "val2".to_string(),
                                start_index: 23,
                                end_index: 27,
                                line_col: (1, 24),
                            },
                            children: vec![],
                            spread: None,
                            filters: vec![],
                            kind: ValueKind::Variable,
                            start_index: 23,
                            end_index: 27,
                            line_col: (1, 24),
                        },
                        is_flag: false,
                        start_index: 18,
                        end_index: 27,
                        line_col: (1, 19),
                    }
                ],
                is_self_closing: false,
                syntax: TagSyntax::Django,
                start_index: 0,
                end_index: 30,
                line_col: (1, 4),
            }
        );
    }

    // Test that we do NOT allow whitespace around the `=`, e.g. `key= val`, `key =val`, `key = val`
    #[test]
    fn test_kwarg_whitespace_around_equals_1() {
        // Test whitespace after key
        let input = "{% my_tag key= val %}";
        assert!(
            TagParser::parse_tag(input, &HashSet::new()).is_err(),
            "Should not allow whitespace after key before equals"
        );
    }
    #[test]
    fn test_kwarg_whitespace_around_equals_2() {
        // Test whitespace before value
        let input = "{% my_tag key =val %}";
        assert!(
            TagParser::parse_tag(input, &HashSet::new()).is_err(),
            "Should not allow whitespace before value after equals"
        );
    }
    #[test]
    fn test_kwarg_whitespace_around_equals_3() {
        // Test whitespace on both sides
        let input = "{% my_tag key = val %}";
        assert!(
            TagParser::parse_tag(input, &HashSet::new()).is_err(),
            "Should not allow whitespace around equals"
        );
    }
    #[test]
    fn test_kwarg_whitespace_around_equals_4() {
        // Test multiple attributes with mixed whitespace
        let input = "{% my_tag key1= val1 key2 =val2 key3 = val3 %}";
        assert!(
            TagParser::parse_tag(input, &HashSet::new()).is_err(),
            "Should not allow whitespace around equals in any attribute"
        );
    }

    #[test]
    fn test_kwarg_special_chars() {
        let input = r#"{% my_tag @click.stop=handler attr:key=val %}"#;
        let result = TagParser::parse_tag(input, &HashSet::new()).unwrap();
        assert_eq!(
            result,
            Tag {
                name: TagToken {
                    token: "my_tag".to_string(),
                    start_index: 3,
                    end_index: 9,
                    line_col: (1, 4),
                },
                attrs: vec![
                    TagAttr {
                        key: Some(TagToken {
                            token: "@click.stop".to_string(),
                            start_index: 10,
                            end_index: 21,
                            line_col: (1, 11),
                        }),
                        value: TagValue {
                            token: TagToken {
                                token: "handler".to_string(),
                                start_index: 22,
                                end_index: 29,
                                line_col: (1, 23),
                            },
                            children: vec![],
                            spread: None,
                            filters: vec![],
                            kind: ValueKind::Variable,
                            start_index: 22,
                            end_index: 29,
                            line_col: (1, 23),
                        },
                        is_flag: false,
                        start_index: 10,
                        end_index: 29,
                        line_col: (1, 11),
                    },
                    TagAttr {
                        key: Some(TagToken {
                            token: "attr:key".to_string(),
                            start_index: 30,
                            end_index: 38,
                            line_col: (1, 31),
                        }),
                        value: TagValue {
                            token: TagToken {
                                token: "val".to_string(),
                                start_index: 39,
                                end_index: 42,
                                line_col: (1, 40),
                            },
                            children: vec![],
                            spread: None,
                            filters: vec![],
                            kind: ValueKind::Variable,
                            start_index: 39,
                            end_index: 42,
                            line_col: (1, 40),
                        },
                        is_flag: false,
                        start_index: 30,
                        end_index: 42,
                        line_col: (1, 31),
                    }
                ],
                is_self_closing: false,
                syntax: TagSyntax::Django,
                start_index: 0,
                end_index: 45,
                line_col: (1, 4),
            }
        );
    }

    #[test]
    fn test_kwarg_invalid() {
        let inputs = vec![
            "{% my_tag :key=val %}",
            "{% my_tag ...key=val %}",
            "{% my_tag _('hello')=val %}",
            r#"{% my_tag "key"=val %}"#,
            "{% my_tag key[0]=val %}",
        ];

        for input in inputs {
            assert!(
                TagParser::parse_tag(input, &HashSet::new()).is_err(),
                "Input should fail: {}",
                input
            );
        }
    }

    #[test]
    fn test_comment_before() {
        let input = r#"{% my_tag {# comment #} value %}"#;
        let result = TagParser::parse_tag(input, &HashSet::new()).unwrap();
        assert_eq!(
            result,
            Tag {
                name: TagToken {
                    token: "my_tag".to_string(),
                    start_index: 3,
                    end_index: 9,
                    line_col: (1, 4),
                },
                attrs: vec![TagAttr {
                    key: None,
                    value: TagValue {
                        token: TagToken {
                            token: "value".to_string(),
                            start_index: 24,
                            end_index: 29,
                            line_col: (1, 25),
                        },
                        children: vec![],
                        spread: None,
                        filters: vec![],
                        kind: ValueKind::Variable,
                        start_index: 24,
                        end_index: 29,
                        line_col: (1, 25),
                    },
                    is_flag: false,
                    start_index: 24,
                    end_index: 29,
                    line_col: (1, 25),
                },],
                is_self_closing: false,
                syntax: TagSyntax::Django,
                start_index: 0,
                end_index: 32,
                line_col: (1, 4),
            }
        );
    }

    #[test]
    fn test_comment_after() {
        // Test comment after attribute
        let input = "{% my_tag key=val{# comment #} %}";
        let result = TagParser::parse_tag(input, &HashSet::new()).unwrap();
        assert_eq!(
            result,
            Tag {
                name: TagToken {
                    token: "my_tag".to_string(),
                    start_index: 3,
                    end_index: 9,
                    line_col: (1, 4),
                },
                attrs: vec![TagAttr {
                    key: Some(TagToken {
                        token: "key".to_string(),
                        start_index: 10,
                        end_index: 13,
                        line_col: (1, 11),
                    }),
                    value: TagValue {
                        token: TagToken {
                            token: "val".to_string(),
                            start_index: 14,
                            end_index: 17,
                            line_col: (1, 15),
                        },
                        children: vec![],
                        spread: None,
                        filters: vec![],
                        kind: ValueKind::Variable,
                        start_index: 14,
                        end_index: 17,
                        line_col: (1, 15),
                    },
                    is_flag: false,
                    start_index: 10,
                    end_index: 17,
                    line_col: (1, 11),
                },],
                is_self_closing: false,
                syntax: TagSyntax::Django,
                start_index: 0,
                end_index: 33,
                line_col: (1, 4),
            }
        );
    }

    #[test]
    fn test_comment_between() {
        let input = "{% my_tag key1=val1 {# comment #} key2=val2 %}";
        let result = TagParser::parse_tag(input, &HashSet::new()).unwrap();
        assert_eq!(
            result,
            Tag {
                name: TagToken {
                    token: "my_tag".to_string(),
                    start_index: 3,
                    end_index: 9,
                    line_col: (1, 4),
                },
                attrs: vec![
                    TagAttr {
                        key: Some(TagToken {
                            token: "key1".to_string(),
                            start_index: 10,
                            end_index: 14,
                            line_col: (1, 11),
                        }),
                        value: TagValue {
                            token: TagToken {
                                token: "val1".to_string(),
                                start_index: 15,
                                end_index: 19,
                                line_col: (1, 16),
                            },
                            children: vec![],
                            spread: None,
                            filters: vec![],
                            kind: ValueKind::Variable,
                            start_index: 15,
                            end_index: 19,
                            line_col: (1, 16),
                        },
                        is_flag: false,
                        start_index: 10,
                        end_index: 19,
                        line_col: (1, 11),
                    },
                    TagAttr {
                        key: Some(TagToken {
                            token: "key2".to_string(),
                            start_index: 34,
                            end_index: 38,
                            line_col: (1, 35),
                        }),
                        value: TagValue {
                            token: TagToken {
                                token: "val2".to_string(),
                                start_index: 39,
                                end_index: 43,
                                line_col: (1, 40),
                            },
                            children: vec![],
                            spread: None,
                            filters: vec![],
                            kind: ValueKind::Variable,
                            start_index: 39,
                            end_index: 43,
                            line_col: (1, 40),
                        },
                        is_flag: false,
                        start_index: 34,
                        end_index: 43,
                        line_col: (1, 35),
                    }
                ],
                is_self_closing: false,
                syntax: TagSyntax::Django,
                start_index: 0,
                end_index: 46,
                line_col: (1, 4),
            }
        );
    }

    #[test]
    fn test_comment_multiple() {
        // Test multiple comments
        // {% my_tag {# c1 #}key1=val1{# c2 #} {# c3 #}key2=val2{# c4 #} %}
        // Position breakdown:
        // 0-2: {%
        // 3-9: my_tag
        // 10-18: {# c1 #}
        // 18-22: key1
        // 23-27: val1
        // 27-36: {# c2 #}
        // 37-46: {# c3 #}
        // 46-50: key2  (but actual test shows it's at 44-48)
        let input = "{% my_tag {# c1 #}key1=val1{# c2 #} {# c3 #}key2=val2{# c4 #} %}";
        let result = TagParser::parse_tag(input, &HashSet::new()).unwrap();
        assert_eq!(
            result,
            Tag {
                name: TagToken {
                    token: "my_tag".to_string(),
                    start_index: 3,
                    end_index: 9,
                    line_col: (1, 4),
                },
                attrs: vec![
                    TagAttr {
                        key: Some(TagToken {
                            token: "key1".to_string(),
                            start_index: 18,
                            end_index: 22,
                            line_col: (1, 19),
                        }),
                        value: TagValue {
                            token: TagToken {
                                token: "val1".to_string(),
                                start_index: 23,
                                end_index: 27,
                                line_col: (1, 24),
                            },
                            children: vec![],
                            spread: None,
                            filters: vec![],
                            kind: ValueKind::Variable,
                            start_index: 23,
                            end_index: 27,
                            line_col: (1, 24),
                        },
                        is_flag: false,
                        start_index: 18,
                        end_index: 27,
                        line_col: (1, 19),
                    },
                    TagAttr {
                        key: Some(TagToken {
                            token: "key2".to_string(),
                            start_index: 44,
                            end_index: 48,
                            line_col: (1, 45),
                        }),
                        value: TagValue {
                            token: TagToken {
                                token: "val2".to_string(),
                                start_index: 49,
                                end_index: 53,
                                line_col: (1, 50),
                            },
                            children: vec![],
                            spread: None,
                            filters: vec![],
                            kind: ValueKind::Variable,
                            start_index: 49,
                            end_index: 53,
                            line_col: (1, 50),
                        },
                        is_flag: false,
                        start_index: 44,
                        end_index: 53,
                        line_col: (1, 45),
                    }
                ],
                is_self_closing: false,
                syntax: TagSyntax::Django,
                start_index: 0,
                end_index: 64,
                line_col: (1, 4),
            }
        );
    }

    #[test]
    fn test_comment_no_whitespace() {
        // Test that comments without whitespace between tag_name and attributes should fail
        // because we require at least one WHITESPACE (not just comments)
        let input = "{% my_tag {# c1 #}key1=val1{# c2 #}key2=val2{# c3 #} %}";
        let result = TagParser::parse_tag(input, &HashSet::new());
        assert!(result.is_err(), "Should error when there's no whitespace between tag_name and attributes (only comments)");
    }

    #[test]
    fn test_comment_with_newlines() {
        // Test comment with newlines
        let input = "{% my_tag key1=val1 {# multi\nline\ncomment #} key2=val2 %}";
        let result = TagParser::parse_tag(input, &HashSet::new()).unwrap();
        assert_eq!(
            result,
            Tag {
                name: TagToken {
                    token: "my_tag".to_string(),
                    start_index: 3,
                    end_index: 9,
                    line_col: (1, 4),
                },
                attrs: vec![
                    TagAttr {
                        key: Some(TagToken {
                            token: "key1".to_string(),
                            start_index: 10,
                            end_index: 14,
                            line_col: (1, 11),
                        }),
                        value: TagValue {
                            token: TagToken {
                                token: "val1".to_string(),
                                start_index: 15,
                                end_index: 19,
                                line_col: (1, 16),
                            },
                            children: vec![],
                            spread: None,
                            filters: vec![],
                            kind: ValueKind::Variable,
                            start_index: 15,
                            end_index: 19,
                            line_col: (1, 16),
                        },
                        is_flag: false,
                        start_index: 10,
                        end_index: 19,
                        line_col: (1, 11),
                    },
                    TagAttr {
                        key: Some(TagToken {
                            token: "key2".to_string(),
                            start_index: 45,
                            end_index: 49,
                            line_col: (3, 12),
                        }),
                        value: TagValue {
                            token: TagToken {
                                token: "val2".to_string(),
                                start_index: 50,
                                end_index: 54,
                                line_col: (3, 17),
                            },
                            children: vec![],
                            spread: None,
                            filters: vec![],
                            kind: ValueKind::Variable,
                            start_index: 50,
                            end_index: 54,
                            line_col: (3, 17),
                        },
                        is_flag: false,
                        start_index: 45,
                        end_index: 54,
                        line_col: (3, 12),
                    }
                ],
                is_self_closing: false,
                syntax: TagSyntax::Django,
                start_index: 0,
                end_index: 57,
                line_col: (1, 4),
            }
        );
    }

    #[test]
    fn test_comment_not_allowed_between_key_and_value() {
        // Test comment between key and equals
        let input = "{% my_tag key{# comment #}=val %}";
        assert!(
            TagParser::parse_tag(input, &HashSet::new()).is_err(),
            "Should not allow comment between key and equals"
        );

        // Test comment between equals and value
        let input = "{% my_tag key={# comment #}val %}";
        assert!(
            TagParser::parse_tag(input, &HashSet::new()).is_err(),
            "Should not allow comment between equals and value"
        );
    }

    #[test]
    fn test_spread_basic() {
        let input = "{% my_tag ...myvalue %}";
        let result = TagParser::parse_tag(input, &HashSet::new()).unwrap();
        assert_eq!(
            result,
            Tag {
                name: TagToken {
                    token: "my_tag".to_string(),
                    start_index: 3,
                    end_index: 9,
                    line_col: (1, 4),
                },
                attrs: vec![TagAttr {
                    key: None,
                    value: TagValue {
                        token: TagToken {
                            token: "myvalue".to_string(),
                            start_index: 13,
                            end_index: 20,
                            line_col: (1, 14),
                        },
                        children: vec![],
                        spread: Some("...".to_string()),
                        filters: vec![],
                        kind: ValueKind::Variable,
                        start_index: 10,
                        end_index: 20,
                        line_col: (1, 11),
                    },
                    is_flag: false,
                    start_index: 10,
                    end_index: 20,
                    line_col: (1, 11),
                },],
                is_self_closing: false,
                syntax: TagSyntax::Django,
                start_index: 0,
                end_index: 23,
                line_col: (1, 4),
            }
        );
    }
    #[test]
    fn test_spread_between() {
        // Test spread with other attributes
        let input = "{% my_tag key1=val1 ...myvalue key2=val2 %}";
        let result = TagParser::parse_tag(input, &HashSet::new()).unwrap();
        assert_eq!(
            result,
            Tag {
                name: TagToken {
                    token: "my_tag".to_string(),
                    start_index: 3,
                    end_index: 9,
                    line_col: (1, 4),
                },
                attrs: vec![
                    TagAttr {
                        key: Some(TagToken {
                            token: "key1".to_string(),
                            start_index: 10,
                            end_index: 14,
                            line_col: (1, 11),
                        }),
                        value: TagValue {
                            token: TagToken {
                                token: "val1".to_string(),
                                start_index: 15,
                                end_index: 19,
                                line_col: (1, 16),
                            },
                            children: vec![],
                            spread: None,
                            filters: vec![],
                            kind: ValueKind::Variable,
                            start_index: 15,
                            end_index: 19,
                            line_col: (1, 16),
                        },
                        is_flag: false,
                        start_index: 10,
                        end_index: 19,
                        line_col: (1, 11),
                    },
                    TagAttr {
                        key: None,
                        value: TagValue {
                            token: TagToken {
                                token: "myvalue".to_string(),
                                start_index: 23,
                                end_index: 30,
                                line_col: (1, 24),
                            },
                            children: vec![],
                            spread: Some("...".to_string()),
                            filters: vec![],
                            kind: ValueKind::Variable,
                            start_index: 20,
                            end_index: 30,
                            line_col: (1, 21),
                        },
                        is_flag: false,
                        start_index: 20,
                        end_index: 30,
                        line_col: (1, 21),
                    },
                    TagAttr {
                        key: Some(TagToken {
                            token: "key2".to_string(),
                            start_index: 31,
                            end_index: 35,
                            line_col: (1, 32),
                        }),
                        value: TagValue {
                            token: TagToken {
                                token: "val2".to_string(),
                                start_index: 36,
                                end_index: 40,
                                line_col: (1, 37),
                            },
                            children: vec![],
                            spread: None,
                            filters: vec![],
                            kind: ValueKind::Variable,
                            start_index: 36,
                            end_index: 40,
                            line_col: (1, 37),
                        },
                        is_flag: false,
                        start_index: 31,
                        end_index: 40,
                        line_col: (1, 32),
                    }
                ],
                is_self_closing: false,
                syntax: TagSyntax::Django,
                start_index: 0,
                end_index: 43,
                line_col: (1, 4),
            }
        );
    }

    #[test]
    fn test_spread_multiple() {
        let input = "{% my_tag ...dict1 key=val ...dict2 %}";
        let result = TagParser::parse_tag(input, &HashSet::new()).unwrap();
        assert_eq!(
            result,
            Tag {
                name: TagToken {
                    token: "my_tag".to_string(),
                    start_index: 3,
                    end_index: 9,
                    line_col: (1, 4),
                },
                attrs: vec![
                    TagAttr {
                        key: None,
                        value: TagValue {
                            token: TagToken {
                                token: "dict1".to_string(),
                                start_index: 13,
                                end_index: 18,
                                line_col: (1, 14),
                            },
                            children: vec![],
                            spread: Some("...".to_string()),
                            filters: vec![],
                            kind: ValueKind::Variable,
                            start_index: 10,
                            end_index: 18,
                            line_col: (1, 11),
                        },
                        is_flag: false,
                        start_index: 10,
                        end_index: 18,
                        line_col: (1, 11),
                    },
                    TagAttr {
                        key: Some(TagToken {
                            token: "key".to_string(),
                            start_index: 19,
                            end_index: 22,
                            line_col: (1, 20),
                        }),
                        value: TagValue {
                            token: TagToken {
                                token: "val".to_string(),
                                start_index: 23,
                                end_index: 26,
                                line_col: (1, 24),
                            },
                            children: vec![],
                            spread: None,
                            filters: vec![],
                            kind: ValueKind::Variable,
                            start_index: 23,
                            end_index: 26,
                            line_col: (1, 24),
                        },
                        is_flag: false,
                        start_index: 19,
                        end_index: 26,
                        line_col: (1, 20),
                    },
                    TagAttr {
                        key: None,
                        value: TagValue {
                            token: TagToken {
                                token: "dict2".to_string(),
                                start_index: 30,
                                end_index: 35,
                                line_col: (1, 31),
                            },
                            children: vec![],
                            spread: Some("...".to_string()),
                            filters: vec![],
                            kind: ValueKind::Variable,
                            start_index: 27,
                            end_index: 35,
                            line_col: (1, 28),
                        },
                        is_flag: false,
                        start_index: 27,
                        end_index: 35,
                        line_col: (1, 28),
                    }
                ],
                is_self_closing: false,
                syntax: TagSyntax::Django,
                start_index: 0,
                end_index: 38,
                line_col: (1, 4),
            }
        );
    }

    #[test]
    fn test_spread_dict() {
        // Test spread with dictionary
        let input = r#"{% my_tag ...{"key": "value"} %}"#;
        let result = TagParser::parse_tag(input, &HashSet::new()).unwrap();
        assert_eq!(
            result,
            Tag {
                name: TagToken {
                    token: "my_tag".to_string(),
                    start_index: 3,
                    end_index: 9,
                    line_col: (1, 4),
                },
                attrs: vec![TagAttr {
                    key: None,
                    value: TagValue {
                        token: TagToken {
                            token: "{\"key\": \"value\"}".to_string(),
                            start_index: 13,
                            end_index: 29,
                            line_col: (1, 14),
                        },
                        children: vec![
                            TagValue {
                                token: TagToken {
                                    token: "\"key\"".to_string(),
                                    start_index: 14,
                                    end_index: 19,
                                    line_col: (1, 15),
                                },
                                children: vec![],
                                spread: None,
                                filters: vec![],
                                kind: ValueKind::String,
                                start_index: 14,
                                end_index: 19,
                                line_col: (1, 15),
                            },
                            TagValue {
                                token: TagToken {
                                    token: "\"value\"".to_string(),
                                    start_index: 21,
                                    end_index: 28,
                                    line_col: (1, 22),
                                },
                                children: vec![],
                                spread: None,
                                filters: vec![],
                                kind: ValueKind::String,
                                start_index: 21,
                                end_index: 28,
                                line_col: (1, 22),
                            },
                        ],
                        spread: Some("...".to_string()),
                        filters: vec![],
                        kind: ValueKind::Dict,
                        start_index: 10,
                        end_index: 29,
                        line_col: (1, 11),
                    },
                    is_flag: false,
                    start_index: 10,
                    end_index: 29,
                    line_col: (1, 11),
                }],
                is_self_closing: false,
                syntax: TagSyntax::Django,
                start_index: 0,
                end_index: 32,
                line_col: (1, 4),
            }
        );
    }

    #[test]
    fn test_spread_list() {
        let input = "{% my_tag ...[1, 2, 3] %}";
        let result = TagParser::parse_tag(input, &HashSet::new()).unwrap();
        assert_eq!(
            result,
            Tag {
                name: TagToken {
                    token: "my_tag".to_string(),
                    start_index: 3,
                    end_index: 9,
                    line_col: (1, 4),
                },
                attrs: vec![TagAttr {
                    key: None,
                    value: TagValue {
                        token: TagToken {
                            token: "[1, 2, 3]".to_string(),
                            start_index: 13,
                            end_index: 22,
                            line_col: (1, 14),
                        },
                        children: vec![
                            TagValue {
                                token: TagToken {
                                    token: "1".to_string(),
                                    start_index: 14,
                                    end_index: 15,
                                    line_col: (1, 15),
                                },
                                children: vec![],
                                spread: None,
                                filters: vec![],
                                kind: ValueKind::Int,
                                start_index: 14,
                                end_index: 15,
                                line_col: (1, 15),
                            },
                            TagValue {
                                token: TagToken {
                                    token: "2".to_string(),
                                    start_index: 17,
                                    end_index: 18,
                                    line_col: (1, 18),
                                },
                                children: vec![],
                                spread: None,
                                filters: vec![],
                                kind: ValueKind::Int,
                                start_index: 17,
                                end_index: 18,
                                line_col: (1, 18),
                            },
                            TagValue {
                                token: TagToken {
                                    token: "3".to_string(),
                                    start_index: 20,
                                    end_index: 21,
                                    line_col: (1, 21),
                                },
                                children: vec![],
                                spread: None,
                                filters: vec![],
                                kind: ValueKind::Int,
                                start_index: 20,
                                end_index: 21,
                                line_col: (1, 21),
                            }
                        ],
                        spread: Some("...".to_string()),
                        filters: vec![],
                        kind: ValueKind::List,
                        start_index: 10,
                        end_index: 22,
                        line_col: (1, 11),
                    },
                    is_flag: false,
                    start_index: 10,
                    end_index: 22,
                    line_col: (1, 11),
                }],
                is_self_closing: false,
                syntax: TagSyntax::Django,
                start_index: 0,
                end_index: 25,
                line_col: (1, 4),
            }
        );
    }

    #[test]
    fn test_spread_i18n() {
        // Test spread with i18n string
        let input = "{% my_tag ..._('hello') %}";
        let result = TagParser::parse_tag(input, &HashSet::new()).unwrap();
        assert_eq!(
            result,
            Tag {
                name: TagToken {
                    token: "my_tag".to_string(),
                    start_index: 3,
                    end_index: 9,
                    line_col: (1, 4),
                },
                attrs: vec![TagAttr {
                    key: None,
                    value: TagValue {
                        token: TagToken {
                            token: "_('hello')".to_string(),
                            start_index: 13,
                            end_index: 23,
                            line_col: (1, 14),
                        },
                        children: vec![],
                        spread: Some("...".to_string()),
                        filters: vec![],
                        kind: ValueKind::Translation,
                        start_index: 10,
                        end_index: 23,
                        line_col: (1, 11),
                    },
                    is_flag: false,
                    start_index: 10,
                    end_index: 23,
                    line_col: (1, 11),
                },],
                is_self_closing: false,
                syntax: TagSyntax::Django,
                start_index: 0,
                end_index: 26,
                line_col: (1, 4),
            }
        );
    }

    #[test]
    fn test_spread_variable() {
        // Test spread with variable
        let input = "{% my_tag ...my_var %}";
        let result = TagParser::parse_tag(input, &HashSet::new()).unwrap();
        assert_eq!(
            result,
            Tag {
                name: TagToken {
                    token: "my_tag".to_string(),
                    start_index: 3,
                    end_index: 9,
                    line_col: (1, 4),
                },
                attrs: vec![TagAttr {
                    key: None,
                    value: TagValue {
                        token: TagToken {
                            token: "my_var".to_string(),
                            start_index: 13,
                            end_index: 19,
                            line_col: (1, 14),
                        },
                        children: vec![],
                        spread: Some("...".to_string()),
                        filters: vec![],
                        kind: ValueKind::Variable,
                        start_index: 10,
                        end_index: 19,
                        line_col: (1, 11),
                    },
                    is_flag: false,
                    start_index: 10,
                    end_index: 19,
                    line_col: (1, 11),
                },],
                is_self_closing: false,
                syntax: TagSyntax::Django,
                start_index: 0,
                end_index: 22,
                line_col: (1, 4),
            }
        );
    }

    #[test]
    fn test_spread_number() {
        // Test spread with number
        let input = "{% my_tag ...42 %}";
        let result = TagParser::parse_tag(input, &HashSet::new()).unwrap();
        assert_eq!(
            result,
            Tag {
                name: TagToken {
                    token: "my_tag".to_string(),
                    start_index: 3,
                    end_index: 9,
                    line_col: (1, 4),
                },
                attrs: vec![TagAttr {
                    key: None,
                    value: TagValue {
                        token: TagToken {
                            token: "42".to_string(),
                            start_index: 13,
                            end_index: 15,
                            line_col: (1, 14),
                        },
                        children: vec![],
                        spread: Some("...".to_string()),
                        filters: vec![],
                        kind: ValueKind::Int,
                        start_index: 10,
                        end_index: 15,
                        line_col: (1, 11),
                    },
                    is_flag: false,
                    start_index: 10,
                    end_index: 15,
                    line_col: (1, 11),
                }],
                is_self_closing: false,
                syntax: TagSyntax::Django,
                start_index: 0,
                end_index: 18,
                line_col: (1, 4),
            }
        );
    }

    #[test]
    fn test_spread_string() {
        // Test spread with string literal
        let input = r#"{% my_tag ..."hello" %}"#;
        let result = TagParser::parse_tag(input, &HashSet::new()).unwrap();
        assert_eq!(
            result,
            Tag {
                name: TagToken {
                    token: "my_tag".to_string(),
                    start_index: 3,
                    end_index: 9,
                    line_col: (1, 4),
                },
                attrs: vec![TagAttr {
                    key: None,
                    value: TagValue {
                        token: TagToken {
                            token: "\"hello\"".to_string(),
                            start_index: 13,
                            end_index: 20,
                            line_col: (1, 14),
                        },
                        children: vec![],
                        spread: Some("...".to_string()),
                        filters: vec![],
                        kind: ValueKind::String,
                        start_index: 10,
                        end_index: 20,
                        line_col: (1, 11),
                    },
                    is_flag: false,
                    start_index: 10,
                    end_index: 20,
                    line_col: (1, 11),
                }],
                is_self_closing: false,
                syntax: TagSyntax::Django,
                start_index: 0,
                end_index: 23,
                line_col: (1, 4),
            }
        );
    }

    #[test]
    fn test_spread_invalid() {
        // Test spread missing value
        let input = "{% my_tag ... %}";
        assert!(
            TagParser::parse_tag(input, &HashSet::new()).is_err(),
            "Should not allow spread operator without a value"
        );
        // Test spread whitespace between operator and value
        let input = "{% my_tag ...  myvalue %}";
        assert!(
            TagParser::parse_tag(input, &HashSet::new()).is_err(),
            "Should not allow spread operator with whitespace between operator and value"
        );

        // Test spread in key position
        let input = "{% my_tag ...key=val %}";
        assert!(
            TagParser::parse_tag(input, &HashSet::new()).is_err(),
            "Should not allow spread operator in key position"
        );

        // Test spread in value position of key-value pair
        let input = "{% my_tag key=...val %}";
        assert!(
            TagParser::parse_tag(input, &HashSet::new()).is_err(),
            "Should not allow spread operator in value position of key-value pair"
        );

        // Test spread operator inside list
        let input = "{% my_tag [1, ...my_list, 2] %}";
        assert!(
            TagParser::parse_tag(input, &HashSet::new()).is_err(),
            "Should not allow ... spread operator inside list"
        );

        // Test spread operator inside list with filters
        let input = "{% my_tag [1, ...my_list|filter, 2] %}";
        assert!(
            TagParser::parse_tag(input, &HashSet::new()).is_err(),
            "Should not allow ... spread operator inside list with filters"
        );

        // Test spread operator inside nested list
        let input = "{% my_tag [1, [...my_list], 2] %}";
        assert!(
            TagParser::parse_tag(input, &HashSet::new()).is_err(),
            "Should not allow ... spread operator inside nested list"
        );
    }

    #[test]
    fn test_filter_basic() {
        let input = "{% my_tag value|lower %}";
        let result = TagParser::parse_tag(input, &HashSet::new()).unwrap();
        assert_eq!(
            result,
            Tag {
                name: TagToken {
                    token: "my_tag".to_string(),
                    start_index: 3,
                    end_index: 9,
                    line_col: (1, 4),
                },
                attrs: vec![TagAttr {
                    key: None,
                    value: TagValue {
                        token: TagToken {
                            token: "value".to_string(),
                            start_index: 10,
                            end_index: 15,
                            line_col: (1, 11),
                        },
                        children: vec![],
                        spread: None,
                        filters: vec![TagValueFilter {
                            arg: None,
                            token: TagToken {
                                token: "lower".to_string(),
                                start_index: 16,
                                end_index: 21,
                                line_col: (1, 17),
                            },
                            start_index: 15,
                            end_index: 21,
                            line_col: (1, 16),
                        }],
                        kind: ValueKind::Variable,
                        start_index: 10,
                        end_index: 21,
                        line_col: (1, 11),
                    },
                    is_flag: false,
                    start_index: 10,
                    end_index: 21,
                    line_col: (1, 11),
                }],
                is_self_closing: false,
                syntax: TagSyntax::Django,
                start_index: 0,
                end_index: 24,
                line_col: (1, 4),
            }
        );
    }

    #[test]
    fn test_filter_multiple() {
        let input = "{% my_tag value|lower|title|default:'hello' %}";
        let result = TagParser::parse_tag(input, &HashSet::new()).unwrap();
        assert_eq!(
            result,
            Tag {
                name: TagToken {
                    token: "my_tag".to_string(),
                    start_index: 3,
                    end_index: 9,
                    line_col: (1, 4),
                },
                attrs: vec![TagAttr {
                    key: None,
                    value: TagValue {
                        token: TagToken {
                            token: "value".to_string(),
                            start_index: 10,
                            end_index: 15,
                            line_col: (1, 11),
                        },
                        children: vec![],
                        kind: ValueKind::Variable,
                        spread: None,
                        filters: vec![
                            TagValueFilter {
                                token: TagToken {
                                    token: "lower".to_string(),
                                    start_index: 16,
                                    end_index: 21,
                                    line_col: (1, 17),
                                },
                                arg: None,
                                start_index: 15,
                                end_index: 21,
                                line_col: (1, 16),
                            },
                            TagValueFilter {
                                token: TagToken {
                                    token: "title".to_string(),
                                    start_index: 22,
                                    end_index: 27,
                                    line_col: (1, 23),
                                },
                                arg: None,
                                start_index: 21,
                                end_index: 27,
                                line_col: (1, 22),
                            },
                            TagValueFilter {
                                token: TagToken {
                                    token: "default".to_string(),
                                    start_index: 28,
                                    end_index: 35,
                                    line_col: (1, 29),
                                },
                                arg: Some(TagValue {
                                    token: TagToken {
                                        token: "'hello'".to_string(),
                                        start_index: 36,
                                        end_index: 43,
                                        line_col: (1, 37),
                                    },
                                    children: vec![],
                                    kind: ValueKind::String,
                                    spread: None,
                                    filters: vec![],
                                    start_index: 35,
                                    end_index: 43,
                                    line_col: (1, 36),
                                }),
                                start_index: 27,
                                end_index: 43,
                                line_col: (1, 28),
                            }
                        ],
                        start_index: 10,
                        end_index: 43,
                        line_col: (1, 11),
                    },
                    is_flag: false,
                    start_index: 10,
                    end_index: 43,
                    line_col: (1, 11),
                }],
                is_self_closing: false,
                syntax: TagSyntax::Django,
                start_index: 0,
                end_index: 46,
                line_col: (1, 4),
            }
        );
    }

    #[test]
    fn test_filter_arg_string() {
        let input = "{% my_tag value|default:'hello' %}";
        let result = TagParser::parse_tag(input, &HashSet::new()).unwrap();
        assert_eq!(
            result,
            Tag {
                name: TagToken {
                    token: "my_tag".to_string(),
                    start_index: 3,
                    end_index: 9,
                    line_col: (1, 4),
                },
                attrs: vec![TagAttr {
                    key: None,
                    value: TagValue {
                        token: TagToken {
                            token: "value".to_string(),
                            start_index: 10,
                            end_index: 15,
                            line_col: (1, 11),
                        },
                        children: vec![],
                        spread: None,
                        filters: vec![TagValueFilter {
                            token: TagToken {
                                token: "default".to_string(),
                                start_index: 16,
                                end_index: 23,
                                line_col: (1, 17),
                            },
                            arg: Some(TagValue {
                                token: TagToken {
                                    token: "'hello'".to_string(),
                                    start_index: 24,
                                    end_index: 31,
                                    line_col: (1, 25),
                                },
                                children: vec![],
                                kind: ValueKind::String,
                                spread: None,
                                filters: vec![],
                                start_index: 23,
                                end_index: 31,
                                line_col: (1, 24),
                            }),
                            start_index: 15,
                            end_index: 31,
                            line_col: (1, 16),
                        }],
                        kind: ValueKind::Variable,
                        start_index: 10,
                        end_index: 31,
                        line_col: (1, 11),
                    },
                    is_flag: false,
                    start_index: 10,
                    end_index: 31,
                    line_col: (1, 11),
                }],
                is_self_closing: false,
                syntax: TagSyntax::Django,
                start_index: 0,
                end_index: 34,
                line_col: (1, 4),
            }
        );
    }

    #[test]
    fn test_filter_arg_number() {
        let input = "{% my_tag value|add:42 %}";
        let result = TagParser::parse_tag(input, &HashSet::new()).unwrap();
        assert_eq!(
            result,
            Tag {
                name: TagToken {
                    token: "my_tag".to_string(),
                    start_index: 3,
                    end_index: 9,
                    line_col: (1, 4),
                },
                attrs: vec![TagAttr {
                    key: None,
                    value: TagValue {
                        token: TagToken {
                            token: "value".to_string(),
                            start_index: 10,
                            end_index: 15,
                            line_col: (1, 11),
                        },
                        children: vec![],
                        spread: None,
                        filters: vec![TagValueFilter {
                            token: TagToken {
                                token: "add".to_string(),
                                start_index: 16,
                                end_index: 19,
                                line_col: (1, 17),
                            },
                            arg: Some(TagValue {
                                token: TagToken {
                                    token: "42".to_string(),
                                    start_index: 20,
                                    end_index: 22,
                                    line_col: (1, 21),
                                },
                                children: vec![],
                                kind: ValueKind::Int,
                                spread: None,
                                filters: vec![],
                                start_index: 19,
                                end_index: 22,
                                line_col: (1, 20),
                            }),
                            start_index: 15,
                            end_index: 22,
                            line_col: (1, 16),
                        }],
                        kind: ValueKind::Variable,
                        start_index: 10,
                        end_index: 22,
                        line_col: (1, 11),
                    },
                    is_flag: false,
                    start_index: 10,
                    end_index: 22,
                    line_col: (1, 11),
                }],
                is_self_closing: false,
                syntax: TagSyntax::Django,
                start_index: 0,
                end_index: 25,
                line_col: (1, 4),
            }
        );
    }

    #[test]
    fn test_filter_arg_variable() {
        let input = "{% my_tag value|default:my_var.field %}";
        let result = TagParser::parse_tag(input, &HashSet::new()).unwrap();
        assert_eq!(
            result,
            Tag {
                name: TagToken {
                    token: "my_tag".to_string(),
                    start_index: 3,
                    end_index: 9,
                    line_col: (1, 4),
                },
                attrs: vec![TagAttr {
                    key: None,
                    value: TagValue {
                        token: TagToken {
                            token: "value".to_string(),
                            start_index: 10,
                            end_index: 15,
                            line_col: (1, 11),
                        },
                        children: vec![],
                        spread: None,
                        filters: vec![TagValueFilter {
                            token: TagToken {
                                token: "default".to_string(),
                                start_index: 16,
                                end_index: 23,
                                line_col: (1, 17),
                            },
                            arg: Some(TagValue {
                                token: TagToken {
                                    token: "my_var.field".to_string(),
                                    start_index: 24,
                                    end_index: 36,
                                    line_col: (1, 25),
                                },
                                children: vec![],
                                kind: ValueKind::Variable,
                                spread: None,
                                filters: vec![],
                                start_index: 23,
                                end_index: 36,
                                line_col: (1, 24),
                            }),
                            start_index: 15,
                            end_index: 36,
                            line_col: (1, 16),
                        }],
                        kind: ValueKind::Variable,
                        start_index: 10,
                        end_index: 36,
                        line_col: (1, 11),
                    },
                    is_flag: false,
                    start_index: 10,
                    end_index: 36,
                    line_col: (1, 11),
                }],
                is_self_closing: false,
                syntax: TagSyntax::Django,
                start_index: 0,
                end_index: 39,
                line_col: (1, 4),
            }
        );
    }

    #[test]
    fn test_filter_arg_i18n() {
        let input = "{% my_tag value|default:_('hello') %}";
        let result = TagParser::parse_tag(input, &HashSet::new()).unwrap();
        assert_eq!(
            result,
            Tag {
                name: TagToken {
                    token: "my_tag".to_string(),
                    start_index: 3,
                    end_index: 9,
                    line_col: (1, 4),
                },
                attrs: vec![TagAttr {
                    key: None,
                    value: TagValue {
                        token: TagToken {
                            token: "value".to_string(),
                            start_index: 10,
                            end_index: 15,
                            line_col: (1, 11),
                        },
                        children: vec![],
                        spread: None,
                        filters: vec![TagValueFilter {
                            token: TagToken {
                                token: "default".to_string(),
                                start_index: 16,
                                end_index: 23,
                                line_col: (1, 17),
                            },
                            arg: Some(TagValue {
                                token: TagToken {
                                    token: "_('hello')".to_string(),
                                    start_index: 24,
                                    end_index: 34,
                                    line_col: (1, 25),
                                },
                                children: vec![],
                                kind: ValueKind::Translation,
                                spread: None,
                                filters: vec![],
                                start_index: 23,
                                end_index: 34,
                                line_col: (1, 24),
                            }),
                            start_index: 15,
                            end_index: 34,
                            line_col: (1, 16),
                        }],
                        kind: ValueKind::Variable,
                        start_index: 10,
                        end_index: 34,
                        line_col: (1, 11),
                    },
                    is_flag: false,
                    start_index: 10,
                    end_index: 34,
                    line_col: (1, 11),
                }],
                is_self_closing: false,
                syntax: TagSyntax::Django,
                start_index: 0,
                end_index: 37,
                line_col: (1, 4),
            }
        );
    }

    #[test]
    fn test_filter_arg_list() {
        let input = "{% my_tag value|default:[1, 2, 3] %}";
        let result = TagParser::parse_tag(input, &HashSet::new()).unwrap();
        assert_eq!(
            result,
            Tag {
                name: TagToken {
                    token: "my_tag".to_string(),
                    start_index: 3,
                    end_index: 9,
                    line_col: (1, 4),
                },
                attrs: vec![TagAttr {
                    key: None,
                    value: TagValue {
                        token: TagToken {
                            token: "value".to_string(),
                            start_index: 10,
                            end_index: 15,
                            line_col: (1, 11),
                        },
                        children: vec![],
                        spread: None,
                        filters: vec![TagValueFilter {
                            token: TagToken {
                                token: "default".to_string(),
                                start_index: 16,
                                end_index: 23,
                                line_col: (1, 17),
                            },
                            arg: Some(TagValue {
                                token: TagToken {
                                    token: "[1, 2, 3]".to_string(),
                                    start_index: 24,
                                    end_index: 33,
                                    line_col: (1, 25),
                                },
                                children: vec![
                                    TagValue {
                                        token: TagToken {
                                            token: "1".to_string(),
                                            start_index: 25,
                                            end_index: 26,
                                            line_col: (1, 26),
                                        },
                                        children: vec![],
                                        kind: ValueKind::Int,
                                        spread: None,
                                        filters: vec![],
                                        start_index: 25,
                                        end_index: 26,
                                        line_col: (1, 26),
                                    },
                                    TagValue {
                                        token: TagToken {
                                            token: "2".to_string(),
                                            start_index: 28,
                                            end_index: 29,
                                            line_col: (1, 29),
                                        },
                                        children: vec![],
                                        kind: ValueKind::Int,
                                        spread: None,
                                        filters: vec![],
                                        start_index: 28,
                                        end_index: 29,
                                        line_col: (1, 29),
                                    },
                                    TagValue {
                                        token: TagToken {
                                            token: "3".to_string(),
                                            start_index: 31,
                                            end_index: 32,
                                            line_col: (1, 32),
                                        },
                                        children: vec![],
                                        kind: ValueKind::Int,
                                        spread: None,
                                        filters: vec![],
                                        start_index: 31,
                                        end_index: 32,
                                        line_col: (1, 32),
                                    },
                                ],
                                kind: ValueKind::List,
                                spread: None,
                                filters: vec![],
                                start_index: 23,
                                end_index: 33,
                                line_col: (1, 24),
                            }),
                            start_index: 15,
                            end_index: 33,
                            line_col: (1, 16),
                        }],
                        kind: ValueKind::Variable,
                        start_index: 10,
                        end_index: 33,
                        line_col: (1, 11),
                    },
                    is_flag: false,
                    start_index: 10,
                    end_index: 33,
                    line_col: (1, 11),
                }],
                is_self_closing: false,
                syntax: TagSyntax::Django,
                start_index: 0,
                end_index: 36,
                line_col: (1, 4),
            }
        );
    }

    #[test]
    fn test_filter_arg_dict() {
        let input = r#"{% my_tag value|default:{"key": "val"} %}"#;
        let result = TagParser::parse_tag(input, &HashSet::new()).unwrap();
        assert_eq!(
            result,
            Tag {
                name: TagToken {
                    token: "my_tag".to_string(),
                    start_index: 3,
                    end_index: 9,
                    line_col: (1, 4),
                },
                attrs: vec![TagAttr {
                    key: None,
                    value: TagValue {
                        token: TagToken {
                            token: "value".to_string(),
                            start_index: 10,
                            end_index: 15,
                            line_col: (1, 11),
                        },
                        children: vec![],
                        spread: None,
                        filters: vec![TagValueFilter {
                            token: TagToken {
                                token: "default".to_string(),
                                start_index: 16,
                                end_index: 23,
                                line_col: (1, 17),
                            },
                            arg: Some(TagValue {
                                token: TagToken {
                                    token: "{\"key\": \"val\"}".to_string(),
                                    start_index: 24,
                                    end_index: 38,
                                    line_col: (1, 25),
                                },
                                children: vec![
                                    TagValue {
                                        token: TagToken {
                                            token: "\"key\"".to_string(),
                                            start_index: 25,
                                            end_index: 30,
                                            line_col: (1, 26),
                                        },
                                        children: vec![],
                                        kind: ValueKind::String,
                                        spread: None,
                                        filters: vec![],
                                        start_index: 25,
                                        end_index: 30,
                                        line_col: (1, 26),
                                    },
                                    TagValue {
                                        token: TagToken {
                                            token: "\"val\"".to_string(),
                                            start_index: 32,
                                            end_index: 37,
                                            line_col: (1, 33),
                                        },
                                        children: vec![],
                                        kind: ValueKind::String,
                                        spread: None,
                                        filters: vec![],
                                        start_index: 32,
                                        end_index: 37,
                                        line_col: (1, 33),
                                    },
                                ],
                                kind: ValueKind::Dict,
                                spread: None,
                                filters: vec![],
                                start_index: 23,
                                end_index: 38,
                                line_col: (1, 24),
                            }),
                            start_index: 15,
                            end_index: 38,
                            line_col: (1, 16),
                        }],
                        kind: ValueKind::Variable,
                        start_index: 10,
                        end_index: 38,
                        line_col: (1, 11),
                    },
                    is_flag: false,
                    start_index: 10,
                    end_index: 38,
                    line_col: (1, 11),
                }],
                is_self_closing: false,
                syntax: TagSyntax::Django,
                start_index: 0,
                end_index: 41,
                line_col: (1, 4),
            }
        );
    }

    #[test]
    fn test_filter_arg_template_string() {
        let input = r#"{% my_tag value|default:"{{ var }}" %}"#;
        let result = TagParser::parse_tag(input, &HashSet::new()).unwrap();
        assert_eq!(
            result,
            Tag {
                name: TagToken {
                    token: "my_tag".to_string(),
                    start_index: 3,
                    end_index: 9,
                    line_col: (1, 4),
                },
                attrs: vec![TagAttr {
                    key: None,
                    value: TagValue {
                        token: TagToken {
                            token: "value".to_string(),
                            start_index: 10,
                            end_index: 15,
                            line_col: (1, 11),
                        },
                        children: vec![],
                        spread: None,
                        filters: vec![TagValueFilter {
                            token: TagToken {
                                token: "default".to_string(),
                                start_index: 16,
                                end_index: 23,
                                line_col: (1, 17),
                            },
                            arg: Some(TagValue {
                                token: TagToken {
                                    token: "\"{{ var }}\"".to_string(),
                                    start_index: 24,
                                    end_index: 35,
                                    line_col: (1, 25),
                                },
                                children: vec![],
                                kind: ValueKind::TemplateString,
                                spread: None,
                                filters: vec![],
                                start_index: 23,
                                end_index: 35,
                                line_col: (1, 24),
                            }),
                            start_index: 15,
                            end_index: 35,
                            line_col: (1, 16),
                        }],
                        kind: ValueKind::Variable,
                        start_index: 10,
                        end_index: 35,
                        line_col: (1, 11),
                    },
                    is_flag: false,
                    start_index: 10,
                    end_index: 35,
                    line_col: (1, 11),
                }],
                is_self_closing: false,
                syntax: TagSyntax::Django,
                start_index: 0,
                end_index: 38,
                line_col: (1, 4),
            }
        );
    }

    #[test]
    fn test_filter_arg_nested() {
        let input = r#"{% my_tag value|default:[1, {"key": "val"}, _("hello")] %}"#;
        let result = TagParser::parse_tag(input, &HashSet::new()).unwrap();
        assert_eq!(
            result,
            Tag {
                name: TagToken {
                    token: "my_tag".to_string(),
                    start_index: 3,
                    end_index: 9,
                    line_col: (1, 4),
                },
                attrs: vec![TagAttr {
                    key: None,
                    value: TagValue {
                        token: TagToken {
                            token: "value".to_string(),
                            start_index: 10,
                            end_index: 15,
                            line_col: (1, 11),
                        },
                        children: vec![],
                        spread: None,
                        filters: vec![TagValueFilter {
                            token: TagToken {
                                token: "default".to_string(),
                                start_index: 16,
                                end_index: 23,
                                line_col: (1, 17),
                            },
                            arg: Some(TagValue {
                                token: TagToken {
                                    token: "[1, {\"key\": \"val\"}, _(\"hello\")]".to_string(),
                                    start_index: 24,
                                    end_index: 55,
                                    line_col: (1, 25),
                                },
                                children: vec![
                                    TagValue {
                                        token: TagToken {
                                            token: "1".to_string(),
                                            start_index: 25,
                                            end_index: 26,
                                            line_col: (1, 26),
                                        },
                                        children: vec![],
                                        kind: ValueKind::Int,
                                        spread: None,
                                        filters: vec![],
                                        start_index: 25,
                                        end_index: 26,
                                        line_col: (1, 26),
                                    },
                                    TagValue {
                                        token: TagToken {
                                            token: "{\"key\": \"val\"}".to_string(),
                                            start_index: 28,
                                            end_index: 42,
                                            line_col: (1, 29),
                                        },
                                        children: vec![
                                            TagValue {
                                                token: TagToken {
                                                    token: "\"key\"".to_string(),
                                                    start_index: 29,
                                                    end_index: 34,
                                                    line_col: (1, 30),
                                                },
                                                children: vec![],
                                                kind: ValueKind::String,
                                                spread: None,
                                                filters: vec![],
                                                start_index: 29,
                                                end_index: 34,
                                                line_col: (1, 30),
                                            },
                                            TagValue {
                                                token: TagToken {
                                                    token: "\"val\"".to_string(),
                                                    start_index: 36,
                                                    end_index: 41,
                                                    line_col: (1, 37),
                                                },
                                                children: vec![],
                                                kind: ValueKind::String,
                                                spread: None,
                                                filters: vec![],
                                                start_index: 36,
                                                end_index: 41,
                                                line_col: (1, 37),
                                            },
                                        ],
                                        kind: ValueKind::Dict,
                                        spread: None,
                                        filters: vec![],
                                        start_index: 28,
                                        end_index: 42,
                                        line_col: (1, 29),
                                    },
                                    TagValue {
                                        token: TagToken {
                                            token: "_(\"hello\")".to_string(),
                                            start_index: 44,
                                            end_index: 54,
                                            line_col: (1, 45),
                                        },
                                        children: vec![],
                                        kind: ValueKind::Translation,
                                        spread: None,
                                        filters: vec![],
                                        start_index: 44,
                                        end_index: 54,
                                        line_col: (1, 45),
                                    },
                                ],
                                kind: ValueKind::List,
                                spread: None,
                                filters: vec![],
                                start_index: 23,
                                end_index: 55,
                                line_col: (1, 24),
                            }),
                            start_index: 15,
                            end_index: 55,
                            line_col: (1, 16),
                        }],
                        kind: ValueKind::Variable,
                        start_index: 10,
                        end_index: 55,
                        line_col: (1, 11),
                    },
                    is_flag: false,
                    start_index: 10,
                    end_index: 55,
                    line_col: (1, 11),
                }],
                is_self_closing: false,
                syntax: TagSyntax::Django,
                start_index: 0,
                end_index: 58,
                line_col: (1, 4),
            }
        );
    }

    #[test]
    fn test_filter_invalid() {
        // Test using colon instead of pipe
        let input = "{% my_tag value:filter %}";
        assert!(
            TagParser::parse_tag(input, &HashSet::new()).is_err(),
            "Should not allow colon instead of pipe for filter"
        );

        // Test using colon with filter argument
        let input = "{% my_tag value:filter:arg %}";
        assert!(
            TagParser::parse_tag(input, &HashSet::new()).is_err(),
            "Should not allow colon instead of pipe for filter with argument"
        );

        // Test using colon after a valid filter
        let input = "{% my_tag value|filter:arg:filter2 %}";
        assert!(
            TagParser::parse_tag(input, &HashSet::new()).is_err(),
            "Should not allow colon to start a new filter after an argument"
        );
    }

    #[test]
    fn test_i18n_whitespace() {
        let input = "{% my_tag value|default:_( 'hello' ) %}";
        let result = TagParser::parse_tag(input, &HashSet::new()).unwrap();
        assert_eq!(
            result,
            Tag {
                name: TagToken {
                    token: "my_tag".to_string(),
                    start_index: 3,
                    end_index: 9,
                    line_col: (1, 4),
                },
                attrs: vec![TagAttr {
                    key: None,
                    value: TagValue {
                        token: TagToken {
                            token: "value".to_string(),
                            start_index: 10,
                            end_index: 15,
                            line_col: (1, 11),
                        },
                        children: vec![],
                        spread: None,
                        filters: vec![TagValueFilter {
                            token: TagToken {
                                token: "default".to_string(),
                                start_index: 16,
                                end_index: 23,
                                line_col: (1, 17),
                            },
                            arg: Some(TagValue {
                                token: TagToken {
                                    token: "_('hello')".to_string(),
                                    start_index: 24,
                                    end_index: 36,
                                    line_col: (1, 25),
                                },
                                children: vec![],
                                kind: ValueKind::Translation,
                                spread: None,
                                filters: vec![],
                                start_index: 23,
                                end_index: 36,
                                line_col: (1, 24),
                            }),
                            start_index: 15,
                            end_index: 36,
                            line_col: (1, 16),
                        }],
                        kind: ValueKind::Variable,
                        start_index: 10,
                        end_index: 36,
                        line_col: (1, 11),
                    },
                    is_flag: false,
                    start_index: 10,
                    end_index: 36,
                    line_col: (1, 11),
                }],
                is_self_closing: false,
                syntax: TagSyntax::Django,
                start_index: 0,
                end_index: 39,
                line_col: (1, 4),
            }
        );
    }

    #[test]
    fn test_i18n_comments() {
        let input = "{% my_tag value|default:_({# open paren #}'hello'{# close paren #}) %}";
        let result = TagParser::parse_tag(input, &HashSet::new()).unwrap();
        assert_eq!(
            result,
            Tag {
                name: TagToken {
                    token: "my_tag".to_string(),
                    start_index: 3,
                    end_index: 9,
                    line_col: (1, 4),
                },
                attrs: vec![TagAttr {
                    key: None,
                    value: TagValue {
                        token: TagToken {
                            token: "value".to_string(),
                            start_index: 10,
                            end_index: 15,
                            line_col: (1, 11),
                        },
                        children: vec![],
                        spread: None,
                        filters: vec![TagValueFilter {
                            token: TagToken {
                                token: "default".to_string(),
                                start_index: 16,
                                end_index: 23,
                                line_col: (1, 17),
                            },
                            arg: Some(TagValue {
                                token: TagToken {
                                    token: "_('hello')".to_string(),
                                    start_index: 24,
                                    end_index: 67,
                                    line_col: (1, 25),
                                },
                                children: vec![],
                                kind: ValueKind::Translation,
                                spread: None,
                                filters: vec![],
                                start_index: 23,
                                end_index: 67,
                                line_col: (1, 24),
                            }),
                            start_index: 15,
                            end_index: 67,
                            line_col: (1, 16),
                        }],
                        kind: ValueKind::Variable,
                        start_index: 10,
                        end_index: 67,
                        line_col: (1, 11),
                    },
                    is_flag: false,
                    start_index: 10,
                    end_index: 67,
                    line_col: (1, 11),
                }],
                is_self_closing: false,
                syntax: TagSyntax::Django,
                start_index: 0,
                end_index: 70,
                line_col: (1, 4),
            }
        );
    }

    #[test]
    fn test_list_empty() {
        // Empty list
        let input = "{% my_tag [] %}";
        let result = TagParser::parse_tag(input, &HashSet::new()).unwrap();
        assert_eq!(
            result,
            Tag {
                name: TagToken {
                    token: "my_tag".to_string(),
                    start_index: 3,
                    end_index: 9,
                    line_col: (1, 4),
                },
                attrs: vec![TagAttr {
                    key: None,
                    value: TagValue {
                        token: TagToken {
                            token: "[]".to_string(),
                            start_index: 10,
                            end_index: 12,
                            line_col: (1, 11),
                        },
                        children: vec![],
                        spread: None,
                        filters: vec![],
                        kind: ValueKind::List,
                        start_index: 10,
                        end_index: 12,
                        line_col: (1, 11),
                    },
                    is_flag: false,
                    start_index: 10,
                    end_index: 12,
                    line_col: (1, 11),
                }],
                is_self_closing: false,
                syntax: TagSyntax::Django,
                start_index: 0,
                end_index: 15,
                line_col: (1, 4),
            }
        );
    }

    #[test]
    fn test_list_basic() {
        // Simple list with numbers
        let input = "{% my_tag [1, 2, 3] %}";
        let result = TagParser::parse_tag(input, &HashSet::new()).unwrap();
        assert_eq!(
            result,
            Tag {
                name: TagToken {
                    token: "my_tag".to_string(),
                    start_index: 3,
                    end_index: 9,
                    line_col: (1, 4),
                },
                attrs: vec![TagAttr {
                    key: None,
                    value: TagValue {
                        token: TagToken {
                            token: "[1, 2, 3]".to_string(),
                            start_index: 10,
                            end_index: 19,
                            line_col: (1, 11),
                        },
                        children: vec![
                            TagValue {
                                token: TagToken {
                                    token: "1".to_string(),
                                    start_index: 11,
                                    end_index: 12,
                                    line_col: (1, 12),
                                },
                                spread: None,
                                filters: vec![],
                                children: vec![],
                                kind: ValueKind::Int,
                                start_index: 11,
                                end_index: 12,
                                line_col: (1, 12),
                            },
                            TagValue {
                                token: TagToken {
                                    token: "2".to_string(),
                                    start_index: 14,
                                    end_index: 15,
                                    line_col: (1, 15),
                                },
                                spread: None,
                                filters: vec![],
                                children: vec![],
                                kind: ValueKind::Int,
                                start_index: 14,
                                end_index: 15,
                                line_col: (1, 15),
                            },
                            TagValue {
                                token: TagToken {
                                    token: "3".to_string(),
                                    start_index: 17,
                                    end_index: 18,
                                    line_col: (1, 18),
                                },
                                spread: None,
                                filters: vec![],
                                children: vec![],
                                kind: ValueKind::Int,
                                start_index: 17,
                                end_index: 18,
                                line_col: (1, 18),
                            },
                        ],
                        spread: None,
                        filters: vec![],
                        kind: ValueKind::List,
                        start_index: 10,
                        end_index: 19,
                        line_col: (1, 11),
                    },
                    is_flag: false,
                    start_index: 10,
                    end_index: 19,
                    line_col: (1, 11),
                }],
                is_self_closing: false,
                syntax: TagSyntax::Django,
                start_index: 0,
                end_index: 22,
                line_col: (1, 4),
            }
        );
    }

    #[test]
    fn test_list_mixed() {
        // List with mixed types
        let input = "{% my_tag [42, 'hello', my_var] %}";
        let result = TagParser::parse_tag(input, &HashSet::new()).unwrap();
        assert_eq!(
            result,
            Tag {
                name: TagToken {
                    token: "my_tag".to_string(),
                    start_index: 3,
                    end_index: 9,
                    line_col: (1, 4),
                },
                attrs: vec![TagAttr {
                    key: None,
                    value: TagValue {
                        token: TagToken {
                            token: "[42, 'hello', my_var]".to_string(),
                            start_index: 10,
                            end_index: 31,
                            line_col: (1, 11),
                        },
                        children: vec![
                            TagValue {
                                token: TagToken {
                                    token: "42".to_string(),
                                    start_index: 11,
                                    end_index: 13,
                                    line_col: (1, 12),
                                },
                                spread: None,
                                filters: vec![],
                                children: vec![],
                                kind: ValueKind::Int,
                                start_index: 11,
                                end_index: 13,
                                line_col: (1, 12),
                            },
                            TagValue {
                                token: TagToken {
                                    token: "'hello'".to_string(),
                                    start_index: 15,
                                    end_index: 22,
                                    line_col: (1, 16),
                                },
                                spread: None,
                                filters: vec![],
                                children: vec![],
                                kind: ValueKind::String,
                                start_index: 15,
                                end_index: 22,
                                line_col: (1, 16),
                            },
                            TagValue {
                                token: TagToken {
                                    token: "my_var".to_string(),
                                    start_index: 24,
                                    end_index: 30,
                                    line_col: (1, 25),
                                },
                                spread: None,
                                filters: vec![],
                                children: vec![],
                                kind: ValueKind::Variable,
                                start_index: 24,
                                end_index: 30,
                                line_col: (1, 25),
                            },
                        ],
                        spread: None,
                        filters: vec![],
                        kind: ValueKind::List,
                        start_index: 10,
                        end_index: 31,
                        line_col: (1, 11),
                    },
                    is_flag: false,
                    start_index: 10,
                    end_index: 31,
                    line_col: (1, 11),
                }],
                is_self_closing: false,
                syntax: TagSyntax::Django,
                start_index: 0,
                end_index: 34,
                line_col: (1, 4),
            }
        );
    }

    #[test]
    fn test_list_filter() {
        // List with filter on the entire list
        let input = "{% my_tag [1, 2, 3]|first %}";
        let result = TagParser::parse_tag(input, &HashSet::new()).unwrap();
        assert_eq!(
            result,
            Tag {
                name: TagToken {
                    token: "my_tag".to_string(),
                    start_index: 3,
                    end_index: 9,
                    line_col: (1, 4),
                },
                attrs: vec![TagAttr {
                    key: None,
                    value: TagValue {
                        token: TagToken {
                            token: "[1, 2, 3]".to_string(),
                            start_index: 10,
                            end_index: 19,
                            line_col: (1, 11),
                        },
                        children: vec![
                            TagValue {
                                token: TagToken {
                                    token: "1".to_string(),
                                    start_index: 11,
                                    end_index: 12,
                                    line_col: (1, 12),
                                },
                                spread: None,
                                filters: vec![],
                                children: vec![],
                                kind: ValueKind::Int,
                                start_index: 11,
                                end_index: 12,
                                line_col: (1, 12),
                            },
                            TagValue {
                                token: TagToken {
                                    token: "2".to_string(),
                                    start_index: 14,
                                    end_index: 15,
                                    line_col: (1, 15),
                                },
                                spread: None,
                                filters: vec![],
                                children: vec![],
                                kind: ValueKind::Int,
                                start_index: 14,
                                end_index: 15,
                                line_col: (1, 15),
                            },
                            TagValue {
                                token: TagToken {
                                    token: "3".to_string(),
                                    start_index: 17,
                                    end_index: 18,
                                    line_col: (1, 18),
                                },
                                spread: None,
                                filters: vec![],
                                children: vec![],
                                kind: ValueKind::Int,
                                start_index: 17,
                                end_index: 18,
                                line_col: (1, 18),
                            },
                        ],
                        spread: None,
                        filters: vec![TagValueFilter {
                            token: TagToken {
                                token: "first".to_string(),
                                start_index: 20,
                                end_index: 25,
                                line_col: (1, 21),
                            },
                            arg: None,
                            start_index: 19,
                            end_index: 25,
                            line_col: (1, 20),
                        }],
                        kind: ValueKind::List,
                        start_index: 10,
                        end_index: 25,
                        line_col: (1, 11),
                    },
                    is_flag: false,
                    start_index: 10,
                    end_index: 25,
                    line_col: (1, 11),
                }],
                is_self_closing: false,
                syntax: TagSyntax::Django,
                start_index: 0,
                end_index: 28,
                line_col: (1, 4),
            }
        );
    }

    #[test]
    fn test_list_filter_item() {
        // List with filters on individual items
        let input = "{% my_tag ['hello'|upper, 'world'|title] %}";
        let result = TagParser::parse_tag(input, &HashSet::new()).unwrap();
        assert_eq!(
            result,
            Tag {
                name: TagToken {
                    token: "my_tag".to_string(),
                    start_index: 3,
                    end_index: 9,
                    line_col: (1, 4),
                },
                attrs: vec![TagAttr {
                    key: None,
                    value: TagValue {
                        token: TagToken {
                            token: "['hello'|upper, 'world'|title]".to_string(),
                            start_index: 10,
                            end_index: 40,
                            line_col: (1, 11),
                        },
                        children: vec![
                            TagValue {
                                token: TagToken {
                                    token: "'hello'".to_string(),
                                    start_index: 11,
                                    end_index: 18,
                                    line_col: (1, 12),
                                },
                                spread: None,
                                filters: vec![TagValueFilter {
                                    arg: None,
                                    token: TagToken {
                                        token: "upper".to_string(),
                                        start_index: 19,
                                        end_index: 24,
                                        line_col: (1, 20),
                                    },
                                    start_index: 18,
                                    end_index: 24,
                                    line_col: (1, 19),
                                }],
                                children: vec![],
                                kind: ValueKind::String,
                                start_index: 11,
                                end_index: 24,
                                line_col: (1, 12),
                            },
                            TagValue {
                                token: TagToken {
                                    token: "'world'".to_string(),
                                    start_index: 26,
                                    end_index: 33,
                                    line_col: (1, 27),
                                },
                                spread: None,
                                filters: vec![TagValueFilter {
                                    arg: None,
                                    token: TagToken {
                                        token: "title".to_string(),
                                        start_index: 34,
                                        end_index: 39,
                                        line_col: (1, 35),
                                    },
                                    start_index: 33,
                                    end_index: 39,
                                    line_col: (1, 34),
                                }],
                                children: vec![],
                                kind: ValueKind::String,
                                start_index: 26,
                                end_index: 39,
                                line_col: (1, 27),
                            },
                        ],
                        spread: None,
                        filters: vec![],
                        kind: ValueKind::List,
                        start_index: 10,
                        end_index: 40,
                        line_col: (1, 11),
                    },
                    is_flag: false,
                    start_index: 10,
                    end_index: 40,
                    line_col: (1, 11),
                }],
                is_self_closing: false,
                syntax: TagSyntax::Django,
                start_index: 0,
                end_index: 43,
                line_col: (1, 4),
            }
        );
    }

    #[test]
    fn test_list_filter_everywhere() {
        // List with both item filters and list filter
        let input = "{% my_tag ['a'|upper, 'b'|upper]|join:',' %}";
        let result = TagParser::parse_tag(input, &HashSet::new()).unwrap();
        assert_eq!(
            result,
            Tag {
                name: TagToken {
                    token: "my_tag".to_string(),
                    start_index: 3,
                    end_index: 9,
                    line_col: (1, 4),
                },
                attrs: vec![TagAttr {
                    key: None,
                    is_flag: false,
                    value: TagValue {
                        token: TagToken {
                            token: "['a'|upper, 'b'|upper]".to_string(),
                            start_index: 10,
                            end_index: 32,
                            line_col: (1, 11),
                        },
                        children: vec![
                            TagValue {
                                token: TagToken {
                                    token: "'a'".to_string(),
                                    start_index: 11,
                                    end_index: 14,
                                    line_col: (1, 12),
                                },
                                spread: None,
                                filters: vec![TagValueFilter {
                                    arg: None,
                                    token: TagToken {
                                        token: "upper".to_string(),
                                        start_index: 15,
                                        end_index: 20,
                                        line_col: (1, 16),
                                    },
                                    start_index: 14,
                                    end_index: 20,
                                    line_col: (1, 15),
                                }],
                                children: vec![],
                                kind: ValueKind::String,
                                start_index: 11,
                                end_index: 20,
                                line_col: (1, 12),
                            },
                            TagValue {
                                token: TagToken {
                                    token: "'b'".to_string(),
                                    start_index: 22,
                                    end_index: 25,
                                    line_col: (1, 23),
                                },
                                spread: None,
                                filters: vec![TagValueFilter {
                                    arg: None,
                                    token: TagToken {
                                        token: "upper".to_string(),
                                        start_index: 26,
                                        end_index: 31,
                                        line_col: (1, 27),
                                    },
                                    start_index: 25,
                                    end_index: 31,
                                    line_col: (1, 26),
                                }],
                                children: vec![],
                                kind: ValueKind::String,
                                start_index: 22,
                                end_index: 31,
                                line_col: (1, 23),
                            },
                        ],
                        spread: None,
                        filters: vec![TagValueFilter {
                            arg: Some(TagValue {
                                token: TagToken {
                                    token: "','".to_string(),
                                    start_index: 38,
                                    end_index: 41,
                                    line_col: (1, 39),
                                },
                                spread: None,
                                filters: vec![],
                                children: vec![],
                                kind: ValueKind::String,
                                start_index: 37,
                                end_index: 41,
                                line_col: (1, 38),
                            }),
                            token: TagToken {
                                token: "join".to_string(),
                                start_index: 33,
                                end_index: 37,
                                line_col: (1, 34),
                            },
                            start_index: 32,
                            end_index: 41,
                            line_col: (1, 33),
                        }],
                        kind: ValueKind::List,
                        start_index: 10,
                        end_index: 41,
                        line_col: (1, 11),
                    },
                    start_index: 10,
                    end_index: 41,
                    line_col: (1, 11),
                }],
                is_self_closing: false,
                syntax: TagSyntax::Django,
                start_index: 0,
                end_index: 44,
                line_col: (1, 4),
            }
        );
    }

    #[test]
    fn test_list_nested() {
        // Simple nested list
        let input = "{% my_tag [1, [2, 3], 4] %}";
        let result = TagParser::parse_tag(input, &HashSet::new()).unwrap();
        assert_eq!(
            result,
            Tag {
                name: TagToken {
                    token: "my_tag".to_string(),
                    start_index: 3,
                    end_index: 9,
                    line_col: (1, 4),
                },
                attrs: vec![TagAttr {
                    key: None,
                    is_flag: false,
                    value: TagValue {
                        token: TagToken {
                            token: "[1, [2, 3], 4]".to_string(),
                            start_index: 10,
                            end_index: 24,
                            line_col: (1, 11),
                        },
                        children: vec![
                            TagValue {
                                token: TagToken {
                                    token: "1".to_string(),
                                    start_index: 11,
                                    end_index: 12,
                                    line_col: (1, 12),
                                },
                                spread: None,
                                filters: vec![],
                                children: vec![],
                                kind: ValueKind::Int,
                                start_index: 11,
                                end_index: 12,
                                line_col: (1, 12),
                            },
                            TagValue {
                                token: TagToken {
                                    token: "[2, 3]".to_string(),
                                    start_index: 14,
                                    end_index: 20,
                                    line_col: (1, 15),
                                },
                                children: vec![
                                    TagValue {
                                        token: TagToken {
                                            token: "2".to_string(),
                                            start_index: 15,
                                            end_index: 16,
                                            line_col: (1, 16),
                                        },
                                        spread: None,
                                        filters: vec![],
                                        children: vec![],
                                        kind: ValueKind::Int,
                                        start_index: 15,
                                        end_index: 16,
                                        line_col: (1, 16),
                                    },
                                    TagValue {
                                        token: TagToken {
                                            token: "3".to_string(),
                                            start_index: 18,
                                            end_index: 19,
                                            line_col: (1, 19),
                                        },
                                        spread: None,
                                        filters: vec![],
                                        children: vec![],
                                        kind: ValueKind::Int,
                                        start_index: 18,
                                        end_index: 19,
                                        line_col: (1, 19),
                                    },
                                ],
                                spread: None,
                                filters: vec![],
                                kind: ValueKind::List,
                                start_index: 14,
                                end_index: 20,
                                line_col: (1, 15),
                            },
                            TagValue {
                                token: TagToken {
                                    token: "4".to_string(),
                                    start_index: 22,
                                    end_index: 23,
                                    line_col: (1, 23),
                                },
                                spread: None,
                                filters: vec![],
                                children: vec![],
                                kind: ValueKind::Int,
                                start_index: 22,
                                end_index: 23,
                                line_col: (1, 23),
                            },
                        ],
                        spread: None,
                        filters: vec![],
                        kind: ValueKind::List,
                        start_index: 10,
                        end_index: 24,
                        line_col: (1, 11),
                    },
                    start_index: 10,
                    end_index: 24,
                    line_col: (1, 11),
                }],
                is_self_closing: false,
                syntax: TagSyntax::Django,
                start_index: 0,
                end_index: 27,
                line_col: (1, 4),
            }
        );
    }

    #[test]
    fn test_list_nested_filter() {
        // Nested list with filters
        let input = "{% my_tag [[1, 2]|first, [3, 4]|last]|join:',' %}";
        let result = TagParser::parse_tag(input, &HashSet::new()).unwrap();
        assert_eq!(
            result,
            Tag {
                name: TagToken {
                    token: "my_tag".to_string(),
                    start_index: 3,
                    end_index: 9,
                    line_col: (1, 4),
                },
                attrs: vec![TagAttr {
                    key: None,
                    is_flag: false,
                    value: TagValue {
                        token: TagToken {
                            token: "[[1, 2]|first, [3, 4]|last]".to_string(),
                            start_index: 10,
                            end_index: 37,
                            line_col: (1, 11),
                        },
                        children: vec![
                            TagValue {
                                token: TagToken {
                                    token: "[1, 2]".to_string(),
                                    start_index: 11,
                                    end_index: 17,
                                    line_col: (1, 12),
                                },
                                children: vec![
                                    TagValue {
                                        token: TagToken {
                                            token: "1".to_string(),
                                            start_index: 12,
                                            end_index: 13,
                                            line_col: (1, 13),
                                        },
                                        spread: None,
                                        filters: vec![],
                                        children: vec![],
                                        kind: ValueKind::Int,
                                        start_index: 12,
                                        end_index: 13,
                                        line_col: (1, 13),
                                    },
                                    TagValue {
                                        token: TagToken {
                                            token: "2".to_string(),
                                            start_index: 15,
                                            end_index: 16,
                                            line_col: (1, 16),
                                        },
                                        spread: None,
                                        filters: vec![],
                                        children: vec![],
                                        kind: ValueKind::Int,
                                        start_index: 15,
                                        end_index: 16,
                                        line_col: (1, 16),
                                    },
                                ],
                                spread: None,
                                filters: vec![TagValueFilter {
                                    arg: None,
                                    token: TagToken {
                                        token: "first".to_string(),
                                        start_index: 18,
                                        end_index: 23,
                                        line_col: (1, 19),
                                    },
                                    start_index: 17,
                                    end_index: 23,
                                    line_col: (1, 18),
                                }],
                                kind: ValueKind::List,
                                start_index: 11,
                                end_index: 23,
                                line_col: (1, 12),
                            },
                            TagValue {
                                token: TagToken {
                                    token: "[3, 4]".to_string(),
                                    start_index: 25,
                                    end_index: 31,
                                    line_col: (1, 26),
                                },
                                children: vec![
                                    TagValue {
                                        token: TagToken {
                                            token: "3".to_string(),
                                            start_index: 26,
                                            end_index: 27,
                                            line_col: (1, 27),
                                        },
                                        spread: None,
                                        filters: vec![],
                                        children: vec![],
                                        kind: ValueKind::Int,
                                        start_index: 26,
                                        end_index: 27,
                                        line_col: (1, 27),
                                    },
                                    TagValue {
                                        token: TagToken {
                                            token: "4".to_string(),
                                            start_index: 29,
                                            end_index: 30,
                                            line_col: (1, 30),
                                        },
                                        spread: None,
                                        filters: vec![],
                                        children: vec![],
                                        kind: ValueKind::Int,
                                        start_index: 29,
                                        end_index: 30,
                                        line_col: (1, 30),
                                    },
                                ],
                                spread: None,
                                filters: vec![TagValueFilter {
                                    arg: None,
                                    token: TagToken {
                                        token: "last".to_string(),
                                        start_index: 32,
                                        end_index: 36,
                                        line_col: (1, 33),
                                    },
                                    start_index: 31,
                                    end_index: 36,
                                    line_col: (1, 32),
                                }],
                                kind: ValueKind::List,
                                start_index: 25,
                                end_index: 36,
                                line_col: (1, 26),
                            },
                        ],
                        spread: None,
                        filters: vec![TagValueFilter {
                            arg: Some(TagValue {
                                token: TagToken {
                                    token: "','".to_string(),
                                    start_index: 43,
                                    end_index: 46,
                                    line_col: (1, 44),
                                },
                                spread: None,
                                filters: vec![],
                                children: vec![],
                                kind: ValueKind::String,
                                start_index: 42,
                                end_index: 46,
                                line_col: (1, 43),
                            }),
                            token: TagToken {
                                token: "join".to_string(),
                                start_index: 38,
                                end_index: 42,
                                line_col: (1, 39),
                            },
                            start_index: 37,
                            end_index: 46,
                            line_col: (1, 38),
                        }],
                        kind: ValueKind::List,
                        start_index: 10,
                        end_index: 46,
                        line_col: (1, 11),
                    },
                    start_index: 10,
                    end_index: 46,
                    line_col: (1, 11),
                }],
                is_self_closing: false,
                syntax: TagSyntax::Django,
                start_index: 0,
                end_index: 49,
                line_col: (1, 4),
            }
        );
    }

    #[test]
    fn test_list_whitespace() {
        // Test whitespace in list
        let input = "{% my_tag [ 1 , 2 , 3 ] %}";
        let result = TagParser::parse_tag(input, &HashSet::new()).unwrap();
        assert_eq!(
            result,
            Tag {
                name: TagToken {
                    token: "my_tag".to_string(),
                    start_index: 3,
                    end_index: 9,
                    line_col: (1, 4),
                },
                attrs: vec![TagAttr {
                    key: None,
                    is_flag: false,
                    value: TagValue {
                        token: TagToken {
                            token: "[ 1 , 2 , 3 ]".to_string(),
                            start_index: 10,
                            end_index: 23,
                            line_col: (1, 11),
                        },
                        children: vec![
                            TagValue {
                                token: TagToken {
                                    token: "1".to_string(),
                                    start_index: 12,
                                    end_index: 13,
                                    line_col: (1, 13),
                                },
                                spread: None,
                                filters: vec![],
                                children: vec![],
                                kind: ValueKind::Int,
                                start_index: 12,
                                end_index: 13,
                                line_col: (1, 13),
                            },
                            TagValue {
                                token: TagToken {
                                    token: "2".to_string(),
                                    start_index: 16,
                                    end_index: 17,
                                    line_col: (1, 17),
                                },
                                spread: None,
                                filters: vec![],
                                children: vec![],
                                kind: ValueKind::Int,
                                start_index: 16,
                                end_index: 17,
                                line_col: (1, 17),
                            },
                            TagValue {
                                token: TagToken {
                                    token: "3".to_string(),
                                    start_index: 20,
                                    end_index: 21,
                                    line_col: (1, 21),
                                },
                                spread: None,
                                filters: vec![],
                                children: vec![],
                                kind: ValueKind::Int,
                                start_index: 20,
                                end_index: 21,
                                line_col: (1, 21),
                            },
                        ],
                        spread: None,
                        filters: vec![],
                        kind: ValueKind::List,
                        start_index: 10,
                        end_index: 23,
                        line_col: (1, 11),
                    },
                    start_index: 10,
                    end_index: 23,
                    line_col: (1, 11),
                }],
                is_self_closing: false,
                syntax: TagSyntax::Django,
                start_index: 0,
                end_index: 26,
                line_col: (1, 4),
            }
        );
    }

    #[test]
    fn test_list_comments() {
        // Test comments in list
        let input =
            "{% my_tag {# before start #}[{# first #}1,{# second #}2,{# third #}3{# end #}]{# after end #} %}";
        let result = TagParser::parse_tag(input, &HashSet::new()).unwrap();
        assert_eq!(
            result,
            Tag {
                name: TagToken {
                    token: "my_tag".to_string(),
                    start_index: 3,
                    end_index: 9,
                    line_col: (1, 4),
                },
                attrs: vec![TagAttr {
                    key: None,
                    is_flag: false,
                    value: TagValue {
                        token: TagToken {
                            token: "[{# first #}1,{# second #}2,{# third #}3{# end #}]".to_string(),
                            start_index: 28,
                            end_index: 78,
                            line_col: (1, 29),
                        },
                        children: vec![
                            TagValue {
                                token: TagToken {
                                    token: "1".to_string(),
                                    start_index: 40,
                                    end_index: 41,
                                    line_col: (1, 41),
                                },
                                spread: None,
                                filters: vec![],
                                children: vec![],
                                kind: ValueKind::Int,
                                start_index: 40,
                                end_index: 41,
                                line_col: (1, 41),
                            },
                            TagValue {
                                token: TagToken {
                                    token: "2".to_string(),
                                    start_index: 54,
                                    end_index: 55,
                                    line_col: (1, 55),
                                },
                                spread: None,
                                filters: vec![],
                                children: vec![],
                                kind: ValueKind::Int,
                                start_index: 54,
                                end_index: 55,
                                line_col: (1, 55),
                            },
                            TagValue {
                                token: TagToken {
                                    token: "3".to_string(),
                                    start_index: 67,
                                    end_index: 68,
                                    line_col: (1, 68),
                                },
                                spread: None,
                                filters: vec![],
                                children: vec![],
                                kind: ValueKind::Int,
                                start_index: 67,
                                end_index: 68,
                                line_col: (1, 68),
                            },
                        ],
                        spread: None,
                        filters: vec![],
                        kind: ValueKind::List,
                        start_index: 28,
                        end_index: 78,
                        line_col: (1, 29),
                    },
                    start_index: 28,
                    end_index: 78,
                    line_col: (1, 29),
                }],
                is_self_closing: false,
                syntax: TagSyntax::Django,
                start_index: 0,
                end_index: 96,
                line_col: (1, 4),
            }
        );
    }

    #[test]
    fn test_list_trailing_comma() {
        let input = "{% my_tag [1, 2, 3,] %}";
        let result = TagParser::parse_tag(input, &HashSet::new()).unwrap();
        assert_eq!(
            result,
            Tag {
                name: TagToken {
                    token: "my_tag".to_string(),
                    start_index: 3,
                    end_index: 9,
                    line_col: (1, 4),
                },
                attrs: vec![TagAttr {
                    key: None,
                    is_flag: false,
                    value: TagValue {
                        token: TagToken {
                            token: "[1, 2, 3,]".to_string(),
                            start_index: 10,
                            end_index: 20,
                            line_col: (1, 11),
                        },
                        children: vec![
                            TagValue {
                                token: TagToken {
                                    token: "1".to_string(),
                                    start_index: 11,
                                    end_index: 12,
                                    line_col: (1, 12),
                                },
                                spread: None,
                                filters: vec![],
                                children: vec![],
                                kind: ValueKind::Int,
                                start_index: 11,
                                end_index: 12,
                                line_col: (1, 12),
                            },
                            TagValue {
                                token: TagToken {
                                    token: "2".to_string(),
                                    start_index: 14,
                                    end_index: 15,
                                    line_col: (1, 15),
                                },
                                spread: None,
                                filters: vec![],
                                children: vec![],
                                kind: ValueKind::Int,
                                start_index: 14,
                                end_index: 15,
                                line_col: (1, 15),
                            },
                            TagValue {
                                token: TagToken {
                                    token: "3".to_string(),
                                    start_index: 17,
                                    end_index: 18,
                                    line_col: (1, 18),
                                },
                                spread: None,
                                filters: vec![],
                                children: vec![],
                                kind: ValueKind::Int,
                                start_index: 17,
                                end_index: 18,
                                line_col: (1, 18),
                            },
                        ],
                        spread: None,
                        filters: vec![],
                        kind: ValueKind::List,
                        start_index: 10,
                        end_index: 20,
                        line_col: (1, 11),
                    },
                    start_index: 10,
                    end_index: 20,
                    line_col: (1, 11),
                }],
                is_self_closing: false,
                syntax: TagSyntax::Django,
                start_index: 0,
                end_index: 23,
                line_col: (1, 4),
            }
        );
    }

    #[test]
    fn test_list_spread() {
        let input =
            "{% my_tag [1, *[2, 3], *{'a': 1}, *my_list, *'xyz', *_('hello'), *'{{ var }}', *3.14, 4] %}";
        let result = TagParser::parse_tag(input, &HashSet::new()).unwrap();
        assert_eq!(
            result,
            Tag {
                name: TagToken {
                    token: "my_tag".to_string(),
                    start_index: 3,
                    end_index: 9,
                    line_col: (1, 4),
                },
                attrs: vec![TagAttr {
                    key: None,
                    is_flag: false,
                    value: TagValue {
                        token: TagToken {
                            token: "[1, *[2, 3], *{'a': 1}, *my_list, *'xyz', *_('hello'), *'{{ var }}', *3.14, 4]".to_string(),
                            start_index: 10,
                            end_index: 88,
                            line_col: (1, 11),
                        },
                        children: vec![
                            TagValue {
                                token: TagToken {
                                    token: "1".to_string(),
                                    start_index: 11,
                                    end_index: 12,
                                    line_col: (1, 12),
                                },
                                spread: None,
                                filters: vec![],
                                children: vec![],
                                kind: ValueKind::Int,
                                start_index: 11,
                                end_index: 12,
                                line_col: (1, 12),
                            },
                            TagValue {
                                token: TagToken {
                                    token: "[2, 3]".to_string(),
                                    start_index: 15,
                                    end_index: 21,
                                    line_col: (1, 16),
                                },
                                children: vec![
                                    TagValue {
                                        token: TagToken {
                                            token: "2".to_string(),
                                            start_index: 16,
                                            end_index: 17,
                                            line_col: (1, 17),
                                        },
                                        spread: None,
                                        filters: vec![],
                                        children: vec![],
                                        kind: ValueKind::Int,
                                        start_index: 16,
                                        end_index: 17,
                                        line_col: (1, 17),
                                    },
                                    TagValue {
                                        token: TagToken {
                                            token: "3".to_string(),
                                            start_index: 19,
                                            end_index: 20,
                                            line_col: (1, 20),
                                        },
                                        spread: None,
                                        filters: vec![],
                                        children: vec![],
                                        kind: ValueKind::Int,
                                        start_index: 19,
                                        end_index: 20,
                                        line_col: (1, 20),
                                    },
                                ],
                                spread: Some("*".to_string()),
                                filters: vec![],
                                kind: ValueKind::List,
                                start_index: 14,
                                end_index: 21,
                                line_col: (1, 15),
                            },
                            TagValue {
                                token: TagToken {
                                    token: "{'a': 1}".to_string(),
                                    start_index: 24,
                                    end_index: 32,
                                    line_col: (1, 25),
                                },
                                children: vec![
                                    TagValue {
                                        token: TagToken {
                                            token: "'a'".to_string(),
                                            start_index: 25,
                                            end_index: 28,
                                            line_col: (1, 26),
                                        },
                                        spread: None,
                                        filters: vec![],
                                        children: vec![],
                                        kind: ValueKind::String,
                                        start_index: 25,
                                        end_index: 28,
                                        line_col: (1, 26),
                                    },
                                    TagValue {
                                        token: TagToken {
                                            token: "1".to_string(),
                                            start_index: 30,
                                            end_index: 31,
                                            line_col: (1, 31),
                                        },
                                        spread: None,
                                        filters: vec![],
                                        children: vec![],
                                        kind: ValueKind::Int,
                                        start_index: 30,
                                        end_index: 31,
                                        line_col: (1, 31),
                                    },
                                ],
                                spread: Some("*".to_string()),
                                filters: vec![],
                                kind: ValueKind::Dict,
                                start_index: 23,
                                end_index: 32,
                                line_col: (1, 24),
                            },
                            TagValue {
                                token: TagToken {
                                    token: "my_list".to_string(),
                                    start_index: 35,
                                    end_index: 42,
                                    line_col: (1, 36),
                                },
                                spread: Some("*".to_string()),
                                filters: vec![],
                                children: vec![],
                                kind: ValueKind::Variable,
                                start_index: 34,
                                end_index: 42,
                                line_col: (1, 35),
                            },
                            TagValue {
                                token: TagToken {
                                    token: "'xyz'".to_string(),
                                    start_index: 45,
                                    end_index: 50,
                                    line_col: (1, 46),
                                },
                                spread: Some("*".to_string()),
                                filters: vec![],
                                children: vec![],
                                kind: ValueKind::String,
                                start_index: 44,
                                end_index: 50,
                                line_col: (1, 45),
                            },
                            TagValue {
                                token: TagToken {
                                    token: "_('hello')".to_string(),
                                    start_index: 53,
                                    end_index: 63,
                                    line_col: (1, 54),
                                },
                                spread: Some("*".to_string()),
                                filters: vec![],
                                children: vec![],
                                kind: ValueKind::Translation,
                                start_index: 52,
                                end_index: 63,
                                line_col: (1, 53),
                            },
                            TagValue {
                                token: TagToken {
                                    token: "'{{ var }}'".to_string(),
                                    start_index: 66,
                                    end_index: 77,
                                    line_col: (1, 67),
                                },
                                spread: Some("*".to_string()),
                                filters: vec![],
                                children: vec![],
                                kind: ValueKind::TemplateString,
                                start_index: 65,
                                end_index: 77,
                                line_col: (1, 66),
                            },
                            TagValue {
                                token: TagToken {
                                    token: "3.14".to_string(),
                                    start_index: 80,
                                    end_index: 84,
                                    line_col: (1, 81),
                                },
                                spread: Some("*".to_string()),
                                filters: vec![],
                                children: vec![],
                                kind: ValueKind::Float,
                                start_index: 79,
                                end_index: 84,
                                line_col: (1, 80),
                            },
                            TagValue {
                                token: TagToken {
                                    token: "4".to_string(),
                                    start_index: 86,
                                    end_index: 87,
                                    line_col: (1, 87),
                                },
                                spread: None,
                                filters: vec![],
                                children: vec![],
                                kind: ValueKind::Int,
                                start_index: 86,
                                end_index: 87,
                                line_col: (1, 87),
                            },
                        ],
                        spread: None,
                        filters: vec![],
                        kind: ValueKind::List,
                        start_index: 10,
                        end_index: 88,
                        line_col: (1, 11),
                    },
                    start_index: 10,
                    end_index: 88,
                    line_col: (1, 11),
                }],
                is_self_closing: false,
                syntax: TagSyntax::Django,
                start_index: 0,
                end_index: 91,
                line_col: (1, 4),
            }
        );
    }

    #[test]
    fn test_list_spread_filter() {
        let input = "{% my_tag [1, *[2|upper, 3|lower], *{'a': 1}|default:empty, *my_list|join:\",\", *'xyz'|upper, *_('hello')|escape, *'{{ var }}'|safe, *3.14|round, 4|default:0] %}";
        let result = TagParser::parse_tag(input, &HashSet::new()).unwrap();
        assert_eq!(
            result,
            Tag {
                name: TagToken {
                    token: "my_tag".to_string(),
                    start_index: 3,
                    end_index: 9,
                    line_col: (1, 4),
                },
                attrs: vec![TagAttr {
                    key: None,
                    is_flag: false,
                    value: TagValue {
                        token: TagToken {
                            token: "[1, *[2|upper, 3|lower], *{'a': 1}|default:empty, *my_list|join:\",\", *'xyz'|upper, *_('hello')|escape, *'{{ var }}'|safe, *3.14|round, 4|default:0]".to_string(),
                            start_index: 10,
                            end_index: 157,
                            line_col: (1, 11),
                        },
                        children: vec![
                            TagValue {
                                token: TagToken {
                                    token: "1".to_string(),
                                    start_index: 11,
                                    end_index: 12,
                                    line_col: (1, 12),
                                },
                                spread: None,
                                filters: vec![],
                                children: vec![],
                                kind: ValueKind::Int,
                                start_index: 11,
                                end_index: 12,
                                line_col: (1, 12),
                            },
                            TagValue {
                                token: TagToken {
                                    token: "[2|upper, 3|lower]".to_string(),
                                    start_index: 15,
                                    end_index: 33,
                                    line_col: (1, 16),
                                },
                                children: vec![
                                    TagValue {
                                        token: TagToken {
                                            token: "2".to_string(),
                                            start_index: 16,
                                            end_index: 17,
                                            line_col: (1, 17),
                                        },
                                        spread: None,
                                        children: vec![],
                                        filters: vec![TagValueFilter {
                                            arg: None,
                                            token: TagToken {
                                                token: "upper".to_string(),
                                                start_index: 18,
                                                end_index: 23,
                                                line_col: (1, 19),
                                            },
                                            start_index: 17,
                                            end_index: 23,
                                            line_col: (1, 18),
                                        }],
                                        kind: ValueKind::Int,
                                        start_index: 16,
                                        end_index: 23,
                                        line_col: (1, 17),
                                    },
                                    TagValue {
                                        token: TagToken {
                                            token: "3".to_string(),
                                            start_index: 25,
                                            end_index: 26,
                                            line_col: (1, 26),
                                        },
                                        spread: None,
                                        children: vec![],
                                        filters: vec![TagValueFilter {
                                            arg: None,
                                            token: TagToken {
                                                token: "lower".to_string(),
                                                start_index: 27,
                                                end_index: 32,
                                                line_col: (1, 28),
                                            },
                                            start_index: 26,
                                            end_index: 32,
                                            line_col: (1, 27),
                                        }],
                                        kind: ValueKind::Int,
                                        start_index: 25,
                                        end_index: 32,
                                        line_col: (1, 26),
                                    },
                                ],
                                spread: Some("*".to_string()),
                                filters: vec![],
                                kind: ValueKind::List,
                                start_index: 14,
                                end_index: 33,
                                line_col: (1, 15),
                            },
                            TagValue {
                                token: TagToken {
                                    token: "{'a': 1}".to_string(),
                                    start_index: 36,
                                    end_index: 44,
                                    line_col: (1, 37),
                                },
                                children: vec![
                                    TagValue {
                                        token: TagToken {
                                            token: "'a'".to_string(),
                                            start_index: 37,
                                            end_index: 40,
                                            line_col: (1, 38),
                                        },
                                        spread: None,
                                        filters: vec![],
                                        children: vec![],
                                        kind: ValueKind::String,
                                        start_index: 37,
                                        end_index: 40,
                                        line_col: (1, 38),
                                    },
                                    TagValue {
                                        token: TagToken {
                                            token: "1".to_string(),
                                            start_index: 42,
                                            end_index: 43,
                                            line_col: (1, 43),
                                        },
                                        spread: None,
                                        filters: vec![],
                                        children: vec![],
                                        kind: ValueKind::Int,
                                        start_index: 42,
                                        end_index: 43,
                                        line_col: (1, 43),
                                    },
                                ],
                                spread: Some("*".to_string()),
                                filters: vec![TagValueFilter {
                                    token: TagToken {
                                        token: "default".to_string(),
                                        start_index: 45,
                                        end_index: 52,
                                        line_col: (1, 46),
                                    },
                                    arg: Some(TagValue {
                                        token: TagToken {
                                            token: "empty".to_string(),
                                            start_index: 53,
                                            end_index: 58,
                                            line_col: (1, 54),
                                        },
                                        spread: None,
                                        filters: vec![],
                                        children: vec![],
                                        kind: ValueKind::Variable,
                                        start_index: 52,
                                        end_index: 58,
                                        line_col: (1, 53),
                                    }),
                                    start_index: 44,
                                    end_index: 58,
                                    line_col: (1, 45),
                                }],
                                kind: ValueKind::Dict,
                                start_index: 35,
                                end_index: 58,
                                line_col: (1, 36),
                            },
                            TagValue {
                                token: TagToken {
                                    token: "my_list".to_string(),
                                    start_index: 61,
                                    end_index: 68,
                                    line_col: (1, 62),
                                },
                                spread: Some("*".to_string()),
                                children: vec![],
                                filters: vec![TagValueFilter {
                                    token: TagToken {
                                        token: "join".to_string(),
                                        start_index: 69,
                                        end_index: 73,
                                        line_col: (1, 70),
                                    },
                                    arg: Some(TagValue {
                                        token: TagToken {
                                            token: "\",\"".to_string(),
                                            start_index: 74,
                                            end_index: 77,
                                            line_col: (1, 75),
                                        },
                                        spread: None,
                                        filters: vec![],
                                        children: vec![],
                                        kind: ValueKind::String,
                                        start_index: 73,
                                        end_index: 77,
                                        line_col: (1, 74),
                                    }),
                                    start_index: 68,
                                    end_index: 77,
                                    line_col: (1, 69),
                                }],
                                kind: ValueKind::Variable,
                                start_index: 60,
                                end_index: 77,
                                line_col: (1, 61),
                            },
                            TagValue {
                                token: TagToken {
                                    token: "'xyz'".to_string(),
                                    start_index: 80,
                                    end_index: 85,
                                    line_col: (1, 81),
                                },
                                spread: Some("*".to_string()),
                                children: vec![],
                                filters: vec![TagValueFilter {
                                    token: TagToken {
                                        token: "upper".to_string(),
                                        start_index: 86,
                                        end_index: 91,
                                        line_col: (1, 87),
                                    },
                                    arg: None,
                                    start_index: 85,
                                    end_index: 91,
                                    line_col: (1, 86),
                                }],
                                kind: ValueKind::String,
                                start_index: 79,
                                end_index: 91,
                                line_col: (1, 80),
                            },
                            TagValue {
                                token: TagToken {
                                    token: "_('hello')".to_string(),
                                    start_index: 94,
                                    end_index: 104,
                                    line_col: (1, 95),
                                },
                                spread: Some("*".to_string()),
                                children: vec![],
                                filters: vec![TagValueFilter {
                                    token: TagToken {
                                        token: "escape".to_string(),
                                        start_index: 105,
                                        end_index: 111,
                                        line_col: (1, 106),
                                    },
                                    arg: None,
                                    start_index: 104,
                                    end_index: 111,
                                    line_col: (1, 105),
                                }],
                                kind: ValueKind::Translation,
                                start_index: 93,
                                end_index: 111,
                                line_col: (1, 94),
                            },
                            TagValue {
                                token: TagToken {
                                    token: "'{{ var }}'".to_string(),
                                    start_index: 114,
                                    end_index: 125,
                                    line_col: (1, 115),
                                },
                                spread: Some("*".to_string()),
                                children: vec![],
                                filters: vec![TagValueFilter {
                                    token: TagToken {
                                        token: "safe".to_string(),
                                        start_index: 126,
                                        end_index: 130,
                                        line_col: (1, 127),
                                    },
                                    arg: None,
                                    start_index: 125,
                                    end_index: 130,
                                    line_col: (1, 126),
                                }],
                                kind: ValueKind::TemplateString,
                                start_index: 113,
                                end_index: 130,
                                line_col: (1, 114),
                            },
                            TagValue {
                                token: TagToken {
                                    token: "3.14".to_string(),
                                    start_index: 133,
                                    end_index: 137,
                                    line_col: (1, 134),
                                },
                                spread: Some("*".to_string()),
                                children: vec![],
                                filters: vec![TagValueFilter {
                                    token: TagToken {
                                        token: "round".to_string(),
                                        start_index: 138,
                                        end_index: 143,
                                        line_col: (1, 139),
                                    },
                                    arg: None,
                                    start_index: 137,
                                    end_index: 143,
                                    line_col: (1, 138),
                                }],
                                kind: ValueKind::Float,
                                start_index: 132,
                                end_index: 143,
                                line_col: (1, 133),
                            },
                            TagValue {
                                token: TagToken {
                                    token: "4".to_string(),
                                    start_index: 145,
                                    end_index: 146,
                                    line_col: (1, 146),
                                },
                                spread: None,
                                children: vec![],
                                filters: vec![TagValueFilter {
                                    token: TagToken {
                                        token: "default".to_string(),
                                        start_index: 147,
                                        end_index: 154,
                                        line_col: (1, 148),
                                    },
                                    arg: Some(TagValue {
                                        token: TagToken {
                                            token: "0".to_string(),
                                            start_index: 155,
                                            end_index: 156,
                                            line_col: (1, 156),
                                        },
                                        spread: None,
                                        filters: vec![],
                                        children: vec![],
                                        kind: ValueKind::Int,
                                        start_index: 154,
                                        end_index: 156,
                                        line_col: (1, 155),
                                    }),
                                    start_index: 146,
                                    end_index: 156,
                                    line_col: (1, 147),
                                }],
                                kind: ValueKind::Int,
                                start_index: 145,
                                end_index: 156,
                                line_col: (1, 146),
                            },
                        ],
                        spread: None,
                        filters: vec![],
                        kind: ValueKind::List,
                        start_index: 10,
                        end_index: 157,
                        line_col: (1, 11),
                    },
                    start_index: 10,
                    end_index: 157,
                    line_col: (1, 11),
                }],
                is_self_closing: false,
                syntax: TagSyntax::Django,
                start_index: 0,
                end_index: 160,
                line_col: (1, 4),
            }
        );
    }

    #[test]
    fn test_list_spread_invalid() {
        // Test asterisk at top level as value-only
        let input = "{% my_tag *value %}";
        assert!(
            TagParser::parse_tag(input, &HashSet::new()).is_err(),
            "Should not allow asterisk operator at top level"
        );

        // Test asterisk in value position of key-value pair
        let input = "{% my_tag key=*value %}";
        assert!(
            TagParser::parse_tag(input, &HashSet::new()).is_err(),
            "Should not allow asterisk operator in value position of key-value pair"
        );

        // Test asterisk in key position
        let input = "{% my_tag *key=value %}";
        assert!(
            TagParser::parse_tag(input, &HashSet::new()).is_err(),
            "Should not allow asterisk operator in key position"
        );

        // Test asterisk with nested list at top level
        let input = "{% my_tag *[1, 2, 3] %}";
        assert!(
            TagParser::parse_tag(input, &HashSet::new()).is_err(),
            "Should not allow asterisk operator with list at top level"
        );

        // Test asterisk with nested list in key-value pair
        let input = "{% my_tag key=*[1, 2, 3] %}";
        assert!(
            TagParser::parse_tag(input, &HashSet::new()).is_err(),
            "Should not allow asterisk operator with list in key-value pair"
        );

        // Test combining spread operators
        let input = "{% my_tag ...*[1, 2, 3] %}";
        assert!(
            TagParser::parse_tag(input, &HashSet::new()).is_err(),
            "Should not allow combining spread operators"
        );

        // Test combining spread operators with variable
        let input = "{% my_tag ...*my_list %}";
        assert!(
            TagParser::parse_tag(input, &HashSet::new()).is_err(),
            "Should not allow combining spread operators with variable"
        );

        // Test combining spread operators
        let input = "{% my_tag *...[1, 2, 3] %}";
        assert!(
            TagParser::parse_tag(input, &HashSet::new()).is_err(),
            "Should not allow combining spread operators"
        );
    }

    #[test]
    fn test_list_spread_comments() {
        // Test comments before / after spread
        let input = "{% my_tag [{# ... #}*{# ... #}1,*{# ... #}2,{# ... #}3] %}";
        let result = TagParser::parse_tag(input, &HashSet::new()).unwrap();
        assert_eq!(
            result,
            Tag {
                name: TagToken {
                    token: "my_tag".to_string(),
                    start_index: 3,
                    end_index: 9,
                    line_col: (1, 4),
                },
                attrs: vec![TagAttr {
                    key: None,
                    value: TagValue {
                        token: TagToken {
                            token: "[{# ... #}*{# ... #}1,*{# ... #}2,{# ... #}3]".to_string(),
                            start_index: 10,
                            end_index: 55,
                            line_col: (1, 11),
                        },
                        children: vec![
                            TagValue {
                                token: TagToken {
                                    token: "1".to_string(),
                                    start_index: 30,
                                    end_index: 31,
                                    line_col: (1, 31),
                                },
                                spread: Some("*".to_string()),
                                filters: vec![],
                                children: vec![],
                                kind: ValueKind::Int,
                                start_index: 29,
                                end_index: 31,
                                line_col: (1, 30),
                            },
                            TagValue {
                                token: TagToken {
                                    token: "2".to_string(),
                                    start_index: 42,
                                    end_index: 43,
                                    line_col: (1, 43),
                                },
                                spread: Some("*".to_string()),
                                filters: vec![],
                                children: vec![],
                                kind: ValueKind::Int,
                                start_index: 41,
                                end_index: 43,
                                line_col: (1, 42),
                            },
                            TagValue {
                                token: TagToken {
                                    token: "3".to_string(),
                                    start_index: 53,
                                    end_index: 54,
                                    line_col: (1, 54),
                                },
                                spread: None,
                                filters: vec![],
                                children: vec![],
                                kind: ValueKind::Int,
                                start_index: 53,
                                end_index: 54,
                                line_col: (1, 54),
                            },
                        ],
                        spread: None,
                        filters: vec![],
                        kind: ValueKind::List,
                        start_index: 10,
                        end_index: 55,
                        line_col: (1, 11),
                    },
                    is_flag: false,
                    start_index: 10,
                    end_index: 55,
                    line_col: (1, 11),
                }],
                is_self_closing: false,
                syntax: TagSyntax::Django,
                start_index: 0,
                end_index: 58,
                line_col: (1, 4),
            }
        );
    }

    #[test]
    fn test_list_spread_nested_comments() {
        // Test comments with nested spread
        let input =
            "{% my_tag {# c0 #}[1, {# c1 #}*{# c2 #}[2, {# c3 #}*{# c4 #}[3, 4]], 5]{# c5 #} %}";
        let result = TagParser::parse_tag(input, &HashSet::new()).unwrap();
        assert_eq!(
            result,
            Tag {
                name: TagToken {
                    token: "my_tag".to_string(),
                    start_index: 3,
                    end_index: 9,
                    line_col: (1, 4),
                },
                attrs: vec![TagAttr {
                    key: None,
                    is_flag: false,
                    value: TagValue {
                        token: TagToken {
                            token: "[1, {# c1 #}*{# c2 #}[2, {# c3 #}*{# c4 #}[3, 4]], 5]"
                                .to_string(),
                            start_index: 18,
                            end_index: 71,
                            line_col: (1, 19),
                        },
                        children: vec![
                            TagValue {
                                token: TagToken {
                                    token: "1".to_string(),
                                    start_index: 19,
                                    end_index: 20,
                                    line_col: (1, 20),
                                },
                                spread: None,
                                filters: vec![],
                                children: vec![],
                                kind: ValueKind::Int,
                                start_index: 19,
                                end_index: 20,
                                line_col: (1, 20),
                            },
                            TagValue {
                                token: TagToken {
                                    token: "[2, {# c3 #}*{# c4 #}[3, 4]]".to_string(),
                                    start_index: 39,
                                    end_index: 67,
                                    line_col: (1, 40),
                                },
                                children: vec![
                                    TagValue {
                                        token: TagToken {
                                            token: "2".to_string(),
                                            start_index: 40,
                                            end_index: 41,
                                            line_col: (1, 41),
                                        },
                                        spread: None,
                                        children: vec![],
                                        filters: vec![],
                                        kind: ValueKind::Int,
                                        start_index: 40,
                                        end_index: 41,
                                        line_col: (1, 41),
                                    },
                                    TagValue {
                                        token: TagToken {
                                            token: "[3, 4]".to_string(),
                                            start_index: 60,
                                            end_index: 66,
                                            line_col: (1, 61),
                                        },
                                        children: vec![
                                            TagValue {
                                                token: TagToken {
                                                    token: "3".to_string(),
                                                    start_index: 61,
                                                    end_index: 62,
                                                    line_col: (1, 62),
                                                },
                                                spread: None,
                                                filters: vec![],
                                                children: vec![],
                                                kind: ValueKind::Int,
                                                start_index: 61,
                                                end_index: 62,
                                                line_col: (1, 62),
                                            },
                                            TagValue {
                                                token: TagToken {
                                                    token: "4".to_string(),
                                                    start_index: 64,
                                                    end_index: 65,
                                                    line_col: (1, 65),
                                                },
                                                spread: None,
                                                filters: vec![],
                                                children: vec![],
                                                kind: ValueKind::Int,
                                                start_index: 64,
                                                end_index: 65,
                                                line_col: (1, 65),
                                            },
                                        ],
                                        spread: Some("*".to_string()),
                                        filters: vec![],
                                        kind: ValueKind::List,
                                        start_index: 59,
                                        end_index: 66,
                                        line_col: (1, 60),
                                    },
                                ],
                                spread: Some("*".to_string()),
                                filters: vec![],
                                kind: ValueKind::List,
                                start_index: 38,
                                end_index: 67,
                                line_col: (1, 39),
                            },
                            TagValue {
                                token: TagToken {
                                    token: "5".to_string(),
                                    start_index: 69,
                                    end_index: 70,
                                    line_col: (1, 70),
                                },
                                spread: None,
                                filters: vec![],
                                children: vec![],
                                kind: ValueKind::Int,
                                start_index: 69,
                                end_index: 70,
                                line_col: (1, 70),
                            },
                        ],
                        spread: None,
                        filters: vec![],
                        kind: ValueKind::List,
                        start_index: 18,
                        end_index: 71,
                        line_col: (1, 19),
                    },
                    start_index: 18,
                    end_index: 71,
                    line_col: (1, 19),
                }],
                is_self_closing: false,
                syntax: TagSyntax::Django,
                start_index: 0,
                end_index: 82,
                line_col: (1, 4),
            }
        );
    }

    #[test]
    fn test_template_string_negative() {
        // Test simple string without template string
        let input = "{% my_tag \"Hello\" %}";
        let result = TagParser::parse_tag(input, &HashSet::new()).unwrap();
        assert_eq!(
            result,
            Tag {
                name: TagToken {
                    token: "my_tag".to_string(),
                    start_index: 3,
                    end_index: 9,
                    line_col: (1, 4),
                },
                attrs: vec![TagAttr {
                    key: None,
                    value: TagValue {
                        token: TagToken {
                            token: "\"Hello\"".to_string(),
                            start_index: 10,
                            end_index: 17,
                            line_col: (1, 11),
                        },
                        children: vec![],
                        spread: None,
                        filters: vec![],
                        kind: ValueKind::String,
                        start_index: 10,
                        end_index: 17,
                        line_col: (1, 11),
                    },
                    is_flag: false,
                    start_index: 10,
                    end_index: 17,
                    line_col: (1, 11),
                }],
                is_self_closing: false,
                syntax: TagSyntax::Django,
                start_index: 0,
                end_index: 20,
                line_col: (1, 4),
            }
        );
    }

    #[test]
    fn test_template_string_block() {
        // Test string with {% tag %}
        let input = "{% my_tag \"Hello {% lorem w 1 %}\" %}";
        let result = TagParser::parse_tag(input, &HashSet::new()).unwrap();
        assert_eq!(
            result,
            Tag {
                name: TagToken {
                    token: "my_tag".to_string(),
                    start_index: 3,
                    end_index: 9,
                    line_col: (1, 4),
                },
                attrs: vec![TagAttr {
                    key: None,
                    value: TagValue {
                        token: TagToken {
                            token: "\"Hello {% lorem w 1 %}\"".to_string(),
                            start_index: 10,
                            end_index: 33,
                            line_col: (1, 11),
                        },
                        children: vec![],
                        spread: None,
                        filters: vec![],
                        kind: ValueKind::TemplateString,
                        start_index: 10,
                        end_index: 33,
                        line_col: (1, 11),
                    },
                    is_flag: false,
                    start_index: 10,
                    end_index: 33,
                    line_col: (1, 11),
                }],
                is_self_closing: false,
                syntax: TagSyntax::Django,
                start_index: 0,
                end_index: 36,
                line_col: (1, 4),
            }
        );
    }

    #[test]
    fn test_template_string_variable() {
        // Test string with {{ variable }}
        let input = "{% my_tag \"Hello {{ last_name }}\" %}";
        let result = TagParser::parse_tag(input, &HashSet::new()).unwrap();
        assert_eq!(
            result,
            Tag {
                name: TagToken {
                    token: "my_tag".to_string(),
                    start_index: 3,
                    end_index: 9,
                    line_col: (1, 4),
                },
                attrs: vec![TagAttr {
                    key: None,
                    value: TagValue {
                        token: TagToken {
                            token: "\"Hello {{ last_name }}\"".to_string(),
                            start_index: 10,
                            end_index: 33,
                            line_col: (1, 11),
                        },
                        children: vec![],
                        spread: None,
                        filters: vec![],
                        kind: ValueKind::TemplateString,
                        start_index: 10,
                        end_index: 33,
                        line_col: (1, 11),
                    },
                    is_flag: false,
                    start_index: 10,
                    end_index: 33,
                    line_col: (1, 11),
                }],
                is_self_closing: false,
                syntax: TagSyntax::Django,
                start_index: 0,
                end_index: 36,
                line_col: (1, 4),
            }
        );
    }

    #[test]
    fn test_template_string_comment() {
        // Test string with {# comment #}
        let input = "{% my_tag \"Hello {# TODO #}\" %}";
        let result = TagParser::parse_tag(input, &HashSet::new()).unwrap();
        assert_eq!(
            result,
            Tag {
                name: TagToken {
                    token: "my_tag".to_string(),
                    start_index: 3,
                    end_index: 9,
                    line_col: (1, 4),
                },
                attrs: vec![TagAttr {
                    key: None,
                    value: TagValue {
                        token: TagToken {
                            token: "\"Hello {# TODO #}\"".to_string(),
                            start_index: 10,
                            end_index: 28,
                            line_col: (1, 11),
                        },
                        children: vec![],
                        spread: None,
                        filters: vec![],
                        kind: ValueKind::TemplateString,
                        start_index: 10,
                        end_index: 28,
                        line_col: (1, 11),
                    },
                    is_flag: false,
                    start_index: 10,
                    end_index: 28,
                    line_col: (1, 11),
                }],
                is_self_closing: false,
                syntax: TagSyntax::Django,
                start_index: 0,
                end_index: 31,
                line_col: (1, 4),
            }
        );
    }

    #[test]
    fn test_template_string_mixed() {
        // Test string with multiple template tags
        let input = "{% my_tag \"Hello {{ first_name }} {% lorem 1 w %} {# TODO #}\" %}";
        let result = TagParser::parse_tag(input, &HashSet::new()).unwrap();
        assert_eq!(
            result,
            Tag {
                name: TagToken {
                    token: "my_tag".to_string(),
                    start_index: 3,
                    end_index: 9,
                    line_col: (1, 4),
                },
                attrs: vec![TagAttr {
                    key: None,
                    value: TagValue {
                        token: TagToken {
                            token: "\"Hello {{ first_name }} {% lorem 1 w %} {# TODO #}\""
                                .to_string(),
                            start_index: 10,
                            end_index: 61,
                            line_col: (1, 11),
                        },
                        children: vec![],
                        spread: None,
                        filters: vec![],
                        kind: ValueKind::TemplateString,
                        start_index: 10,
                        end_index: 61,
                        line_col: (1, 11),
                    },
                    is_flag: false,
                    start_index: 10,
                    end_index: 61,
                    line_col: (1, 11),
                }],
                is_self_closing: false,
                syntax: TagSyntax::Django,
                start_index: 0,
                end_index: 64,
                line_col: (1, 4),
            }
        );
    }

    #[test]
    fn test_template_string_invalid() {
        // Test incomplete template tags (should not be marked as template_string)
        let inputs = vec![
            r#"{% my_tag "Hello {{ first_name" %}"#,
            r#"{% my_tag "Hello {% first_name" %}"#,
            r#"{% my_tag "Hello {# first_name" %}"#,
            r#"{% my_tag "Hello {{ first_name %}" %}"#,
            r#"{% my_tag "Hello first_name }}" %}"#,
            r#"{% my_tag "Hello }} first_name {{" %}"#,
        ];
        for input in inputs {
            let result = TagParser::parse_tag(input, &HashSet::new()).unwrap();
            // Extract the string value from the input (between quotes)
            let string_start = input.find('"').unwrap() + 1;
            let string_end = input.rfind('"').unwrap();
            let string_value = &input[string_start..string_end];

            assert_eq!(
                result,
                Tag {
                    name: TagToken {
                        token: "my_tag".to_string(),
                        start_index: 3,
                        end_index: 9,
                        line_col: (1, 4),
                    },
                    attrs: vec![TagAttr {
                        key: None,
                        value: TagValue {
                            token: TagToken {
                                token: format!("\"{}\"", string_value),
                                start_index: 10,
                                end_index: 10 + string_value.len() + 2,
                                line_col: (1, 11),
                            },
                            children: vec![],
                            spread: None,
                            filters: vec![],
                            kind: ValueKind::String,
                            start_index: 10,
                            end_index: 10 + string_value.len() + 2,
                            line_col: (1, 11),
                        },
                        is_flag: false,
                        start_index: 10,
                        end_index: 10 + string_value.len() + 2,
                        line_col: (1, 11),
                    }],
                    is_self_closing: false,
                    syntax: TagSyntax::Django,
                    start_index: 0,
                    end_index: input.len(),
                    line_col: (1, 4),
                }
            );
        }
    }

    #[test]
    fn test_template_string_filter_arg() {
        // Test that template strings are detected in filter args
        let input = "{% my_tag value|default:\"{{ var }}\" %}";
        let result = TagParser::parse_tag(input, &HashSet::new()).unwrap();
        assert_eq!(
            result,
            Tag {
                name: TagToken {
                    token: "my_tag".to_string(),
                    start_index: 3,
                    end_index: 9,
                    line_col: (1, 4),
                },
                attrs: vec![TagAttr {
                    key: None,
                    value: TagValue {
                        token: TagToken {
                            token: "value".to_string(),
                            start_index: 10,
                            end_index: 15,
                            line_col: (1, 11),
                        },
                        children: vec![],
                        spread: None,
                        filters: vec![TagValueFilter {
                            token: TagToken {
                                token: "default".to_string(),
                                start_index: 16,
                                end_index: 23,
                                line_col: (1, 17),
                            },
                            arg: Some(TagValue {
                                token: TagToken {
                                    token: "\"{{ var }}\"".to_string(),
                                    start_index: 24,
                                    end_index: 35,
                                    line_col: (1, 25),
                                },
                                children: vec![],
                                spread: None,
                                filters: vec![],
                                kind: ValueKind::TemplateString,
                                start_index: 23,
                                end_index: 35,
                                line_col: (1, 24),
                            }),
                            start_index: 15,
                            end_index: 35,
                            line_col: (1, 16),
                        }],
                        kind: ValueKind::Variable,
                        start_index: 10,
                        end_index: 35,
                        line_col: (1, 11),
                    },
                    is_flag: false,
                    start_index: 10,
                    end_index: 35,
                    line_col: (1, 11),
                }],
                is_self_closing: false,
                syntax: TagSyntax::Django,
                start_index: 0,
                end_index: 38,
                line_col: (1, 4),
            }
        );
    }

    #[test]
    fn test_template_string_i18n() {
        // Test that template strings are not detected in i18n strings
        let input = "{% my_tag _(\"{{ var }}\") %}";
        let result = TagParser::parse_tag(input, &HashSet::new()).unwrap();
        assert_eq!(
            result,
            Tag {
                name: TagToken {
                    token: "my_tag".to_string(),
                    start_index: 3,
                    end_index: 9,
                    line_col: (1, 4),
                },
                attrs: vec![TagAttr {
                    key: None,
                    value: TagValue {
                        token: TagToken {
                            token: "_(\"{{ var }}\")".to_string(),
                            start_index: 10,
                            end_index: 24,
                            line_col: (1, 11),
                        },
                        children: vec![],
                        spread: None,
                        filters: vec![],
                        kind: ValueKind::Translation,
                        start_index: 10,
                        end_index: 24,
                        line_col: (1, 11),
                    },
                    is_flag: false,
                    start_index: 10,
                    end_index: 24,
                    line_col: (1, 11),
                }],
                is_self_closing: false,
                syntax: TagSyntax::Django,
                start_index: 0,
                end_index: 27,
                line_col: (1, 4),
            }
        );
    }

    #[test]
    fn test_dict_filters_key() {
        // Test filters on keys
        let input = r#"{% my_tag {"key"|upper|lower: "value"} %}"#;
        let result = TagParser::parse_tag(input, &HashSet::new()).unwrap();

        assert_eq!(
            result,
            Tag {
                name: TagToken {
                    token: "my_tag".to_string(),
                    start_index: 3,
                    end_index: 9,
                    line_col: (1, 4),
                },
                attrs: vec![TagAttr {
                    key: None,
                    value: TagValue {
                        token: TagToken {
                            token: r#"{"key"|upper|lower: "value"}"#.to_string(),
                            start_index: 10,
                            end_index: 38,
                            line_col: (1, 11),
                        },
                        children: vec![
                            TagValue {
                                token: TagToken {
                                    token: "\"key\"".to_string(),
                                    start_index: 11,
                                    end_index: 16,
                                    line_col: (1, 12),
                                },
                                children: vec![],
                                spread: None,
                                filters: vec![
                                    TagValueFilter {
                                        arg: None,
                                        token: TagToken {
                                            token: "upper".to_string(),
                                            start_index: 17,
                                            end_index: 22,
                                            line_col: (1, 18),
                                        },
                                        start_index: 16,
                                        end_index: 22,
                                        line_col: (1, 17),
                                    },
                                    TagValueFilter {
                                        arg: None,
                                        token: TagToken {
                                            token: "lower".to_string(),
                                            start_index: 23,
                                            end_index: 28,
                                            line_col: (1, 24),
                                        },
                                        start_index: 22,
                                        end_index: 28,
                                        line_col: (1, 23),
                                    },
                                ],
                                kind: ValueKind::String,
                                start_index: 11,
                                end_index: 28,
                                line_col: (1, 12),
                            },
                            TagValue {
                                token: TagToken {
                                    token: "\"value\"".to_string(),
                                    start_index: 30,
                                    end_index: 37,
                                    line_col: (1, 31),
                                },
                                children: vec![],
                                spread: None,
                                filters: vec![],
                                kind: ValueKind::String,
                                start_index: 30,
                                end_index: 37,
                                line_col: (1, 31),
                            },
                        ],
                        spread: None,
                        filters: vec![],
                        kind: ValueKind::Dict,
                        start_index: 10,
                        end_index: 38,
                        line_col: (1, 11),
                    },
                    is_flag: false,
                    start_index: 10,
                    end_index: 38,
                    line_col: (1, 11),
                }],
                is_self_closing: false,
                syntax: TagSyntax::Django,
                start_index: 0,
                end_index: 41,
                line_col: (1, 4),
            }
        );
    }

    #[test]
    fn test_dict_filters_value() {
        // Test filters on values
        let input = r#"{% my_tag {"key": "value"|upper|lower} %}"#;
        let result = TagParser::parse_tag(input, &HashSet::new()).unwrap();

        assert_eq!(
            result,
            Tag {
                name: TagToken {
                    token: "my_tag".to_string(),
                    start_index: 3,
                    end_index: 9,
                    line_col: (1, 4),
                },
                attrs: vec![TagAttr {
                    key: None,
                    value: TagValue {
                        token: TagToken {
                            token: r#"{"key": "value"|upper|lower}"#.to_string(),
                            start_index: 10,
                            end_index: 38,
                            line_col: (1, 11),
                        },
                        children: vec![
                            TagValue {
                                token: TagToken {
                                    token: "\"key\"".to_string(),
                                    start_index: 11,
                                    end_index: 16,
                                    line_col: (1, 12),
                                },
                                children: vec![],
                                spread: None,
                                filters: vec![],
                                kind: ValueKind::String,
                                start_index: 11,
                                end_index: 16,
                                line_col: (1, 12),
                            },
                            TagValue {
                                token: TagToken {
                                    token: "\"value\"".to_string(),
                                    start_index: 18,
                                    end_index: 25,
                                    line_col: (1, 19),
                                },
                                children: vec![],
                                spread: None,
                                filters: vec![
                                    TagValueFilter {
                                        arg: None,
                                        token: TagToken {
                                            token: "upper".to_string(),
                                            start_index: 26,
                                            end_index: 31,
                                            line_col: (1, 27),
                                        },
                                        start_index: 25,
                                        end_index: 31,
                                        line_col: (1, 26),
                                    },
                                    TagValueFilter {
                                        arg: None,
                                        token: TagToken {
                                            token: "lower".to_string(),
                                            start_index: 32,
                                            end_index: 37,
                                            line_col: (1, 33),
                                        },
                                        start_index: 31,
                                        end_index: 37,
                                        line_col: (1, 32),
                                    },
                                ],
                                kind: ValueKind::String,
                                start_index: 18,
                                end_index: 37,
                                line_col: (1, 19),
                            },
                        ],
                        spread: None,
                        filters: vec![],
                        kind: ValueKind::Dict,
                        start_index: 10,
                        end_index: 38,
                        line_col: (1, 11),
                    },
                    is_flag: false,
                    start_index: 10,
                    end_index: 38,
                    line_col: (1, 11),
                }],
                is_self_closing: false,
                syntax: TagSyntax::Django,
                start_index: 0,
                end_index: 41,
                line_col: (1, 4),
            }
        );
    }

    #[test]
    fn test_dict_filters() {
        // Test filter on entire dict
        let input = r#"{% my_tag {"key": "value"}|default:empty_dict %}"#;
        let result = TagParser::parse_tag(input, &HashSet::new()).unwrap();
        assert_eq!(
            result,
            Tag {
                name: TagToken {
                    token: "my_tag".to_string(),
                    start_index: 3,
                    end_index: 9,
                    line_col: (1, 4),
                },
                attrs: vec![TagAttr {
                    key: None,
                    value: TagValue {
                        token: TagToken {
                            token: r#"{"key": "value"}"#.to_string(),
                            start_index: 10,
                            end_index: 26,
                            line_col: (1, 11),
                        },
                        children: vec![
                            TagValue {
                                token: TagToken {
                                    token: "\"key\"".to_string(),
                                    start_index: 11,
                                    end_index: 16,
                                    line_col: (1, 12),
                                },
                                children: vec![],
                                spread: None,
                                filters: vec![],
                                kind: ValueKind::String,
                                start_index: 11,
                                end_index: 16,
                                line_col: (1, 12),
                            },
                            TagValue {
                                token: TagToken {
                                    token: "\"value\"".to_string(),
                                    start_index: 18,
                                    end_index: 25,
                                    line_col: (1, 19),
                                },
                                children: vec![],
                                spread: None,
                                filters: vec![],
                                kind: ValueKind::String,
                                start_index: 18,
                                end_index: 25,
                                line_col: (1, 19),
                            },
                        ],
                        spread: None,
                        filters: vec![TagValueFilter {
                            token: TagToken {
                                token: "default".to_string(),
                                start_index: 27,
                                end_index: 34,
                                line_col: (1, 28),
                            },
                            arg: Some(TagValue {
                                token: TagToken {
                                    token: "empty_dict".to_string(),
                                    start_index: 35,
                                    end_index: 45,
                                    line_col: (1, 36),
                                },
                                children: vec![],
                                spread: None,
                                filters: vec![],
                                kind: ValueKind::Variable,
                                start_index: 34,
                                end_index: 45,
                                line_col: (1, 35),
                            }),
                            start_index: 26,
                            end_index: 45,
                            line_col: (1, 27),
                        }],
                        kind: ValueKind::Dict,
                        start_index: 10,
                        end_index: 45,
                        line_col: (1, 11),
                    },
                    is_flag: false,
                    start_index: 10,
                    end_index: 45,
                    line_col: (1, 11),
                }],
                is_self_closing: false,
                syntax: TagSyntax::Django,
                start_index: 0,
                end_index: 48,
                line_col: (1, 4),
            }
        );
    }

    #[test]
    fn test_dict_filters_all() {
        // Test filter on all dict
        let input = r#"{% my_tag {"key" | default: "value" | default : empty_dict} | default : empty_dict %}"#;
        let result = TagParser::parse_tag(input, &HashSet::new()).unwrap();
        assert_eq!(
            result,
            Tag {
                name: TagToken {
                    token: "my_tag".to_string(),
                    start_index: 3,
                    end_index: 9,
                    line_col: (1, 4),
                },
                attrs: vec![TagAttr {
                    key: None,
                    value: TagValue {
                        token: TagToken {
                            token: r#"{"key" | default: "value" | default : empty_dict}"#
                                .to_string(),
                            start_index: 10,
                            end_index: 59,
                            line_col: (1, 11),
                        },
                        children: vec![
                            TagValue {
                                token: TagToken {
                                    token: "\"key\"".to_string(),
                                    start_index: 11,
                                    end_index: 16,
                                    line_col: (1, 12),
                                },
                                children: vec![],
                                spread: None,
                                filters: vec![TagValueFilter {
                                    arg: None,
                                    token: TagToken {
                                        token: "default".to_string(),
                                        start_index: 19,
                                        end_index: 26,
                                        line_col: (1, 20),
                                    },
                                    start_index: 17,
                                    end_index: 26,
                                    line_col: (1, 18),
                                }],
                                kind: ValueKind::String,
                                start_index: 11,
                                end_index: 26,
                                line_col: (1, 12),
                            },
                            TagValue {
                                token: TagToken {
                                    token: "\"value\"".to_string(),
                                    start_index: 28,
                                    end_index: 35,
                                    line_col: (1, 29),
                                },
                                children: vec![],
                                spread: None,
                                filters: vec![TagValueFilter {
                                    arg: Some(TagValue {
                                        token: TagToken {
                                            token: "empty_dict".to_string(),
                                            start_index: 48,
                                            end_index: 58,
                                            line_col: (1, 49),
                                        },
                                        children: vec![],
                                        spread: None,
                                        filters: vec![],
                                        kind: ValueKind::Variable,
                                        start_index: 45,
                                        end_index: 58,
                                        line_col: (1, 46),
                                    }),
                                    token: TagToken {
                                        token: "default".to_string(),
                                        start_index: 38,
                                        end_index: 45,
                                        line_col: (1, 39),
                                    },
                                    start_index: 36,
                                    end_index: 58,
                                    line_col: (1, 37),
                                }],
                                kind: ValueKind::String,
                                start_index: 28,
                                end_index: 58,
                                line_col: (1, 29),
                            },
                        ],
                        spread: None,
                        filters: vec![TagValueFilter {
                            arg: Some(TagValue {
                                token: TagToken {
                                    token: "empty_dict".to_string(),
                                    start_index: 72,
                                    end_index: 82,
                                    line_col: (1, 73),
                                },
                                children: vec![],
                                spread: None,
                                filters: vec![],
                                kind: ValueKind::Variable,
                                start_index: 69,
                                end_index: 82,
                                line_col: (1, 70),
                            }),
                            token: TagToken {
                                token: "default".to_string(),
                                start_index: 62,
                                end_index: 69,
                                line_col: (1, 63),
                            },
                            start_index: 60,
                            end_index: 82,
                            line_col: (1, 61),
                        }],
                        kind: ValueKind::Dict,
                        start_index: 10,
                        end_index: 82,
                        line_col: (1, 11),
                    },
                    is_flag: false,
                    start_index: 10,
                    end_index: 82,
                    line_col: (1, 11),
                }],
                is_self_closing: false,
                syntax: TagSyntax::Django,
                start_index: 0,
                end_index: 85,
                line_col: (1, 4),
            }
        );
    }

    #[test]
    fn test_dict_nested() {
        // Test dict in list
        let input = "{% my_tag [1, {\"key\": \"val\"}, 2] %}";
        let result = TagParser::parse_tag(input, &HashSet::new()).unwrap();
        assert_eq!(
            result,
            Tag {
                name: TagToken {
                    token: "my_tag".to_string(),
                    start_index: 3,
                    end_index: 9,
                    line_col: (1, 4),
                },
                attrs: vec![TagAttr {
                    key: None,
                    value: TagValue {
                        token: TagToken {
                            token: r#"[1, {"key": "val"}, 2]"#.to_string(),
                            start_index: 10,
                            end_index: 32,
                            line_col: (1, 11),
                        },
                        children: vec![
                            TagValue {
                                token: TagToken {
                                    token: "1".to_string(),
                                    start_index: 11,
                                    end_index: 12,
                                    line_col: (1, 12),
                                },
                                children: vec![],
                                spread: None,
                                filters: vec![],
                                kind: ValueKind::Int,
                                start_index: 11,
                                end_index: 12,
                                line_col: (1, 12),
                            },
                            TagValue {
                                token: TagToken {
                                    token: r#"{"key": "val"}"#.to_string(),
                                    start_index: 14,
                                    end_index: 28,
                                    line_col: (1, 15),
                                },
                                children: vec![
                                    TagValue {
                                        token: TagToken {
                                            token: "\"key\"".to_string(),
                                            start_index: 15,
                                            end_index: 20,
                                            line_col: (1, 16),
                                        },
                                        children: vec![],
                                        spread: None,
                                        filters: vec![],
                                        kind: ValueKind::String,
                                        start_index: 15,
                                        end_index: 20,
                                        line_col: (1, 16),
                                    },
                                    TagValue {
                                        token: TagToken {
                                            token: "\"val\"".to_string(),
                                            start_index: 22,
                                            end_index: 27,
                                            line_col: (1, 23),
                                        },
                                        children: vec![],
                                        spread: None,
                                        filters: vec![],
                                        kind: ValueKind::String,
                                        start_index: 22,
                                        end_index: 27,
                                        line_col: (1, 23),
                                    },
                                ],
                                spread: None,
                                filters: vec![],
                                kind: ValueKind::Dict,
                                start_index: 14,
                                end_index: 28,
                                line_col: (1, 15),
                            },
                            TagValue {
                                token: TagToken {
                                    token: "2".to_string(),
                                    start_index: 30,
                                    end_index: 31,
                                    line_col: (1, 31),
                                },
                                children: vec![],
                                spread: None,
                                filters: vec![],
                                kind: ValueKind::Int,
                                start_index: 30,
                                end_index: 31,
                                line_col: (1, 31),
                            },
                        ],
                        spread: None,
                        filters: vec![],
                        kind: ValueKind::List,
                        start_index: 10,
                        end_index: 32,
                        line_col: (1, 11),
                    },
                    is_flag: false,
                    start_index: 10,
                    end_index: 32,
                    line_col: (1, 11),
                }],
                is_self_closing: false,
                syntax: TagSyntax::Django,
                start_index: 0,
                end_index: 35,
                line_col: (1, 4),
            }
        );
    }

    #[test]
    fn test_dict_nested_list() {
        // Test list in dict
        let input = r#"{% my_tag {"key": [1, 2, 3]} %}"#;
        let result = TagParser::parse_tag(input, &HashSet::new()).unwrap();
        assert_eq!(
            result,
            Tag {
                name: TagToken {
                    token: "my_tag".to_string(),
                    start_index: 3,
                    end_index: 9,
                    line_col: (1, 4),
                },
                attrs: vec![TagAttr {
                    key: None,
                    value: TagValue {
                        token: TagToken {
                            token: r#"{"key": [1, 2, 3]}"#.to_string(),
                            start_index: 10,
                            end_index: 28,
                            line_col: (1, 11),
                        },
                        children: vec![
                            TagValue {
                                token: TagToken {
                                    token: "\"key\"".to_string(),
                                    start_index: 11,
                                    end_index: 16,
                                    line_col: (1, 12),
                                },
                                children: vec![],
                                spread: None,
                                filters: vec![],
                                kind: ValueKind::String,
                                start_index: 11,
                                end_index: 16,
                                line_col: (1, 12),
                            },
                            TagValue {
                                token: TagToken {
                                    token: r#"[1, 2, 3]"#.to_string(),
                                    start_index: 18,
                                    end_index: 27,
                                    line_col: (1, 19),
                                },
                                children: vec![
                                    TagValue {
                                        token: TagToken {
                                            token: "1".to_string(),
                                            start_index: 19,
                                            end_index: 20,
                                            line_col: (1, 20),
                                        },
                                        children: vec![],
                                        spread: None,
                                        filters: vec![],
                                        kind: ValueKind::Int,
                                        start_index: 19,
                                        end_index: 20,
                                        line_col: (1, 20),
                                    },
                                    TagValue {
                                        token: TagToken {
                                            token: "2".to_string(),
                                            start_index: 22,
                                            end_index: 23,
                                            line_col: (1, 23),
                                        },
                                        children: vec![],
                                        spread: None,
                                        filters: vec![],
                                        kind: ValueKind::Int,
                                        start_index: 22,
                                        end_index: 23,
                                        line_col: (1, 23),
                                    },
                                    TagValue {
                                        token: TagToken {
                                            token: "3".to_string(),
                                            start_index: 25,
                                            end_index: 26,
                                            line_col: (1, 26),
                                        },
                                        children: vec![],
                                        spread: None,
                                        filters: vec![],
                                        kind: ValueKind::Int,
                                        start_index: 25,
                                        end_index: 26,
                                        line_col: (1, 26),
                                    },
                                ],
                                spread: None,
                                filters: vec![],
                                kind: ValueKind::List,
                                start_index: 18,
                                end_index: 27,
                                line_col: (1, 19),
                            },
                        ],
                        spread: None,
                        filters: vec![],
                        kind: ValueKind::Dict,
                        start_index: 10,
                        end_index: 28,
                        line_col: (1, 11),
                    },
                    is_flag: false,
                    start_index: 10,
                    end_index: 28,
                    line_col: (1, 11),
                }],
                is_self_closing: false,
                syntax: TagSyntax::Django,
                start_index: 0,
                end_index: 31,
                line_col: (1, 4),
            }
        );
    }

    #[test]
    fn test_dict_invalid() {
        let invalid_inputs = vec![
            (
                r#"{% my_tag {key|lower:my_arg: 123} %}"#,
                "filter arguments in dictionary keys",
            ),
            (
                r#"{% my_tag {"key"|default:empty_dict: "value"|default:empty_dict} %}"#,
                "filter arguments in dictionary keys",
            ),
            ("{% my_tag {key} %}", "missing value"),
            ("{% my_tag {key,} %}", "missing value with comma"),
            ("{% my_tag {key:} %}", "missing value after colon"),
            ("{% my_tag {:value} %}", "missing key"),
            ("{% my_tag {key: key:} %}", "double colon"),
            ("{% my_tag {:key :key} %}", "double key"),
        ];

        for (input, msg) in invalid_inputs {
            assert!(
                TagParser::parse_tag(input, &HashSet::new()).is_err(),
                "Should not allow {}: {}",
                msg,
                input
            );
        }
    }

    #[test]
    fn test_dict_key_types() {
        // Test string literal key
        let input = r#"{% my_tag {"key": "value"} %}"#;
        assert!(TagParser::parse_tag(input, &HashSet::new()).is_ok());

        // Test variable key
        let input = r#"{% my_tag {my_var: "value"} %}"#;
        assert!(TagParser::parse_tag(input, &HashSet::new()).is_ok());

        // Test i18n string key
        let input = r#"{% my_tag {_("hello"): "value"} %}"#;
        assert!(TagParser::parse_tag(input, &HashSet::new()).is_ok());

        // Test number key
        let input = r#"{% my_tag {42: "value"} %}"#;
        assert!(TagParser::parse_tag(input, &HashSet::new()).is_ok());

        // Test filtered key
        let input = r#"{% my_tag {"key"|upper: "value"} %}"#;
        assert!(TagParser::parse_tag(input, &HashSet::new()).is_ok());

        // Test list as key (should fail)
        let input = r#"{% my_tag {[1, 2]: "value"} %}"#;
        assert!(
            TagParser::parse_tag(input, &HashSet::new()).is_err(),
            "Should not allow list as dictionary key"
        );

        // Test dict as key (should fail)
        let input = r#"{% my_tag {"nested": "dict"}: "value" %}"#;
        assert!(
            TagParser::parse_tag(input, &HashSet::new()).is_err(),
            "Should not allow dictionary as dictionary key"
        );
    }
    #[test]
    fn test_dict_value_types() {
        // Test string literal value
        let input = r#"{% my_tag {"key": "value"} %}"#;
        assert!(TagParser::parse_tag(input, &HashSet::new()).is_ok());

        // Test variable value
        let input = r#"{% my_tag {"key": my_var} %}"#;
        assert!(TagParser::parse_tag(input, &HashSet::new()).is_ok());
        // Test number value
        let input = r#"{% my_tag {"key": 42} %}"#;
        assert!(TagParser::parse_tag(input, &HashSet::new()).is_ok());

        // Test list value
        let input = r#"{% my_tag {"key": [1, 2, 3]} %}"#;
        assert!(TagParser::parse_tag(input, &HashSet::new()).is_ok());

        // Test dict value
        let input = r#"{% my_tag {"key": {"nested": "dict"}} %}"#;
        assert!(TagParser::parse_tag(input, &HashSet::new()).is_ok());

        // Test filtered value
        let input = r#"{% my_tag {"key": "value"|upper} %}"#;
        assert!(TagParser::parse_tag(input, &HashSet::new()).is_ok());

        // Test spread value
        let input = r#"{% my_tag {"key1": "val1", **other_dict} %}"#;
        assert!(TagParser::parse_tag(input, &HashSet::new()).is_ok());

        // Test spread with filter that might return dict
        let input = r#"{% my_tag {"key1": "val1", **42|make_dict} %}"#;
        assert!(TagParser::parse_tag(input, &HashSet::new()).is_ok());
    }

    #[test]
    fn test_dict_spread() {
        // Test spreading into dict
        let input = r#"{% my_tag {"key1": "val1", **other_dict, "key2": "val2", **"{{ key3 }}", **_( " key4 ")} %}"#;
        let result = TagParser::parse_tag(input, &HashSet::new()).unwrap();
        assert_eq!(
            result,
            Tag {
                name: TagToken {
                    token: "my_tag".to_string(),
                    start_index: 3,
                    end_index: 9,
                    line_col: (1, 4),
                },
                attrs: vec![TagAttr {
                    key: None,
                    value: TagValue {
                        token: TagToken {
                            token: r#"{"key1": "val1", **other_dict, "key2": "val2", **"{{ key3 }}", **_( " key4 ")}"#.to_string(),
                            start_index: 10,
                            end_index: 88,
                            line_col: (1, 11),
                        },
                        children: vec![
                            TagValue {
                                token: TagToken {
                                    token: "\"key1\"".to_string(),
                                    start_index: 11,
                                    end_index: 17,
                                    line_col: (1, 12),
                                },
                                children: vec![],
                                spread: None,
                                filters: vec![],
                                kind: ValueKind::String,
                                start_index: 11,
                                end_index: 17,
                                line_col: (1, 12),
                            },
                            TagValue {
                                token: TagToken {
                                    token: "\"val1\"".to_string(),
                                    start_index: 19,
                                    end_index: 25,
                                    line_col: (1, 20),
                                },
                                children: vec![],
                                spread: None,
                                filters: vec![],
                                kind: ValueKind::String,
                                start_index: 19,
                                end_index: 25,
                                line_col: (1, 20),
                            },
                            TagValue {
                                token: TagToken {
                                    token: "other_dict".to_string(),
                                    start_index: 29,
                                    end_index: 39,
                                    line_col: (1, 30),
                                },
                                children: vec![],
                                spread: Some("**".to_string()),
                                filters: vec![],
                                kind: ValueKind::Variable,
                                start_index: 27,
                                end_index: 39,
                                line_col: (1, 28),
                            },
                            TagValue {
                                token: TagToken {
                                    token: "\"key2\"".to_string(),
                                    start_index: 41,
                                    end_index: 47,
                                    line_col: (1, 42),
                                },
                                children: vec![],
                                spread: None,
                                filters: vec![],
                                kind: ValueKind::String,
                                start_index: 41,
                                end_index: 47,
                                line_col: (1, 42),
                            },
                            TagValue {
                                token: TagToken {
                                    token: "\"val2\"".to_string(),
                                    start_index: 49,
                                    end_index: 55,
                                    line_col: (1, 50),
                                },
                                children: vec![],
                                spread: None,
                                filters: vec![],
                                kind: ValueKind::String,
                                start_index: 49,
                                end_index: 55,
                                line_col: (1, 50),
                            },
                            TagValue {
                                token: TagToken {
                                    token: "\"{{ key3 }}\"".to_string(),
                                    start_index: 59,
                                    end_index: 71,
                                    line_col: (1, 60),
                                },
                                children: vec![],
                                spread: Some("**".to_string()),
                                filters: vec![],
                                kind: ValueKind::TemplateString,
                                start_index: 57,
                                end_index: 71,
                                line_col: (1, 58),
                            },
                            TagValue {
                                token: TagToken {
                                    token: "_(\" key4 \")".to_string(),
                                    start_index: 75,
                                    end_index: 87,
                                    line_col: (1, 76),
                                },
                                children: vec![],
                                spread: Some("**".to_string()),
                                filters: vec![],
                                kind: ValueKind::Translation,
                                start_index: 73,
                                end_index: 87,
                                line_col: (1, 74),
                            },
                        ],
                        spread: None,
                        filters: vec![],
                        kind: ValueKind::Dict,
                        start_index: 10,
                        end_index: 88,
                        line_col: (1, 11),
                    },
                    is_flag: false,
                    start_index: 10,
                    end_index: 88,
                    line_col: (1, 11),
                }],
                is_self_closing: false,
                syntax: TagSyntax::Django,
                start_index: 0,
                end_index: 91,
                line_col: (1, 4),
            }
        );
    }

    #[test]
    fn test_dict_spread_filters() {
        // Test spreading into dict + filters
        let input = r#"{% my_tag {"key1": "val1", **other_dict, "key2": "val2", **"{{ key3 }}", **_( " key4 ")} %}"#;
        let result = TagParser::parse_tag(input, &HashSet::new()).unwrap();
        assert_eq!(
            result,
            Tag {
                name: TagToken {
                    token: "my_tag".to_string(),
                    start_index: 3,
                    end_index: 9,
                    line_col: (1, 4),
                },
                attrs: vec![TagAttr {
                    key: None,
                    value: TagValue {
                        token: TagToken {
                            token: r#"{"key1": "val1", **other_dict, "key2": "val2", **"{{ key3 }}", **_( " key4 ")}"#.to_string(),
                            start_index: 10,
                            end_index: 88,
                            line_col: (1, 11),
                        },
                        children: vec![
                            TagValue {
                                token: TagToken {
                                    token: "\"key1\"".to_string(),
                                    start_index: 11,
                                    end_index: 17,
                                    line_col: (1, 12),
                                },
                                children: vec![],
                                spread: None,
                                filters: vec![],
                                kind: ValueKind::String,
                                start_index: 11,
                                end_index: 17,
                                line_col: (1, 12),
                            },
                            TagValue {
                                token: TagToken {
                                    token: "\"val1\"".to_string(),
                                    start_index: 19,
                                    end_index: 25,
                                    line_col: (1, 20),
                                },
                                children: vec![],
                                spread: None,
                                filters: vec![],
                                kind: ValueKind::String,
                                start_index: 19,
                                end_index: 25,
                                line_col: (1, 20),
                            },
                            TagValue {
                                token: TagToken {
                                    token: "other_dict".to_string(),
                                    start_index: 29,
                                    end_index: 39,
                                    line_col: (1, 30),
                                },
                                children: vec![],
                                spread: Some("**".to_string()),
                                filters: vec![],
                                kind: ValueKind::Variable,
                                start_index: 27,
                                end_index: 39,
                                line_col: (1, 28),
                            },
                            TagValue {
                                token: TagToken {
                                    token: "\"key2\"".to_string(),
                                    start_index: 41,
                                    end_index: 47,
                                    line_col: (1, 42),
                                },
                                children: vec![],
                                spread: None,
                                filters: vec![],
                                kind: ValueKind::String,
                                start_index: 41,
                                end_index: 47,
                                line_col: (1, 42),
                            },
                            TagValue {
                                token: TagToken {
                                    token: "\"val2\"".to_string(),
                                    start_index: 49,
                                    end_index: 55,
                                    line_col: (1, 50),
                                },
                                children: vec![],
                                spread: None,
                                filters: vec![],
                                kind: ValueKind::String,
                                start_index: 49,
                                end_index: 55,
                                line_col: (1, 50),
                            },
                            TagValue {
                                token: TagToken {
                                    token: "\"{{ key3 }}\"".to_string(),
                                    start_index: 59,
                                    end_index: 71,
                                    line_col: (1, 60),
                                },
                                children: vec![],
                                spread: Some("**".to_string()),
                                filters: vec![],
                                kind: ValueKind::TemplateString,
                                start_index: 57,
                                end_index: 71,
                                line_col: (1, 58),
                            },
                            TagValue {
                                token: TagToken {
                                    token: "_(\" key4 \")".to_string(),
                                    start_index: 75,
                                    end_index: 87,
                                    line_col: (1, 76),
                                },
                                children: vec![],
                                spread: Some("**".to_string()),
                                filters: vec![],
                                kind: ValueKind::Translation,
                                start_index: 73,
                                end_index: 87,
                                line_col: (1, 74),
                            },
                        ],
                        spread: None,
                        filters: vec![],
                        kind: ValueKind::Dict,
                        start_index: 10,
                        end_index: 88,
                        line_col: (1, 11),
                    },
                    is_flag: false,
                    start_index: 10,
                    end_index: 88,
                    line_col: (1, 11),
                }],
                is_self_closing: false,
                syntax: TagSyntax::Django,
                start_index: 0,
                end_index: 91,
                line_col: (1, 4),
            }
        );
    }

    #[test]
    fn test_dict_spread_dict() {
        // Test spreading literal dict
        let input = r#"{% my_tag {"key1": "val1", **{"inner": "value"}, "key2": "val2"} %}"#;
        let result = TagParser::parse_tag(input, &HashSet::new()).unwrap();
        assert_eq!(
            result,
            Tag {
                name: TagToken {
                    token: "my_tag".to_string(),
                    start_index: 3,
                    end_index: 9,
                    line_col: (1, 4),
                },
                attrs: vec![TagAttr {
                    key: None,
                    value: TagValue {
                        token: TagToken {
                            token: r#"{"key1": "val1", **{"inner": "value"}, "key2": "val2"}"#
                                .to_string(),
                            start_index: 10,
                            end_index: 64,
                            line_col: (1, 11),
                        },
                        children: vec![
                            TagValue {
                                token: TagToken {
                                    token: "\"key1\"".to_string(),
                                    start_index: 11,
                                    end_index: 17,
                                    line_col: (1, 12),
                                },
                                children: vec![],
                                spread: None,
                                filters: vec![],
                                kind: ValueKind::String,
                                start_index: 11,
                                end_index: 17,
                                line_col: (1, 12),
                            },
                            TagValue {
                                token: TagToken {
                                    token: "\"val1\"".to_string(),
                                    start_index: 19,
                                    end_index: 25,
                                    line_col: (1, 20),
                                },
                                children: vec![],
                                spread: None,
                                filters: vec![],
                                kind: ValueKind::String,
                                start_index: 19,
                                end_index: 25,
                                line_col: (1, 20),
                            },
                            TagValue {
                                token: TagToken {
                                    token: r#"{"inner": "value"}"#.to_string(),
                                    start_index: 29,
                                    end_index: 47,
                                    line_col: (1, 30),
                                },
                                children: vec![
                                    TagValue {
                                        token: TagToken {
                                            token: "\"inner\"".to_string(),
                                            start_index: 30,
                                            end_index: 37,
                                            line_col: (1, 31),
                                        },
                                        children: vec![],
                                        spread: None,
                                        filters: vec![],
                                        kind: ValueKind::String,
                                        start_index: 30,
                                        end_index: 37,
                                        line_col: (1, 31),
                                    },
                                    TagValue {
                                        token: TagToken {
                                            token: "\"value\"".to_string(),
                                            start_index: 39,
                                            end_index: 46,
                                            line_col: (1, 40),
                                        },
                                        children: vec![],
                                        spread: None,
                                        filters: vec![],
                                        kind: ValueKind::String,
                                        start_index: 39,
                                        end_index: 46,
                                        line_col: (1, 40),
                                    },
                                ],
                                spread: Some("**".to_string()),
                                filters: vec![],
                                kind: ValueKind::Dict,
                                start_index: 27,
                                end_index: 47,
                                line_col: (1, 28),
                            },
                            TagValue {
                                token: TagToken {
                                    token: "\"key2\"".to_string(),
                                    start_index: 49,
                                    end_index: 55,
                                    line_col: (1, 50),
                                },
                                children: vec![],
                                spread: None,
                                filters: vec![],
                                kind: ValueKind::String,
                                start_index: 49,
                                end_index: 55,
                                line_col: (1, 50),
                            },
                            TagValue {
                                token: TagToken {
                                    token: "\"val2\"".to_string(),
                                    start_index: 57,
                                    end_index: 63,
                                    line_col: (1, 58),
                                },
                                children: vec![],
                                spread: None,
                                filters: vec![],
                                kind: ValueKind::String,
                                start_index: 57,
                                end_index: 63,
                                line_col: (1, 58),
                            },
                        ],
                        spread: None,
                        filters: vec![],
                        kind: ValueKind::Dict,
                        start_index: 10,
                        end_index: 64,
                        line_col: (1, 11),
                    },
                    is_flag: false,
                    start_index: 10,
                    end_index: 64,
                    line_col: (1, 11),
                }],
                is_self_closing: false,
                syntax: TagSyntax::Django,
                start_index: 0,
                end_index: 67,
                line_col: (1, 4),
            }
        );
    }

    #[test]
    fn test_dict_with_comments() {
        // Test comments after values
        let input = r#"{% my_tag {# comment before dict #}{{# comment after dict start #}
            "key1": "value1", {# comment after first value #}
            "key2": "value2"
        {# comment before dict end #}}{# comment after dict #} %}"#;
        let result = TagParser::parse_tag(input, &HashSet::new()).unwrap();
        assert_eq!(
            result,
            Tag {
                name: TagToken {
                    token: "my_tag".to_string(),
                    start_index: 3,
                    end_index: 9,
                    line_col: (1, 4),
                },
                attrs: vec![TagAttr {
                    key: None,
                    value: TagValue {
                        token: TagToken {
                            token: r#"{{# comment after dict start #}
            "key1": "value1", {# comment after first value #}
            "key2": "value2"
        {# comment before dict end #}}"#
                                .to_string(),
                            start_index: 35,
                            end_index: 196,
                            line_col: (1, 36),
                        },
                        children: vec![
                            TagValue {
                                token: TagToken {
                                    token: "\"key1\"".to_string(),
                                    start_index: 79,
                                    end_index: 85,
                                    line_col: (2, 13),
                                },
                                children: vec![],
                                spread: None,
                                filters: vec![],
                                kind: ValueKind::String,
                                start_index: 79,
                                end_index: 85,
                                line_col: (2, 13),
                            },
                            TagValue {
                                token: TagToken {
                                    token: "\"value1\"".to_string(),
                                    start_index: 87,
                                    end_index: 95,
                                    line_col: (2, 21),
                                },
                                children: vec![],
                                spread: None,
                                filters: vec![],
                                kind: ValueKind::String,
                                start_index: 87,
                                end_index: 95,
                                line_col: (2, 21),
                            },
                            TagValue {
                                token: TagToken {
                                    token: "\"key2\"".to_string(),
                                    start_index: 141,
                                    end_index: 147,
                                    line_col: (3, 13),
                                },
                                children: vec![],
                                spread: None,
                                filters: vec![],
                                kind: ValueKind::String,
                                start_index: 141,
                                end_index: 147,
                                line_col: (3, 13),
                            },
                            TagValue {
                                token: TagToken {
                                    token: "\"value2\"".to_string(),
                                    start_index: 149,
                                    end_index: 157,
                                    line_col: (3, 21),
                                },
                                children: vec![],
                                spread: None,
                                filters: vec![],
                                kind: ValueKind::String,
                                start_index: 149,
                                end_index: 157,
                                line_col: (3, 21),
                            },
                        ],
                        spread: None,
                        filters: vec![],
                        kind: ValueKind::Dict,
                        start_index: 35,
                        end_index: 196,
                        line_col: (1, 36),
                    },
                    is_flag: false,
                    start_index: 35,
                    end_index: 196,
                    line_col: (1, 36),
                }],
                is_self_closing: false,
                syntax: TagSyntax::Django,
                start_index: 0,
                end_index: 223,
                line_col: (1, 4),
            }
        );
    }

    #[test]
    fn test_dict_comments_colons_commas() {
        // Test comments around colons and commas
        let input = r#"{% my_tag {
            "key1" {# comment before colon #}: {# comment after colon #} "value1" {# comment before comma #}, {# comment after comma #}
            "key2": "value2"
        } %}"#;
        let result = TagParser::parse_tag(input, &HashSet::new()).unwrap();
        assert_eq!(
            result,
            Tag {
                name: TagToken {
                    token: "my_tag".to_string(),
                    start_index: 3,
                    end_index: 9,
                    line_col: (1, 4),
                },
                attrs: vec![TagAttr {
                    key: None,
                    value: TagValue {
                        token: TagToken {
                            token: r#"{
            "key1" {# comment before colon #}: {# comment after colon #} "value1" {# comment before comma #}, {# comment after comma #}
            "key2": "value2"
        }"#.to_string(),
                            start_index: 10,
                            end_index: 186,
                            line_col: (1, 11),
                        },
                        children: vec![
                            TagValue {
                                token: TagToken {
                                    token: "\"key1\"".to_string(),
                                    start_index: 24,
                                    end_index: 30,
                                    line_col: (2, 13),
                                },
                                children: vec![],
                                spread: None,
                                filters: vec![],
                                kind: ValueKind::String,
                                start_index: 24,
                                end_index: 30,
                                line_col: (2, 13),
                            },
                            TagValue {
                                token: TagToken {
                                    token: "\"value1\"".to_string(),
                                    start_index: 85,
                                    end_index: 93,
                                    line_col: (2, 74),
                                },
                                children: vec![],
                                spread: None,
                                filters: vec![],
                                kind: ValueKind::String,
                                start_index: 85,
                                end_index: 93,
                                line_col: (2, 74),
                            },
                            TagValue {
                                token: TagToken {
                                    token: "\"key2\"".to_string(),
                                    start_index: 160,
                                    end_index: 166,
                                    line_col: (3, 13),
                                },
                                children: vec![],
                                spread: None,
                                filters: vec![],
                                kind: ValueKind::String,
                                start_index: 160,
                                end_index: 166,
                                line_col: (3, 13),
                            },
                            TagValue {
                                token: TagToken {
                                    token: "\"value2\"".to_string(),
                                    start_index: 168,
                                    end_index: 176,
                                    line_col: (3, 21),
                                },
                                children: vec![],
                                spread: None,
                                filters: vec![],
                                kind: ValueKind::String,
                                start_index: 168,
                                end_index: 176,
                                line_col: (3, 21),
                            },
                        ],
                        spread: None,
                        filters: vec![],
                        kind: ValueKind::Dict,
                        start_index: 10,
                        end_index: 186,
                        line_col: (1, 11),
                    },
                    is_flag: false,
                    start_index: 10,
                    end_index: 186,
                    line_col: (1, 11),
                }],
                is_self_closing: false,
                syntax: TagSyntax::Django,
                start_index: 0,
                end_index: 189,
                line_col: (1, 4),
            }
        );
    }

    #[test]
    fn test_dict_comments_spread() {
        // Test comments around spread operator
        let input = r#"{% my_tag {
            "key1": "value1",
            {# comment before spread #}**{# comment after spread #}{"key2": "value2"}
        } %}"#;
        let result = TagParser::parse_tag(input, &HashSet::new()).unwrap();
        assert_eq!(
            result,
            Tag {
                name: TagToken {
                    token: "my_tag".to_string(),
                    start_index: 3,
                    end_index: 9,
                    line_col: (1, 4),
                },
                attrs: vec![TagAttr {
                    key: None,
                    value: TagValue {
                        token: TagToken {
                            token: r#"{
            "key1": "value1",
            {# comment before spread #}**{# comment after spread #}{"key2": "value2"}
        }"#
                            .to_string(),
                            start_index: 10,
                            end_index: 137,
                            line_col: (1, 11),
                        },
                        children: vec![
                            TagValue {
                                token: TagToken {
                                    token: "\"key1\"".to_string(),
                                    start_index: 24,
                                    end_index: 30,
                                    line_col: (2, 13),
                                },
                                children: vec![],
                                spread: None,
                                filters: vec![],
                                kind: ValueKind::String,
                                start_index: 24,
                                end_index: 30,
                                line_col: (2, 13),
                            },
                            TagValue {
                                token: TagToken {
                                    token: "\"value1\"".to_string(),
                                    start_index: 32,
                                    end_index: 40,
                                    line_col: (2, 21),
                                },
                                children: vec![],
                                spread: None,
                                filters: vec![],
                                kind: ValueKind::String,
                                start_index: 32,
                                end_index: 40,
                                line_col: (2, 21),
                            },
                            TagValue {
                                token: TagToken {
                                    token: r#"{"key2": "value2"}"#.to_string(),
                                    start_index: 109,
                                    end_index: 127,
                                    line_col: (3, 68),
                                },
                                children: vec![
                                    TagValue {
                                        token: TagToken {
                                            token: "\"key2\"".to_string(),
                                            start_index: 110,
                                            end_index: 116,
                                            line_col: (3, 69),
                                        },
                                        children: vec![],
                                        spread: None,
                                        filters: vec![],
                                        kind: ValueKind::String,
                                        start_index: 110,
                                        end_index: 116,
                                        line_col: (3, 69),
                                    },
                                    TagValue {
                                        token: TagToken {
                                            token: "\"value2\"".to_string(),
                                            start_index: 118,
                                            end_index: 126,
                                            line_col: (3, 77),
                                        },
                                        children: vec![],
                                        spread: None,
                                        filters: vec![],
                                        kind: ValueKind::String,
                                        start_index: 118,
                                        end_index: 126,
                                        line_col: (3, 77),
                                    },
                                ],
                                spread: Some("**".to_string()),
                                filters: vec![],
                                kind: ValueKind::Dict,
                                start_index: 107,
                                end_index: 127,
                                line_col: (3, 66),
                            },
                        ],
                        spread: None,
                        filters: vec![],
                        kind: ValueKind::Dict,
                        start_index: 10,
                        end_index: 137,
                        line_col: (1, 11),
                    },
                    is_flag: false,
                    start_index: 10,
                    end_index: 137,
                    line_col: (1, 11),
                }],
                is_self_closing: false,
                syntax: TagSyntax::Django,
                start_index: 0,
                end_index: 140,
                line_col: (1, 4),
            }
        );
    }

    // #######################################
    // FLAGS
    // #######################################

    #[test]
    fn test_flag() {
        let input = "{% my_tag 123 my_flag key='val' %}";
        let mut flags = HashSet::new();
        flags.insert("my_flag".to_string());
        let result = TagParser::parse_tag(input, &flags).unwrap();
        assert_eq!(
            result,
            Tag {
                name: TagToken {
                    token: "my_tag".to_string(),
                    start_index: 3,
                    end_index: 9,
                    line_col: (1, 4),
                },
                attrs: vec![
                    TagAttr {
                        key: None,
                        value: TagValue {
                            token: TagToken {
                                token: "123".to_string(),
                                start_index: 10,
                                end_index: 13,
                                line_col: (1, 11),
                            },
                            children: vec![],
                            spread: None,
                            filters: vec![],
                            kind: ValueKind::Int,
                            start_index: 10,
                            end_index: 13,
                            line_col: (1, 11),
                        },
                        is_flag: false,
                        start_index: 10,
                        end_index: 13,
                        line_col: (1, 11),
                    },
                    TagAttr {
                        key: None,
                        value: TagValue {
                            token: TagToken {
                                token: "my_flag".to_string(),
                                start_index: 14,
                                end_index: 21,
                                line_col: (1, 15),
                            },
                            children: vec![],
                            spread: None,
                            filters: vec![],
                            kind: ValueKind::Variable,
                            start_index: 14,
                            end_index: 21,
                            line_col: (1, 15),
                        },
                        is_flag: true, // This is what we're testing
                        start_index: 14,
                        end_index: 21,
                        line_col: (1, 15),
                    },
                    TagAttr {
                        key: Some(TagToken {
                            token: "key".to_string(),
                            start_index: 22,
                            end_index: 25,
                            line_col: (1, 23),
                        }),
                        value: TagValue {
                            token: TagToken {
                                token: "'val'".to_string(),
                                start_index: 26,
                                end_index: 31,
                                line_col: (1, 27),
                            },
                            children: vec![],
                            spread: None,
                            filters: vec![],
                            kind: ValueKind::String,
                            start_index: 26,
                            end_index: 31,
                            line_col: (1, 27),
                        },
                        is_flag: false,
                        start_index: 22,
                        end_index: 31,
                        line_col: (1, 23),
                    },
                ],
                is_self_closing: false,
                syntax: TagSyntax::Django,
                start_index: 0,
                end_index: 34,
                line_col: (1, 4),
            }
        );
    }

    #[test]
    fn test_flag_not_as_flag() {
        // Same as test_flag, but `my_flag` is not in the flags set
        let input = "{% my_tag 123 my_flag key='val' %}";
        let flags = HashSet::new(); // empty set
        let result = TagParser::parse_tag(input, &flags).unwrap();
        assert_eq!(
            result,
            Tag {
                name: TagToken {
                    token: "my_tag".to_string(),
                    start_index: 3,
                    end_index: 9,
                    line_col: (1, 4),
                },
                attrs: vec![
                    TagAttr {
                        key: None,
                        value: TagValue {
                            token: TagToken {
                                token: "123".to_string(),
                                start_index: 10,
                                end_index: 13,
                                line_col: (1, 11),
                            },
                            children: vec![],
                            spread: None,
                            filters: vec![],
                            kind: ValueKind::Int,
                            start_index: 10,
                            end_index: 13,
                            line_col: (1, 11),
                        },
                        is_flag: false,
                        start_index: 10,
                        end_index: 13,
                        line_col: (1, 11),
                    },
                    TagAttr {
                        key: None,
                        value: TagValue {
                            token: TagToken {
                                token: "my_flag".to_string(),
                                start_index: 14,
                                end_index: 21,
                                line_col: (1, 15),
                            },
                            children: vec![],
                            spread: None,
                            filters: vec![],
                            kind: ValueKind::Variable,
                            start_index: 14,
                            end_index: 21,
                            line_col: (1, 15),
                        },
                        is_flag: false, // This is what we're testing
                        start_index: 14,
                        end_index: 21,
                        line_col: (1, 15),
                    },
                    TagAttr {
                        key: Some(TagToken {
                            token: "key".to_string(),
                            start_index: 22,
                            end_index: 25,
                            line_col: (1, 23),
                        }),
                        value: TagValue {
                            token: TagToken {
                                token: "'val'".to_string(),
                                start_index: 26,
                                end_index: 31,
                                line_col: (1, 27),
                            },
                            children: vec![],
                            spread: None,
                            filters: vec![],
                            kind: ValueKind::String,
                            start_index: 26,
                            end_index: 31,
                            line_col: (1, 27),
                        },
                        is_flag: false,
                        start_index: 22,
                        end_index: 31,
                        line_col: (1, 23),
                    },
                ],
                is_self_closing: false,
                syntax: TagSyntax::Django,
                start_index: 0,
                end_index: 34,
                line_col: (1, 4),
            }
        );
    }

    #[test]
    fn test_flag_as_spread() {
        let input = "{% my_tag ...my_flag %}";
        let mut flags = HashSet::new();
        flags.insert("my_flag".to_string());
        let result = TagParser::parse_tag(input, &flags).unwrap();

        assert_eq!(
            result,
            Tag {
                name: TagToken {
                    token: "my_tag".to_string(),
                    start_index: 3,
                    end_index: 9,
                    line_col: (1, 4),
                },
                attrs: vec![TagAttr {
                    key: None,
                    value: TagValue {
                        token: TagToken {
                            token: "my_flag".to_string(),
                            start_index: 13,
                            end_index: 20,
                            line_col: (1, 14),
                        },
                        children: vec![],
                        spread: Some("...".to_string()),
                        filters: vec![],
                        kind: ValueKind::Variable,
                        start_index: 10,
                        end_index: 20,
                        line_col: (1, 11),
                    },
                    is_flag: false, // This is what we're testing
                    start_index: 10,
                    end_index: 20,
                    line_col: (1, 11),
                },],
                is_self_closing: false,
                syntax: TagSyntax::Django,
                start_index: 0,
                end_index: 23,
                line_col: (1, 4),
            }
        );
    }

    #[test]
    fn test_flag_as_kwarg() {
        let input = "{% my_tag my_flag=123 %}";
        let mut flags = HashSet::new();
        flags.insert("my_flag".to_string());
        let result = TagParser::parse_tag(input, &flags).unwrap();

        assert_eq!(
            result,
            Tag {
                name: TagToken {
                    token: "my_tag".to_string(),
                    start_index: 3,
                    end_index: 9,
                    line_col: (1, 4),
                },
                attrs: vec![TagAttr {
                    key: Some(TagToken {
                        token: "my_flag".to_string(),
                        start_index: 10,
                        end_index: 17,
                        line_col: (1, 11),
                    }),
                    value: TagValue {
                        token: TagToken {
                            token: "123".to_string(),
                            start_index: 18,
                            end_index: 21,
                            line_col: (1, 19),
                        },
                        children: vec![],
                        spread: None,
                        filters: vec![],
                        kind: ValueKind::Int,
                        start_index: 18,
                        end_index: 21,
                        line_col: (1, 19),
                    },
                    is_flag: false, // This is what we're testing
                    start_index: 10,
                    end_index: 21,
                    line_col: (1, 11),
                },],
                is_self_closing: false,
                syntax: TagSyntax::Django,
                start_index: 0,
                end_index: 24,
                line_col: (1, 4),
            }
        );
    }

    #[test]
    fn test_flag_duplicate() {
        let input = "{% my_tag my_flag my_flag %}";
        let mut flags = HashSet::new();
        flags.insert("my_flag".to_string());
        let result = TagParser::parse_tag(input, &flags);
        assert!(result.is_err());
        if let Err(ParseError::InvalidKey(msg)) = result {
            assert_eq!(msg, "Flag 'my_flag' may be specified only once.");
        } else {
            panic!("Expected InvalidKey error");
        }
    }

    #[test]
    fn test_flag_case_sensitive() {
        let input = "{% my_tag my_flag %}";
        let mut flags = HashSet::new();
        flags.insert("MY_FLAG".to_string()); // Different case
        let result = TagParser::parse_tag(input, &flags).unwrap();

        // my_flag should not be a flag
        assert_eq!(result.attrs[0].is_flag, false);
    }

    // #######################################
    // SELF-CLOSING TAGS
    // #######################################

    #[test]
    fn test_self_closing_tag() {
        let input = "{% my_tag / %}";
        let result = TagParser::parse_tag(input, &HashSet::new()).unwrap();

        assert_eq!(
            result,
            Tag {
                name: TagToken {
                    token: "my_tag".to_string(),
                    start_index: 3,
                    end_index: 9,
                    line_col: (1, 4),
                },
                attrs: vec![],
                is_self_closing: true,
                syntax: TagSyntax::Django,
                start_index: 0,
                end_index: 14,
                line_col: (1, 4),
            }
        );
    }

    #[test]
    fn test_self_closing_tag_with_args() {
        let input = "{% my_tag key=val / %}";
        let result = TagParser::parse_tag(input, &HashSet::new()).unwrap();

        assert_eq!(
            result,
            Tag {
                name: TagToken {
                    token: "my_tag".to_string(),
                    start_index: 3,
                    end_index: 9,
                    line_col: (1, 4),
                },
                attrs: vec![TagAttr {
                    key: Some(TagToken {
                        token: "key".to_string(),
                        start_index: 10,
                        end_index: 13,
                        line_col: (1, 11),
                    }),
                    value: TagValue {
                        token: TagToken {
                            token: "val".to_string(),
                            start_index: 14,
                            end_index: 17,
                            line_col: (1, 15),
                        },
                        children: vec![],
                        spread: None,
                        filters: vec![],
                        kind: ValueKind::Variable,
                        start_index: 14,
                        end_index: 17,
                        line_col: (1, 15),
                    },
                    is_flag: false,
                    start_index: 10,
                    end_index: 17,
                    line_col: (1, 11),
                },],
                is_self_closing: true,
                syntax: TagSyntax::Django,
                start_index: 0,
                end_index: 22,
                line_col: (1, 4),
            }
        );
    }

    #[test]
    fn test_self_closing_tag_in_middle_errors() {
        let input = "{% my_tag / key=val %}";
        let result = TagParser::parse_tag(input, &HashSet::new());
        assert!(
            result.is_err(),
            "Self-closing slash in the middle should be an error"
        );
        // The error message will vary depending on the parser state, so just check it's an error
    }
}
