use actix_web::{HttpResponse, Resource, Responder, web};
use serde_derive::Serialize;
use serde_json::json;

#[derive(Serialize, ToSchema)]
struct Health {
    status: String,
}

#[derive(Serialize)]
enum Status {
    OK,
}

use std::fmt::Display;
impl Display for Status {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Status::OK => write!(f, "OK"),
        }
    }
}

pub async fn health_endpoint() -> impl Responder {
    HttpResponse::Ok().json(json!(Health {
        status: Status::OK.to_string()
    }))
}

use utoipa::ToSchema;
#[utoipa::path(
    get,
    path = "/health",
    responses(
        (status = 200, description = "Return a success message", body = Health, example = "{\"status\": \"OK\"}"),
        (status = 500, description = "Internal server error", body = Health, example =  "{\"status\": \"FAILED\"}"),
    )
)]
pub fn get_health_endpoint_factory() -> Resource {
    web::resource("/health").route(web::get().to(health_endpoint))
}
