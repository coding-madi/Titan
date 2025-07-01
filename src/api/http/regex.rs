use crate::platform::actor_factory::InjestSystem;
use actix::Message;
use actix_web::web::Data;
use actix_web::{HttpResponse, Resource, Responder, web};
use serde_derive::{Deserialize, Serialize};
use std::sync::Arc;
use validator::{Validate, ValidationError};

#[derive(Debug, Serialize, Deserialize, Validate)]
pub struct RegexRequest {
    #[validate(length(min = 3, message = "Name must be greater than 3 chars"))]
    pub name: String,
    pub tenant: String,
    pub service_id: String,
    pub log_name: String,
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
    override_field: Option<String>,
    field: String,
    pattern_string: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GrokPattern {
    override_field: Option<String>,
    field: String,
    pattern_string: String,
}

impl Message for RegexRequest {
    type Result = Result<(), String>;
}

pub async fn submit_new_pattern(
    data: Data<Arc<dyn InjestSystem>>,
    req: web::Json<RegexRequest>,
) -> impl Responder {
    let pattern = req.pattern.iter();

    let pattern_vector = pattern.map(|p| p.clone()).collect();

    let regex_rule = RegexRequest {
        name: req.name.to_string(),
        tenant: req.tenant.to_string(),
        service_id: req.service_id.to_string(),
        log_name: req.log_name.to_string(),
        pattern: pattern_vector,
    };
    data.get_broadcaster_actor().do_send(regex_rule);
    HttpResponse::Ok().json("{\"status\": \"OK\"}")
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

use regex::Regex;
pub fn is_valid_regex(regex: &RegexPattern) -> bool {
    Regex::new(&regex.pattern_string).is_ok()
}
