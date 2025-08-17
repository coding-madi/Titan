use thiserror::Error;

#[derive(Debug, Error)]
pub enum IcebergError {
    #[error("Flush failed: {0}")]
    FlushError(String),

    #[error("Commit failed: {0}")]
    CommitError(String),

    #[error("Storage error: {0}")]
    StorageError(String),

    #[error("Namespace error: {0}")]
    NamespaceError(String),

    #[error("Table error: {0}")]
    TableError(String),
}

impl From<std::string::String> for IcebergError {
    fn from(e: std::string::String) -> Self {
        IcebergError::StorageError(e.to_string())
    }
}
