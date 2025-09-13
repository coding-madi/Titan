use crate::application::actors::broadcast_actor::RecordBatchWrapper;
use crate::core::rhai::engine_builder::execution_engine;
use crate::core::rhai::rhai_engine::RhaiEngine;
use crate::core::rhai::rhai_executor::RhaiExecutor;
use crate::core::rhai::rhai_orchestrator::Orchestrator;
use crate::platform::registry::Registry;
use actix::{Actor, Addr, Context, Handler, Message};

#[derive(Message, Clone)]
#[rtype(result = "()")]
pub enum RhaiActorAddr {
    Real(Vec<Addr<RhaiActor>>),
    #[cfg(test)]
    Mock(Vec<Addr<MockRhaiActor>>),
    Empty,
}

pub struct RhaiActor {
    pub orchestrator: Orchestrator,
    pub registry_address: Addr<Registry>,
}

impl RhaiActor {
    pub fn new(flight_name: String, registry_address: Addr<Registry>) -> Self {
        // create a new executor
        let engine = execution_engine();
        let rhai_engine = RhaiEngine::new(engine);

        let executor = RhaiExecutor::new(flight_name);
        let orchestrator = Orchestrator::new(rhai_engine, executor);
        Self {
            orchestrator,
            registry_address,
        }
    }
}

impl Actor for RhaiActor {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {}
}

impl Handler<RecordBatchWrapper> for RhaiActor {
    type Result = ();

    fn handle(&mut self, msg: RecordBatchWrapper, ctx: &mut Self::Context) -> Self::Result {
        self.orchestrator.append_log(msg);
    }
}

pub struct MockRhaiActor {}

impl Actor for MockRhaiActor {
    type Context = Context<Self>;
}
