use crate::application::actors::broadcaster::broadcast_actor::BroadcastActorWrapper;
use crate::application::actors::parser::parser_actor::ParserActorAddr;
use crate::application::actors::rhai::rhai_actor::RhaiActor;
use crate::core::parser::parser_contract::ParserType;
use actix::Addr;
use actix::Message;

#[derive(Message)]
#[rtype(result = "Vec<ParserActorAddr>")]
pub struct CreateParserActor {
    pub flight_name: String,
    pub rhai_actor: Addr<RhaiActor>,
    pub count: usize,
    pub parser_type: ParserType,
}

#[derive(Message)]
#[rtype(result = "Addr<RhaiActor>")]
pub struct CreateRhaiActor {
    pub flight_name: String,
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct RegisterBroadcastActor {
    pub flight_name: String,
    pub broadcast_actor_wrapped_single: BroadcastActorWrapper,
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct ParserActorReady {
    pub flight_name: String,
    pub parser_actor_addr: ParserActorAddr,
}
