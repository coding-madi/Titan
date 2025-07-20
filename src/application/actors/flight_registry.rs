#[cfg(test)]
use crate::application::actors::parser::RegexRequest;
use crate::platform::registry::Registry;
use actix::{Actor, Addr, AsyncContext};
use actix::{Handler, Message};
use actix_rt::spawn;
use std::collections::{HashMap, HashSet};
use std::io::Error;
use tracing::info;
#[cfg(test)]
use validator::ValidationErrors;

#[derive(Clone, Message)]
#[rtype(result = "()")]
pub enum FlightRegistryActorAddr {
    Real(Addr<FlightRegistry>),
    #[cfg(test)]
    Mock(Addr<MockFlightRegistry>),
    Empty,
}

pub struct FlightRegistry {
    pub flights: HashMap<String, HashMap<String, Vec<Fields>>>,
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
            registry_address.do_send(FlightRegistryActorAddr::Real(address));
        });
        info!("Started FlightRegistry");
    }
}

#[derive(Message)]
#[rtype(result = "Result<bool, Error>")]
pub struct CheckFlight {
    pub team_id: String,
    pub flight: String,
}

impl Handler<CheckFlight> for FlightRegistry {
    type Result = Result<bool, Error>;

    fn handle(&mut self, flight_check: CheckFlight, _ctx: &mut Self::Context) -> Self::Result {
        match self.flights.get(flight_check.team_id.as_str()) {
            Some(team_flights) => Ok(team_flights.contains_key(flight_check.flight.as_str())),
            None => Err(Error::new(std::io::ErrorKind::NotFound, "Team not found")),
        }
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
        match self.flights.get(msg.team_id.as_str()) {
            Some(team_flights) => {
                let flight_names = team_flights.keys().cloned().collect();
                Ok(flight_names)
            }
            None => Err(Error::new(std::io::ErrorKind::NotFound, "Team not found")),
        }
    }
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct RegisterFlight {
    pub team_id: String,
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
        info!("Created flight {} for team {}", msg.flight, msg.team_id);
        self.flights
            .entry(msg.team_id)
            .or_default()
            .insert(msg.flight, msg.fields);
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

    fn handle(&mut self, msg: FlightData, ctx: &mut Self::Context) -> Self::Result {
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

#[cfg(test)]
impl Handler<CheckFlight> for MockFlightRegistry {
    type Result = std::result::Result<bool, Error>;

    fn handle(&mut self, flight_check: CheckFlight, _ctx: &mut Self::Context) -> Self::Result {
        unimplemented!()
    }
}

#[cfg(test)]
impl Handler<ListFlights> for MockFlightRegistry {
    type Result = Result<HashSet<String>, Error>;

    fn handle(&mut self, msg: ListFlights, ctx: &mut Self::Context) -> Self::Result {
        todo!()
    }
}

#[cfg(test)]
impl Handler<RegexRequest> for MockFlightRegistry {
    type Result = Result<(), ValidationErrors>;

    fn handle(&mut self, msg: RegexRequest, ctx: &mut Self::Context) -> Self::Result {
        todo!()
    }
}
