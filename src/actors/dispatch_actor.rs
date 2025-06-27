// Dispatch actor handles below messages

use crate::actors::parser_actor::ParsingActor;
use actix::{Addr, Message};

#[derive(Message)]
#[rtype(result = "()")]
pub enum DispatchCommand {
    ParserHandler(Addr<ParsingActor>),
}
