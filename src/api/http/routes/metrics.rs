use crate::api::http::messages::metric_message::MetricRule;
use crate::platform::registry::Registry;
use actix::Addr;
use actix_web::web::Data;
use actix_web::{HttpResponse, Resource, web};
use std::sync::Arc;
use validator::Validate;

pub fn submit_new_metric_rule_factory() -> Resource {
    web::resource("/metric-rule").route(web::post().to(submit_new_metric_rule))
}

async fn submit_new_metric_rule(
    registry_actor: Data<Arc<Addr<Registry>>>,
    rule: web::Json<MetricRule>,
) -> HttpResponse {
    if let Err(error) = rule.validate() {
        return HttpResponse::BadRequest().json(error);
    }
    let rule = rule.into_inner(); // now you have the MetricRule
    // add it to your actor or in-memory registry
    HttpResponse::Ok().json(rule) // echo back
}
