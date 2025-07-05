use actix::{Actor, Handler, Message};
use std::collections::{HashMap, HashSet};
use std::io::Error;
use tracing::info;

pub struct FlightRegistry {
    pub flights: HashMap<String, HashSet<String>>,
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

    fn handle(&mut self, flight_check: CheckFlight, ctx: &mut Self::Context) -> Self::Result {
        match self.flights.get(flight_check.team_id.as_str()) {
            Some(team_flights) => Ok(team_flights.contains(flight_check.flight.as_str())),
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

    fn handle(&mut self, msg: ListFlights, ctx: &mut Self::Context) -> Self::Result {
        match self.flights.get(msg.team_id.as_str()) {
            Some(team_flights) => Ok(team_flights.clone()),
            None => Err(Error::new(std::io::ErrorKind::NotFound, "Team not found")),
        }
    }
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct RegisterFlight {
    pub team_id: String,
    pub flight: String,
}

impl Handler<RegisterFlight> for FlightRegistry {
    type Result = ();

    fn handle(&mut self, msg: RegisterFlight, ctx: &mut Self::Context) -> Self::Result {
        info!("Created flight {} for team {}", msg.flight, msg.team_id);
        self.flights
            .entry(msg.team_id.clone())
            .or_insert(HashSet::new())
            .insert(msg.flight.clone());
    }
}
