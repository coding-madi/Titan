use thiserror::Error;

#[derive(Debug, Error)]
pub enum ActorError {
    #[error("Actor mail box panic!!: {0}")]
    ActorError(String),
    #[error("Actor not started: {0}")]
    ActorNotStarted(String),
}
