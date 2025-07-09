use actix::{Actor, Handler, Message};
use std::collections::{HashMap, HashSet};
use std::io::Error;
use tracing::info;

pub struct FlightRegistry {
    pub flights: HashMap<String, HashMap<String, Vec<Fields>>>,
}

impl FlightRegistry {
    pub async fn new() -> Self {
        Self {
            flights: HashMap::new(),
        }
    }
}

impl Actor for FlightRegistry {
    type Context = actix::Context<Self>;
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
