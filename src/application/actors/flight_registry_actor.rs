#[cfg(test)]
use crate::application::actors::parser_actor::SubmitRegexRequest;
use crate::platform::registry::Registry;
use actix::{Actor, Addr, AsyncContext};
use actix::{Handler, Message};
use std::collections::{HashMap, HashSet};
use std::io::Error;
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
    pub async fn new(registry: Addr<Registry>) -> Self {
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

#[derive(Message)]
#[rtype(result = "bool")]
pub struct CheckFlight {
    pub flight: String,
}

impl Handler<CheckFlight> for FlightRegistry {
    type Result = bool;

    fn handle(&mut self, flight_check: CheckFlight, _ctx: &mut Self::Context) -> Self::Result {
        self.flights.contains_key(flight_check.flight.as_str())
    }
}

#[derive(Message)]
#[rtype(result = "Result<HashSet<String>, Error>")]
pub struct ListFlights {
    pub team_id: String,
}

impl Handler<ListFlights> for FlightRegistry {
    type Result = Result<HashSet<String>, Error>;

    fn handle(&mut self, msg: ListFlights, _ctx: &mut Self::Context) -> Self::Result {
        todo!()
    }
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct RegisterFlight {
    pub flight: String,
    pub fields: Vec<Fields>,
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct Fields {
    pub column_name: String,
    pub data_type: String,
}

impl Handler<RegisterFlight> for FlightRegistry {
    type Result = ();

    fn handle(&mut self, msg: RegisterFlight, _ctx: &mut Self::Context) -> Self::Result {
        info!("Created flight {} for team", msg.flight);
        let x = self.flights.insert(msg.flight, msg.fields);
    }
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct FlightData {
    pub id: String,
    pub details: String,
}

impl Handler<FlightData> for FlightRegistry {
    type Result = ();

    fn handle(&mut self, _msg: FlightData, _ctx: &mut Self::Context) -> Self::Result {
        todo!()
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
