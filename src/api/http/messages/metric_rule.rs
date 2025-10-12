use crate::api::http::messages::validations::validate_flight_name;
use crate::api::http::messages::validations::validate_script;
use actix_web::FromRequest;
use serde_derive::{Deserialize, Serialize};
use validator::Validate;

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct MetricRule {
    #[validate(custom(function = "validate_flight_name"))]
    flight_name: String,
    #[validate(custom(function = "validate_script"))]
    script: String,
}
