use crate::api::http::messages::regex_messages::RegexHttpRequest;
use crate::application::actors::flight_registry::{
    CheckFlight, FlightRegistryActorWrapped, ListFlights,
};
use crate::application::actors::parser::SubmitRegexRequest;
use crate::core::error::exception::actor_errors::ErrorType;
use crate::core::utils::flight::validate_if_flight_exists;
use crate::core::utils::regex::validate_patterns;
use crate::platform::registry::{
    FetchBroadcastActor, FetchFlightRegistryActor, Registry,
};
use actix::dev::ToEnvelope;
use actix::{Actor, Addr, Handler, MailboxError};
use actix_web::web::{Data, Path};
use actix_web::{HttpResponse, Resource, Responder, web};
use futures_util::SinkExt;
use serde_derive::Serialize;
use std::collections::HashSet;
use std::sync::Arc;
use tracing::{info, warn};
use utoipa::ToSchema;
use validator::Validate;

/// ========== Errors ==========

impl std::error::Error for ErrorType {}

/// ========== Handlers ==========

#[utoipa::path(
    post,
    path = "/pattern",
    request_body(content = RegexHttpRequest, description = "Submit a new regex pattern for ingestion"),
    responses(
        (status = 200, description = "Pattern accepted and dispatched"),
        (status = 400, description = "Validation or JSON error"),
        (status = 404, description = "Flight not found in registry"),
        (status = 409, description = "Flight does not exist")
    ),
    tag = "Patterns"
)]
pub fn submit_new_pattern_factory() -> Resource {
    web::resource("/pattern").route(web::post().to(submit_new_pattern))
}

// TODO -
// 1. Validate regex pattern
// 2. extract flight name from request
// 3. Fetch registry
// Validate if flight exists
// Fetch broadcast registry actor by flight name
// Send regex to the actor
// return the response
pub async fn submit_new_pattern(
    registry_actor: Data<Arc<Addr<Registry>>>,
    req: web::Json<RegexHttpRequest>,
) -> impl Responder {
    if let Err(e) = req.validate() {
        return HttpResponse::BadRequest().json(e);
    }

    // 1. Validate regex pattern
    if let Err(validation_errors) = validate_patterns(&req.pattern) {
        return HttpResponse::BadRequest().json(validation_errors);
    }

    // 2. extract flight name from request
    let regex_request = req.into_inner();
    let flight = regex_request.flight_id.clone();

    // 3. Fetch registry
    let registry = registry_actor.get_ref().as_ref().clone();
    // Validate of flight exists
    if let Ok(is_flight_exists) = validate_if_flight_exists(&flight, registry.clone()).await {
        if !is_flight_exists {
            HttpResponse::NotFound().json(format!("Flight {} does not exist", flight))
        } else {
            // Send data to broadcast actor for handling
            submit_new_pattern_to_broadcast_actor(registry, &regex_request, flight).await
        }
    } else {
        HttpResponse::InternalServerError().json("Internal server error".to_string())
    }
}

async fn submit_new_pattern_to_broadcast_actor(
    registry: Addr<Registry>,
    regex_request: &RegexHttpRequest,
    flight: String,
) -> HttpResponse {
    if let Ok(broadcast_actor_wrapper) = registry
        .send(FetchBroadcastActor {
            flight_name: flight.to_string(),
        })
        .await
        .unwrap()
    {
        match broadcast_actor_wrapper
            .regex_request(SubmitRegexRequest::new(regex_request))
            .await
        {
            Ok(response) => HttpResponse::Ok().json(response),
            Err(e) => HttpResponse::NotFound().json(format!("Internal server error - {}", e)),
        }
    } else {
        HttpResponse::InternalServerError().json("Internal server error".to_string())
    }
}

async fn check_if_flight_exists<F>(
    flight_registry_actor: Addr<F>,
    flight: String,
) -> Result<bool, MailboxError>
where
    F: Actor + Handler<CheckFlight>,
    <F as Actor>::Context: ToEnvelope<F, CheckFlight>,
{
    flight_registry_actor.send(CheckFlight { flight }).await
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
    web::resource("/list-flights/{team_id}").route(web::get().to(fetch_flights))
}

#[derive(Serialize, ToSchema)]
pub struct FlightsList {
    #[schema(example = json!(["batch_1", "batch_2", "batch_3"]))]
    flights: HashSet<String>,
}

pub async fn fetch_flights(path: Path<String>, data: Data<Arc<Addr<Registry>>>) -> impl Responder {
    let team_id = path.into_inner();
    let actor = data.send(FetchFlightRegistryActor).await.unwrap().unwrap();

    match actor {
        FlightRegistryActorWrapped::Real(flight_registry) => {
            let x = flight_registry
                .send(ListFlights {
                    team_id: team_id.clone(),
                })
                .await;
            match x {
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
        #[cfg(test)]
        _ => {
            unimplemented!()
        }
        _ => HttpResponse::NotFound().finish(),
    }
}
