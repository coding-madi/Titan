use actix::MailboxError;
use std::fmt::{Display, Formatter};

#[derive(Debug)]
pub enum ErrorType {
    ActorError(MailboxError),
}

impl Display for ErrorType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ErrorType::ActorError(e) => write!(f, "Actor error: {}", e),
        }
    }
}
