use crate::api::http::messages::metric_message::MetricRule;
use crate::application::actors::rhai::handler::metric_rule_dto::MetricRuleDTO;
use crate::application::actors::rhai::rhai_actor::RhaiActorAddr;
#[cfg(test)]
use crate::application::actors::rhai::rhai_actor::RhaiActorAddr::Mock;
use crate::application::actors::rhai::rhai_actor::RhaiActorAddr::Real;
use crate::monitor::prometheus::registry::REGISTRY;
use crate::platform::registry::{FetchRhaiActor, Registry};
use actix::Addr;
use actix_web::web::{Data, Json};
use actix_web::{HttpResponse, Resource, web};
use futures_core::future::BoxFuture;
use futures_util::SinkExt;
use log::error;
use prometheus::{Encoder, TextEncoder};
use std::sync::Arc;

pub fn prometheus_metrics_factory() -> Resource {
    web::resource("/metrics").route(web::get().to(prometheus_metrics))
}

async fn prometheus_metrics(_registry_actor: Data<Arc<Addr<Registry>>>) -> HttpResponse {
    let metric_families = REGISTRY.gather();

    // Encode them in Prometheus text format
    let mut buffer = Vec::new();
    let encoder = TextEncoder::new();
    encoder.encode(&metric_families, &mut buffer).unwrap();

    // Return as UTF-8 string
    HttpResponse::Ok().body(String::from_utf8(buffer).unwrap())
}

pub fn rhai_metrics_factory() -> Resource {
    web::resource("/metric-rule").route(web::post().to(rhai_metrics))
}

async fn rhai_metrics(
    registry_actor: Data<Arc<Addr<Registry>>>,
    metric_rule: Json<MetricRule>,
) -> HttpResponse {
    let metric_rule = metric_rule.into_inner();
    let fetch_rhai_actor = FetchRhaiActor {
        flight_name: metric_rule.get_flight_name().to_string(),
    };
    let lookup_res = registry_actor.send(fetch_rhai_actor).await;

    let result = match lookup_res {
        Err(mailbox_err) => {
            error!("Actor had a fatal error: {:?}", mailbox_err);
            HttpResponse::ServiceUnavailable().body("Please file a bug!")
        }
        Ok(Err(rhai_err)) => {
            error!("Actor not found: {:?}", rhai_err);
            HttpResponse::NotFound().body("Actor not found")
        }
        Ok(Ok(rhai_actor_enum)) => {
            // We already have the actor enum — now we can perform the async work,
            // but we await it *here* so the boxed future doesn't capture non-Send types.
            match rhai_actor_enum {
                Real(actor) => {
                    // await here (we're inside async fn)
                    let metric_rule_dto = MetricRuleDTO::from(metric_rule.clone());
                    let send_res = actor.send(metric_rule_dto).await;
                    let resp = if send_res.is_ok() {
                        HttpResponse::Ok().json(metric_rule)
                    } else {
                        HttpResponse::InternalServerError().body("Failed to send rule")
                    };
                    resp
                }
                #[cfg(test)]
                Mock(_) => HttpResponse::NotImplemented().finish(),
                _ => HttpResponse::InternalServerError().finish(),
            }
        }
    };
    result
}
