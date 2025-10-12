use thiserror::Error;

#[derive(Debug, Error)]
pub enum BufferError {
    #[error("Buffer missing : {0}")]
    BufferMissing(String),
}
