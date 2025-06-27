use actix_web::{HttpResponse, Resource, Responder, web};

pub async fn health_endpoint() -> impl Responder {
    HttpResponse::Ok().json("{\"status\": \"OK\"}")
}

pub fn get_health_endpoint_factory() -> Resource {
    web::resource("/health").route(web::get().to(health_endpoint))
}
