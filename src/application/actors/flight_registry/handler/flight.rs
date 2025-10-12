use crate::application::actors::flight_registry::flight_registry_actor::FlightRegistry;
use actix::{Handler, Message};
use std::collections::HashSet;
use std::io::Error;
use tracing::info;

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
        self.flights.insert(msg.flight, msg.fields);
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
