use thiserror::Error;

#[derive(Debug, Error)]
pub enum FlightError {
    #[error("Flight missing: {0}")]
    FlightMissing(String),
}

impl From<std::string::String> for FlightError {
    fn from(e: std::string::String) -> Self {
        FlightError::FlightMissing(e)
    }
}
