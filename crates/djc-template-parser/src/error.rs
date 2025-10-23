use thiserror::Error;

#[derive(Debug, Error, PartialEq)]
pub enum CompileError {
    #[error("{0}")]
    Generic(String),
}

impl From<String> for CompileError {
    fn from(error: String) -> Self {
        CompileError::Generic(error)
    }
}

impl From<&str> for CompileError {
    fn from(error: &str) -> Self {
        CompileError::Generic(error.to_string())
    }
}
