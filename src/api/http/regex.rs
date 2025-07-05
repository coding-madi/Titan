use crate::platform::actor_factory::InjestSystem;
use actix::Message;
use actix_web::http::StatusCode;
use actix_web::web::{Data, Path};
use actix_web::{HttpResponse, Resource, Responder, web};
use serde_derive::{Deserialize, Serialize};
use std::collections::HashSet;
use std::sync::Arc;
use validator::{Validate, ValidationError, ValidationErrors};

#[derive(Debug, Serialize, Deserialize, Validate, Clone)]
pub struct RegexRequest {
    #[validate(length(min = 3, message = "Name must be greater than 3 chars"))]
    pub name: String,
    pub tenant: String,
    pub flight_id: String,
    pub log_group: String,
    #[validate(custom(
        function = "validate_regex_pattern",
        message = "The grok pattern is invalid"
    ))]
    pub pattern: Vec<Pattern>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "type", content = "value")]
pub enum Pattern {
    RegexPattern(RegexPattern),
    GrokPattern(GrokPattern),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RegexPattern {
    pub override_field: Option<String>,
    pub field: String,
    pub pattern_string: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GrokPattern {
    override_field: Option<String>,
    field: String,
    pattern_string: String,
}

impl Message for RegexRequest {
    type Result = Result<(), ValidationErrors>;
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
            // 1. Check if the flight exists
            match data
                .get_flight_registry_actor()
                .send(CheckFlight {
                    team_id: team_id.clone(),
                    flight: flight.clone(),
                })
                .await
                .unwrap()
            {
                Ok(exists) => {
                    if !exists {
                        warn!("Flight {} does not exist in the registry.", &flight);
                        return HttpResponse::build(StatusCode::NOT_FOUND).finish();
                    }
                    data.get_broadcaster_actor().do_send(regex_request);
                    HttpResponse::Ok().finish()
                }
                Err(_) => {
                    warn!(
                        "Team - {} and Flight {} does not exist in the registry.",
                        &flight, &team_id
                    );
                    HttpResponse::build(StatusCode::NOT_FOUND).finish()
                }
            }
        }
        Err(e) => HttpResponse::BadRequest().json(e),
    }
}

pub fn submit_new_pattern_factory() -> Resource {
    web::resource("/pattern").route(web::post().to(submit_new_pattern))
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

pub fn get_all_flights_factory() -> Resource {
    web::resource("/list-flights/{team_id}").route(web::get().to(fetch_flights))
}

#[derive(Serialize)]
struct FlightsList {
    flights: HashSet<String>,
}

async fn fetch_flights(path: Path<String>, data: Data<Arc<dyn InjestSystem>>) -> impl Responder {
    let team_id = path.into_inner();
    let x = FlightsList {
        flights: data
            .get_flight_registry_actor()
            .send(ListFlights { team_id })
            .await
            .unwrap()
            .unwrap(),
    };
    HttpResponse::Ok().json(x)
}

use crate::application::actors::flight_registry::{CheckFlight, ListFlights};
use regex::Regex;
use tracing::warn;

pub fn is_valid_regex(regex: &RegexPattern) -> bool {
    Regex::new(&regex.pattern_string).is_ok()
}
