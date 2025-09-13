use serde::Serialize;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum RegexError {
    #[error("Unable to compile Regex {0}")]
    RegexIncorrect(String),
    #[error("Error execution of dataset {0}")]
    RegexExecutionError(String),
    #[error("No matching pattern {0}")]
    NoMatchingPattern(String),
}

impl From<String> for RegexError {
    fn from(e: String) -> Self {
        RegexError::RegexIncorrect(e.to_string())
    }
}

impl Serialize for RegexError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&format!("{:?}", self))
    }
}
