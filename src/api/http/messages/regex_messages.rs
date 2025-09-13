use crate::application::actors::parser_actor::TryParsingRegex;
use crate::core::error::exception::regex::RegexError;
use crate::core::utils::regex::validate_regex_pattern;
use actix::Message;
use serde_derive::{Deserialize, Serialize};
use serde_json::Value;
use utoipa::ToSchema;
use validator::Validate;

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

impl From<TryParsingRegex> for RegexHttpRequest {
    fn from(value: TryParsingRegex) -> Self {
        Self {
            name: value.name,
            flight_id: value.flight_name,
            log_group: value.log_group,
            pattern: value.pattern.into_iter().map(|p| p.into()).collect(),
            try_parse: value.try_parsing,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
#[serde(tag = "type", content = "value")]
pub enum Pattern {
    RegexPattern(RegexPattern),
    GrokPattern(GrokPattern),
}

impl From<crate::core::parser::messages::parser::Pattern> for Pattern {
    fn from(value: crate::core::parser::messages::parser::Pattern) -> Self {
        match value {
            crate::core::parser::messages::parser::Pattern::RegexPattern(p) => {
                Pattern::RegexPattern(p.into())
            }
            crate::core::parser::messages::parser::Pattern::GrokPattern(p) => {
                unimplemented!()
            }
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct RegexPattern {
    pub override_field: Option<String>,
    pub field: String,
    #[schema(example = ".*ERROR.*")]
    pub pattern_string: String,
}

impl From<crate::core::parser::messages::parser::RegexPattern> for RegexPattern {
    fn from(value: crate::core::parser::messages::parser::RegexPattern) -> Self {
        Self {
            override_field: value.override_field,
            field: value.field,
            pattern_string: value.pattern_string,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct GrokPattern {
    pub override_field: Option<String>,
    pub field: String,
    #[schema(example = ".*ERROR.*")]
    pub pattern_string: String,
}
