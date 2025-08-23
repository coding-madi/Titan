use crate::core::error::exception::regex::RegexError;
use actix::Message;
use serde_derive::{Deserialize, Serialize};
use serde_json::Value;
use utoipa::ToSchema;
use validator::Validate;
use crate::core::utils::regex::validate_regex_pattern;

#[derive(Debug, Serialize, Deserialize, Validate, Clone, ToSchema, Message)]
#[rtype(result = "Result<Value, RegexError>")]
pub struct RegexHttpRequest {
    #[validate(length(min = 3, message = "Name must be greater than 3 chars"))]
    pub name: String,
    #[schema(example = "flight-abc")]
    pub flight_id: String,
    pub log_group: String,
    #[validate(custom(function = "validate_regex_pattern"))]
    pub pattern: Vec<Pattern>,
    pub try_parse: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
#[serde(tag = "type", content = "value")]
pub enum Pattern {
    RegexPattern(RegexPattern),
    GrokPattern(GrokPattern),
}

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct RegexPattern {
    pub override_field: Option<String>,
    pub field: String,
    #[schema(example = ".*ERROR.*")]
    pub pattern_string: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct GrokPattern {
    pub override_field: Option<String>,
    pub field: String,
    #[schema(example = ".*ERROR.*")]
    pub pattern_string: String,
}
