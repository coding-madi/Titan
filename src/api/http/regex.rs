use actix::{Actor, Addr, Handler, MailboxError, Message};
use actix::dev::ToEnvelope;
use actix_web::{web, HttpResponse, Responder, Resource};
use serde_derive::{Deserialize, Serialize};
use tracing::{info, warn};
use utoipa::ToSchema;
use validator::{Validate, ValidationError, ValidationErrors};
use regex::Regex;
use std::collections::HashSet;
use std::fmt::{Debug, Display, Formatter};
use std::sync::Arc;
use actix_web::web::{Data, Path};
use serde_json::json;
use crate::application::actors::flight_registry::{CheckFlight, ListFlights};
use crate::platform::actor_factory::{ProdInjestRegistry, Registry};

/// ========== Models ==========

#[derive(Debug, Serialize, Deserialize, Validate, Clone, ToSchema, Message)]
#[rtype(result = "Result<(), ValidationErrors>")]
pub struct RegexRequest {
    #[validate(length(min = 3, message = "Name must be greater than 3 chars"))]
    pub name: String,
    #[schema(example = "team-123")]
    pub tenant: String,
    #[schema(example = "flight-abc")]
    pub flight_id: String,
    pub log_group: String,
    #[validate(custom(function = "validate_regex_pattern"))]
    pub pattern: Vec<Pattern>,
}

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
#[serde(tag = "type", content = "value")]
pub enum Pattern {
    RegexPattern(RegexPattern),
    GrokPattern(GrokPattern),
}

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct RegexPattern {
    pub override_field: Option<String>,
    pub field: String,
    #[schema(example = ".*ERROR.*")]
    pub pattern_string: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct GrokPattern {
    pub override_field: Option<String>,
    pub field: String,
    #[schema(example = ".*ERROR.*")]
    pub pattern_string: String,
}

/// ========== Errors ==========

#[derive(Debug)]
pub enum ErrorType {
    ActorError(MailboxError),
}

impl Display for ErrorType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ErrorType::ActorError(e) => write!(f, "Actor error: {}", e),
        }
    }
}

impl std::error::Error for ErrorType {}

/// ========== Validations ==========

pub fn validate_regex_pattern(patterns: &Vec<Pattern>) -> Result<(), ValidationError> {
    for pattern in patterns {
        match pattern {
            Pattern::RegexPattern(rp) => {
                if !is_valid_regex(rp) {
                    return Err(ValidationError::new("invalid_regex"));
                }
            }
            _ => return Err(ValidationError::new("unsupported_pattern_type")),
        }
    }
    Ok(())
}

pub fn is_valid_regex(regex: &RegexPattern) -> bool {
    Regex::new(&regex.pattern_string).is_ok()
}

/// ========== Handlers ==========

#[utoipa::path(
    post,
    path = "/pattern",
    request_body(content = RegexRequest, description = "Submit a new regex pattern for ingestion"),
    responses(
        (status = 200, description = "Pattern accepted and dispatched"),
        (status = 400, description = "Validation or JSON error"),
        (status = 404, description = "Flight not found in registry"),
        (status = 409, description = "Flight does not exist")
    ),
    tag = "Patterns"
)]
pub fn submit_new_pattern_factory() -> Resource {
    web::resource("/pattern").route(web::post().to(submit_new_pattern::<ProdInjestRegistry>))
}

pub async fn submit_new_pattern<R>(
    data: Data<Arc<R>>,
    req: web::Json<RegexRequest>,
) -> impl Responder
where
    R: Registry + Send + Sync + 'static,
    R::Broadcaster: Actor + Handler<RegexRequest>,
    R::FlightRegistry: Actor + Handler<CheckFlight>,
    <R::Broadcaster as Actor>::Context: ToEnvelope<R::Broadcaster, RegexRequest>,
    <R::FlightRegistry as Actor>::Context: ToEnvelope<R::FlightRegistry, CheckFlight>,
{
    if let Err(validation_errors) = validate_regex_pattern(&req.pattern) {
        return HttpResponse::BadRequest().json(validation_errors);
    }

    let regex_request = req.into_inner();
    let team_id = regex_request.tenant.clone();
    let flight = regex_request.flight_id.clone();
    let flight_registry_actor = data.get_flight_registry_actor();

    match check_if_flight_exists(flight_registry_actor, team_id, flight.clone()).await {
        Ok(true) => {
            data.get_broadcaster_actor().do_send(regex_request);
            HttpResponse::Ok().json(format!("Regex submitted for {}", flight))
        }
        Ok(false) => HttpResponse::Conflict().json(format!(
            "Regex could not be submitted, Flight {} does not exist",
            flight
        )),
        Err(e) => HttpResponse::NotFound().json(format!("Flight {} not found - {}", flight, e)),
    }
}

/// ========== Utility Functions ==========

async fn check_if_flight_exists<F>(
    flight_registry_actor: Addr<F>,
    team_id: String,
    flight: String,
) -> Result<bool, std::io::Error>
where
    F: Actor + Handler<CheckFlight>,
    <F as Actor>::Context: ToEnvelope<F, CheckFlight>,
{
    flight_registry_actor
        .send(CheckFlight { team_id, flight })
        .await
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, format!("Actor error: {}", e)))?
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, format!("Handler error: {}", e)))
}

/// ========== List Flights ==========

#[utoipa::path(
    get,
    path = "/list-flights/{team_id}",
    responses(
        (status = 200, description = "List of flights", body = FlightsList),
        (status = 404, description = "Team ID not found")
    ),
    tag = "Flights"
)]
pub fn get_all_flights_factory() -> Resource {
    web::resource("/list-flights/{team_id}").route(web::get().to(fetch_flights::<ProdInjestRegistry>))
}

#[derive(Serialize, ToSchema)]
pub struct FlightsList {
    #[schema(example = json!(["batch_1", "batch_2", "batch_3"]))]
    flights: HashSet<String>,
}

pub async fn fetch_flights<R>(
    path: Path<String>,
    data: Data<Arc<R>>,
) -> impl Responder
where
    R: Registry + Send + Sync + 'static,
    R::FlightRegistry: Actor + Handler<ListFlights>,
    <R::FlightRegistry as Actor>::Context: ToEnvelope<R::FlightRegistry, ListFlights>,
{
    let team_id = path.into_inner();
    let actor = data.get_flight_registry_actor();

    match actor.send(ListFlights { team_id: team_id.clone() }).await {
        Ok(Ok(flights)) => {
            info!("Flights for {}: {:?}", team_id, flights);
            HttpResponse::Ok().json(FlightsList { flights })
        }
        Ok(Err(e)) => {
            warn!("Failed fetching flights: {}", e);
            HttpResponse::Ok().json("{\"Error\": \"Failed fetching flights\"}")
        }
        Err(e) => {
            warn!("Actor error for {}: {}", team_id, e);
            HttpResponse::NotFound().finish()
        }
    }
}
