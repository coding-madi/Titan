use thiserror::Error;

#[derive(Debug, Error)]
pub enum RhaiError {
    #[error("ScriptCompilation: {0}")]
    ScriptCompilation(String),
    #[error("TypeMismatch: {0}")]
    TypeMismatch(String),
    #[error("ScriptExecution: {0}")]
    ScriptExecution(String),
}

impl From<String> for RhaiError {
    fn from(e: String) -> Self {
        RhaiError::ScriptCompilation(e)
    }
}
