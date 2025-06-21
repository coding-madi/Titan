use std::{fmt, io};

#[derive(Debug)]
pub enum ServerError {
    Io(io::Error),
    TonicTransport(tonic::transport::Error),
}

impl fmt::Display for ServerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ServerError::Io(err) => write!(f, "I/O error: {}", err),
            ServerError::TonicTransport(err) => write!(f, "Tonic transport error: {}", err),
        }
    }
}

// Manual implementation of std::error::Error for ServerError
impl std::error::Error for ServerError {
    // The `source` method provides the underlying error if available.
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ServerError::Io(err) => Some(err),
            ServerError::TonicTransport(err) => Some(err),
        }
    }
}

// Manual `From` implementations to allow using the `?` operator
impl From<io::Error> for ServerError {
    fn from(err: io::Error) -> Self {
        ServerError::Io(err)
    }
}

impl From<tonic::transport::Error> for ServerError {
    fn from(err: tonic::transport::Error) -> Self {
        ServerError::TonicTransport(err)
    }
}
