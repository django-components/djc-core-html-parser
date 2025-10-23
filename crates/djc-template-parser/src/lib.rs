use std::collections::HashSet;
use tag_parser::{ParseError, TagParser};

pub mod ast;
pub mod error;
pub mod tag_compiler;
pub mod tag_parser;

// Re-export the types that users need
pub use ast::{Tag, TagAttr, TagSyntax, TagToken, TagValue, TagValueFilter, ValueKind};

/// Parse a template tag string into a Tag AST
pub fn parse_tag(input: &str, flags: Option<HashSet<String>>) -> Result<Tag, ParseError> {
    let flags_set = flags.unwrap_or_else(HashSet::new);
    TagParser::parse_tag(input, &flags_set)
}

/// Compile a list of TagAttr to a string
pub fn compile_ast_to_string(attributes: &[TagAttr]) -> Result<String, error::CompileError> {
    tag_compiler::compile_ast_to_string(attributes)
}
