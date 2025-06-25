use actix_web::{HttpResponse, Responder};

pub async fn health_endpoint() -> impl Responder {
    HttpResponse::Ok().json("{\"status\": \"OK\"}")
}
