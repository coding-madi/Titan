// Dispatch actor handles below messages

use crate::application::actors::parser::ParserActor;
use actix::{Addr, Message};

#[derive(Message)]
#[rtype(result = "()")]
pub enum DispatchCommand {
    ParserHandler(Addr<ParserActor>),
}
