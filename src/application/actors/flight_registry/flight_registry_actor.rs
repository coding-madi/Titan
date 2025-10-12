pub(crate) use crate::application::actors::flight_registry::handler::flight::Fields;
#[cfg(test)]
use crate::application::actors::parser::parser_actor::SubmitRegexRequest;
use crate::platform::registry::Registry;
use actix::Message;
use actix::{Actor, Addr, AsyncContext};
use std::collections::HashMap;
use tokio::spawn;
use tracing::info;
#[cfg(test)]
use validator::ValidationErrors;

#[derive(Clone, Message, Debug)]
#[rtype(result = "()")]
pub enum FlightRegistryActorWrapped {
    Real(Addr<FlightRegistry>),
    #[cfg(test)]
    Mock(Addr<MockFlightRegistry>),
    Empty,
}

pub struct FlightRegistry {
    pub flights: HashMap<String, Vec<Fields>>,
    pub registry: Addr<Registry>,
}

impl FlightRegistry {
    pub fn new(registry: Addr<Registry>) -> Self {
        Self {
            flights: HashMap::new(),
            registry,
        }
    }
}

impl Actor for FlightRegistry {
    type Context = actix::Context<Self>;

    fn started(&mut self, _ctx: &mut Self::Context) {
        let address = _ctx.address();
        let registry_address = self.registry.clone();
        spawn(async move {
            // let pool = settings_for_spawn.connection_pool().await;
            registry_address.do_send(FlightRegistryActorWrapped::Real(address));
        });
        info!("Started FlightRegistry");
    }
}

#[cfg(test)]
pub struct MockFlightRegistry {
    pub registry_address: Addr<Registry>,
}

#[cfg(test)]
impl MockFlightRegistry {}

#[cfg(test)]
impl Actor for MockFlightRegistry {
    type Context = actix::Context<Self>;
}
