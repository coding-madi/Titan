use crate::application::actors::broadcast_actor::BroadcastActor;
#[cfg(test)]
use crate::application::actors::factory::factory_actor::tests::MockFactoryActor;
#[cfg(test)]
use crate::application::actors::parser_actor::MockParsingActor;
use crate::application::service::parser_service::ParserService;
use crate::core::parser::parser_contract::ParserType;
use crate::core::parser::rust_regex_engine::RustRegexEngine;
use crate::platform::registry::{ParserActor, ParserActorAddr, Registry};
use actix::{Actor, Addr, AsyncContext, Handler, Message};
use log::info;
use std::sync::Arc;

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

#[derive(Message)]
#[rtype(result = "Vec<ParserActorAddr>")]
pub struct CreateParserActor {
    pub flight_name: String,
    pub count: usize,
    pub parser_type: ParserType,
}

impl Handler<CreateParserActor> for FactoryActor {
    type Result = Vec<ParserActorAddr>;

    fn handle(&mut self, msg: CreateParserActor, _ctx: &mut Self::Context) -> Self::Result {
        let mut result = vec![];
        for _ in 0..msg.count {
            let parsing_actor = ParserActorAddr::Real(match msg.parser_type {
                ParserType::RUSTREGEX => {
                    let parser_service = ParserService::new(
                        msg.flight_name.clone(),
                        self.registry.clone(),
                        Arc::new(RustRegexEngine {}),
                    );
                    ParserActor::new(parser_service).start()
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

#[cfg(test)]
pub mod tests {
    use super::*;
    use crate::application::actors::wal_actor::MockWalActor;
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
