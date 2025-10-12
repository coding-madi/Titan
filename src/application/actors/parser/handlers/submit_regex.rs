use crate::api::http::messages::regex_messages::RegexHttpRequest;
use crate::core::error::exception::regex::RegexError;
use crate::core::parser::messages::parser::Pattern;
use crate::platform::registry::ParserActor;
use actix::{Handler, Message};
use serde_json::Value;
use tracing::info;

#[derive(Message, Debug, Clone)]
#[rtype(result = "Result<Value, RegexError>")]
pub struct SubmitRegexRequest {
    pub(crate) name: String,
    pub(crate) flight_id: String,
    pub(crate) log_group: String,
    pub(crate) pattern: Vec<Pattern>,
    try_parse: bool,
}

impl SubmitRegexRequest {
    pub fn new(regex_request: &RegexHttpRequest) -> Self {
        Self {
            name: regex_request.name.clone(),
            flight_id: regex_request.flight_id.clone(),
            log_group: regex_request.log_group.clone(),
            pattern: regex_request
                .pattern
                .clone()
                .into_iter()
                .map(|p| p.into())
                .collect(),
            try_parse: regex_request.try_parse.clone(),
        }
    }

    pub fn get_flight_id(&self) -> &str {
        &self.flight_id
    }

    pub fn is_try_parse(&self) -> bool {
        self.try_parse
    }
}

// Handle regex rule registration
impl Handler<SubmitRegexRequest> for ParserActor {
    type Result = Result<Value, RegexError>;

    // TODO - compile the regex and save
    fn handle(&mut self, msg: SubmitRegexRequest, _ctx: &mut Self::Context) -> Self::Result {
        info!(
            "Received regex rule submission: {} for flight: {}",
            &msg.name, &msg.flight_id
        );
        self.parser_service
            .save_pattern_in_state(msg.log_group.clone(), msg.pattern);
        Ok(Value::String(format!(
            "successfully submitted regex rule - {} for the flight stream - {}",
            &msg.name, &msg.flight_id
        )))
    }
}
