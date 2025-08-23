use thiserror::Error;

#[derive(Debug, Error)]
pub enum RegistryError {
    #[error("Actor missing, {0}")]
    ActorNotInitialized(String),
}
