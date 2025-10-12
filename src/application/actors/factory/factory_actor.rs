use crate::application::actors::broadcaster::broadcast_actor::BroadcastActor;
#[cfg(test)]
use crate::application::actors::factory::factory_actor::tests::MockFactoryActor;
use crate::application::actors::messages::registeration::{CreateParserActor, CreateRhaiActor};
#[cfg(test)]
use crate::application::actors::parser::parser_actor::MockParsingActor;
use crate::application::actors::rhai::rhai_actor::{RhaiActor, RhaiActorAddr};
use crate::application::service::parser_service::ParserService;
use crate::core::parser::parser_contract::ParserType;
use crate::core::parser::rust_regex_engine::RustRegexEngine;
use crate::platform::registry::{ParserActor, ParserActorAddr, Registry};
use actix::{Actor, Addr, AsyncContext, Handler, Message};
use std::sync::Arc;
use tracing::info;

#[derive(Clone, Message, Debug)]
#[rtype(result = "()")]
pub enum FactoryActorAddr {
    Real(Addr<FactoryActor>),
    #[cfg(test)]
    Mock(Addr<MockFactoryActor>),
    Empty,
}

#[derive(Message, Clone)]
#[rtype(result = "()")]
pub struct FactoryActor {
    registry: Addr<Registry>,
}

impl FactoryActor {
    pub fn new(registry: Addr<Registry>) -> Self {
        FactoryActor { registry }
    }
}

impl Actor for FactoryActor {
    type Context = actix::Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        let address = ctx.address();
        let registry = self.registry.clone();
        registry.do_send(FactoryActorAddr::Real(address.clone()));
        info!("Factory actor started")
    }
}

#[derive(Message)]
#[rtype(result = "Addr<BroadcastActor>")]
pub struct CreateBroadcastActor {
    pub(crate) flight_name: String,
    pub parser_actors: Vec<ParserActorAddr>,
}

impl Handler<CreateBroadcastActor> for FactoryActor {
    type Result = Addr<BroadcastActor>;

    fn handle(&mut self, msg: CreateBroadcastActor, _ctx: &mut Self::Context) -> Self::Result {
        let registry = self.registry.clone();
        let flight_name = msg.flight_name.clone();
        BroadcastActor::new(registry, flight_name, msg.parser_actors).start()
    }
}

impl Handler<CreateParserActor> for FactoryActor {
    type Result = Vec<ParserActorAddr>;

    fn handle(&mut self, msg: CreateParserActor, _ctx: &mut Self::Context) -> Self::Result {
        let mut result = vec![];
        info!("Creating {} parser actors", msg.count);
        let rhai_actor = msg.rhai_actor.clone();
        for _ in 0..msg.count {
            let parsing_actor = ParserActorAddr::Real(match msg.parser_type {
                ParserType::RUSTREGEX => {
                    let parser_service = ParserService::new(
                        msg.flight_name.clone(),
                        self.registry.clone(),
                        Arc::new(RustRegexEngine {}),
                        Some(msg.rhai_actor.clone()),
                    );
                    ParserActor::new(msg.flight_name.clone(), parser_service, rhai_actor.clone())
                        .start()
                }
                ParserType::Grok => {
                    unimplemented!()
                }
                ParserType::ArrowRegex => {
                    unimplemented!()
                }
            });
            #[cfg(test)]
            let parsing_actor =
                ParserActorAddr::Mock(MockParsingActor::new(self.registry.clone()).start());
            &mut result.push(parsing_actor);
        }
        result
    }
}

impl Handler<CreateRhaiActor> for FactoryActor {
    type Result = Addr<RhaiActor>;

    fn handle(&mut self, msg: CreateRhaiActor, _ctx: &mut Self::Context) -> Self::Result {
        RhaiActor::new(msg.flight_name, self.registry.clone()).start()
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use actix::Context;
    use tracing::info;

    pub struct MockFactoryActor {}

    impl MockFactoryActor {
        pub fn new(_registry_addr: Addr<Registry>) -> Self {
            MockFactoryActor {}
        }
    }

    impl Actor for MockFactoryActor {
        type Context = Context<Self>;

        fn started(&mut self, _ctx: &mut Self::Context) {
            info!("Mock WAL actor started");
        }
    }
}
