use crate::core::metric::rhai_engine::RhaiEngine;
use crate::core::metric::rhai_executor::RhaiExecutor;
use crate::core::metric::rhai_orchestrator::Orchestrator;
use crate::platform::registry::Registry;
use actix::{Actor, Addr, Context, Handler, Message};
use crate::application::actors::broadcast::RecordBatchWrapper;

#[derive(Message, Clone)]
#[rtype(result = "()")]
pub enum RhaiActorAddr {
    Real(Vec<Addr<RhaiActor>>),
    #[cfg(test)]
    Mock(Vec<Addr<MockRhaiActor>>),
    Empty
}

pub struct RhaiActor {
    pub orchestrator: Orchestrator,
    pub registry_address: Addr<Registry>
}

impl RhaiActor {
    pub fn new(registry_address: Addr<Registry>) -> Self {
        // create a new executor
        let engine = RhaiEngine::new();

        let executor = RhaiExecutor::new();
        let orchestrator = Orchestrator::new(engine, executor);
        Self {
            orchestrator,
            registry_address
        }
    }
}

impl Actor for RhaiActor {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
    }
}

impl Handler<RecordBatchWrapper> for RhaiActor {
    type Result = ();

    fn handle(&mut self, msg: RecordBatchWrapper, ctx: &mut Self::Context) -> Self::Result {
        self.orchestrator.append_log(msg);
    }
}


pub struct MockRhaiActor {}

impl Actor for MockRhaiActor { type Context = Context<Self>; }