use std::sync::Arc;
use actix::Message;
use crate::api::http::health::health_endpoint;
use actix_web::{HttpResponse, Resource, Responder, web};
use actix_web::web::Data;
use serde_derive::{Deserialize, Serialize};
use crate::platform::actor_factory::InjestSystem;

#[derive(Debug, Serialize, Deserialize)]
pub struct RegexRequest {
    pub name: String,
    pub tenant: String,
    pub service_id: String,
    pub log_name: String,
    pub pattern: Vec<Pattern>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", content = "value")]
pub enum Pattern {
    Regex(Regex),
    Grok(Grok),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Regex {
    override_field: Option<String>,
    field: String,
    pattern_string: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Grok {
    override_field: Option<String>,
    field: String,
    pattern_string: String,
}

impl Message for RegexRequest {
    type Result = Result<(), String>;
}

pub async fn submit_new_pattern(data: Data<Arc<dyn InjestSystem>>,req: web::Json<RegexRequest>) -> impl Responder {
    println!("{:?}", req.pattern);
    let regex_rule = RegexRequest {
        name: req.name.to_string(),
        tenant: req.tenant.to_string(),
        service_id: req.service_id.to_string(),
        log_name: req.log_name.to_string(),
        pattern: vec![Pattern::Grok(
            Grok {
                override_field: None,
                field: "x".to_string(),
                pattern_string: ".*".to_string(),
            }
        )],
    };
    data.get_broadcaster_actor().do_send(regex_rule);
    HttpResponse::Ok().json("{\"status\": \"OK\"}")
}

pub fn submit_new_pattern_factory() -> Resource {
    web::resource("/pattern").route(web::post().to(submit_new_pattern))
}
