use crate::api::http::messages::metric_message::MetricRule;
use crate::application::service::rhai_service::RhaiService;
use crate::core::metric::window_state::WindowState;
use crate::core::rhai::engine_builder::execution_engine;
use crate::core::rhai::rhai_executor::RhaiExecutor;
use crate::core::rhai::rhai_parser::RhaiParser;
use crate::platform::registry::Registry;
use actix::{Actor, Addr, AsyncContext, Context, Message};
use std::collections::BTreeMap;
use tracing::info;
use crate::application::actors::rhai::handler::record_batch_wrapper::RhaiActorReady;

#[derive(Message, Clone)]
#[rtype(result = "()")]
pub enum RhaiActorAddr {
    Real(Addr<RhaiActor>),
    #[cfg(test)]
    Mock(Addr<MockRhaiActor>),
    Empty,
}

pub struct RhaiActor {
    pub flight_name: String,
    pub rhai_service: RhaiService,
    pub registry_address: Addr<Registry>,
    pub(crate) window_state: WindowState,
    pub(crate) rules: BTreeMap<String, MetricRule>,
}

impl RhaiActor {
    pub fn new(flight_name: String, registry_address: Addr<Registry>) -> Self {
        // create a new executor
        let engine = execution_engine();
        let rhai_parser = RhaiParser::new(engine); // used to parse ASTs
        let rhai_executor = RhaiExecutor::new(flight_name.clone());
        let rhai_service = RhaiService::new(rhai_parser, rhai_executor);
        let window_state = WindowState::new(10_000, 10_000);
        Self {
            flight_name,
            rhai_service,
            registry_address,
            window_state,
            rules: BTreeMap::new(),
        }
    }

    pub fn get_registry_actor(&self) -> Addr<Registry> {
        self.registry_address.clone()
    }

    pub fn get_flight_name(&self) -> &str {
        &self.flight_name
    }

    /// Helper to round down a nanoseconds epoch to nearest multiple of `window_size_sec`
    pub(crate) fn round_down_to_window(&self, ts_nanos: i64) -> i64 {
        let window_ns = self.window_state.window_size_sec;
        (ts_nanos / window_ns) * window_ns
    }
}

impl Actor for RhaiActor {
    type Context = Context<Self>;

    fn started(&mut self, _ctx: &mut Self::Context) {
        info!("RHAI actor started");
        let flight = self.flight_name.clone();
        self.registry_address.do_send(RhaiActorReady {
            flight_name: flight,
            rhai_actor_addr: RhaiActorAddr::Real(_ctx.address()),
        });
    }
}

pub struct MockRhaiActor {}

impl Actor for MockRhaiActor {
    type Context = Context<Self>;
}
