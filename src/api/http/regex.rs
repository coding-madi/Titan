use crate::platform::actor_factory::InjestSystem;
use actix::{Addr, MailboxError, Message};
use actix_web::web::{Data, Path};
use actix_web::{HttpResponse, Resource, Responder, web};
use serde_derive::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fmt::{Debug, Display, Error, Formatter};
use std::sync::Arc;
use validator::{Validate, ValidationError, ValidationErrors};

#[derive(Debug, Serialize, Deserialize, Validate, Clone, ToSchema)]
pub struct RegexRequest {
    #[validate(length(min = 3, message = "Name must be greater than 3 chars"))]
    pub name: String,
    #[schema(example = "team-123")]
    pub tenant: String,
    #[schema(example = "flight-abc")]
    pub flight_id: String,
    pub log_group: String,
    #[validate(custom(
        function = "validate_regex_pattern",
        message = "The grok pattern is invalid"
    ))]
    #[schema(
        example = json!([
            { "regex": ".*ERROR.*" },
            { "regex": "^WARN.*" }
        ])
    )]
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
    override_field: Option<String>,
    field: String,
    #[schema(example = ".*ERROR.*")]
    pattern_string: String,
}

impl Message for RegexRequest {
    type Result = Result<(), ValidationErrors>;
}

enum ErrorType {
    ActorError(MailboxError),
}

impl Into<std::fmt::Error> for ErrorType {
    fn into(self) -> Error {
        todo!()
    }
}

impl Debug for ErrorType {
    fn fmt(&self, _f: &mut Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}

impl Display for ErrorType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ActorError(e) => write!(f, "Actor error: {}", e),
        }
    }
}

impl std::error::Error for ErrorType {}

#[utoipa::path(
    post,
    path = "/pattern",
    request_body(
        content = RegexRequest,
        description = "Submit a new regex pattern for ingestion"
    ),
    responses(
        (status = 200, description = "Pattern accepted and dispatched"),
        (status = 400, description = "Invalid regex pattern", body = String, example = "Invalid regex syntax: unclosed group"),
        (status = 400, description = "Json deserialize error:", body = String, example = "Json deserialize error: missing field `type` at line 8 column 5"),
        (status = 404, description = "Flight not found in registry")
    ),
    tag = "Patterns"
)]
pub fn submit_new_pattern_factory() -> Resource {
    web::resource("/pattern").route(web::post().to(submit_new_pattern))
}

pub async fn submit_new_pattern(
    data: Data<Arc<dyn InjestSystem>>,
    req: web::Json<RegexRequest>,
) -> impl Responder {
    match validate_regex_pattern(&req.pattern) {
        Ok(_) => {
            let regex_request = req.into_inner();
            let team_id = regex_request.tenant.clone();
            let flight = regex_request.flight_id.clone();
            let flight_registry_actor = data.get_flight_registry_actor();

            // 1. Check if the flight exists
            match check_if_flight_exists(flight_registry_actor, team_id, flight.clone()).await {
                Ok(is_flight_exists) => {
                    if is_flight_exists {
                        data.get_broadcaster_actor().do_send(regex_request);
                        HttpResponse::Ok().json(format!("Regex submitted for {}", flight))
                    } else {
                        HttpResponse::Conflict().json(format!(
                            "Regex could not be submitted, Flight {} does not exist",
                            flight
                        ))
                    }
                }
                Err(e) => {
                    HttpResponse::NotFound().json(format!("Flight {} not found - {}", flight, e))
                }
            }
        }
        Err(validation_errors) => HttpResponse::BadRequest().json(validation_errors),
    }
}

pub fn validate_regex_pattern(patterns: &Vec<Pattern>) -> Result<(), ValidationError> {
    for pattern in patterns {
        match pattern {
            Pattern::RegexPattern(regex_pattern) => {
                if !is_valid_regex(&regex_pattern) {
                    return Err(ValidationError::new("invalid_regex"));
                }
            }
            _ => return Err(ValidationError::new("invalid_pattern_type")),
        }
    }
    Ok(())
}

async fn check_if_flight_exists(
    flight_registry_actor: Addr<FlightRegistry>,
    team_id: String,
    flight: String,
) -> Result<bool, std::io::Error> {
    let exists = flight_registry_actor
        .send(CheckFlight {
            team_id: team_id.clone(),
            flight: flight.clone(),
        })
        .await
        .map_err(|e| ErrorType::ActorError(e))
        .unwrap();
    exists
}



#[utoipa::path(
    get,
    path = "/list-flights/{team_id}",
    responses(
        (status = 200, description = "Return a success message", body = FlightsList),
    )
)]
pub fn get_all_flights_factory() -> Resource {
    web::resource("/list-flights/{team_id}").route(web::get().to(fetch_flights))
}

#[derive(Serialize, ToSchema)]
pub struct FlightsList {
    #[schema(example = json!(["batch_1", "batch_2", "batch_3"]))]
    flights: HashSet<String>,
}

async fn fetch_flights(path: Path<String>, data: Data<Arc<dyn InjestSystem>>) -> impl Responder {
    let team_id = path.into_inner();
    let flight_list = data
        .get_flight_registry_actor()
        .send(ListFlights {
            team_id: team_id.clone(),
        })
        .await
        .map_err(|e| {
            warn!("Actor error: {}", e);
            ActorError(e)
        });

    match flight_list {
        Ok(res) => match res {
            Ok(flights) => {
                info!("Flights: {:?}", flights);
                HttpResponse::Ok().json(FlightsList { flights })
            }
            Err(_) => {
                warn!("Error fetching flights");
                HttpResponse::Ok().json("{\"Error\" : \"Error fetching\"}")
            }
        },
        Err(_) => HttpResponse::NotFound().finish(),
    }
}

use crate::application::actors::flight_registry::{CheckFlight, FlightRegistry, ListFlights};
use regex::Regex;
use tracing::{info, warn};
use utoipa::ToSchema;

pub fn is_valid_regex(regex: &RegexPattern) -> bool {
    Regex::new(&regex.pattern_string).is_ok()
}

use crate::api::http::regex::ErrorType::ActorError;
