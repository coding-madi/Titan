use crate::application::actors::parser::handlers::submit_regex::SubmitRegexRequest;
use crate::core::error::exception::regex::RegexError;
use crate::core::parser::messages::parser::Pattern;
use crate::platform::registry::ParserActor;
use actix::{ActorFutureExt, Handler, Message, ResponseActFuture, WrapFuture};
use serde_json::Value;

#[derive(Message, Clone, Debug)]
#[rtype(result = "Result<Value, RegexError>")]
pub struct TryParsingRegex {
    pub name: String,
    pub flight_name: String,
    pub log_group: String,
    pub pattern: Vec<Pattern>,
    pub try_parsing: bool,
}

impl TryParsingRegex {
    pub fn new(regex_request: &SubmitRegexRequest) -> Self {
        Self {
            name: regex_request.name.clone(),
            flight_name: regex_request.flight_id.clone(),
            log_group: regex_request.log_group.clone(),
            pattern: regex_request.pattern.clone(),
            try_parsing: true,
        }
    }
}

impl Handler<TryParsingRegex> for ParserActor {
    type Result = ResponseActFuture<Self, Result<Value, RegexError>>;

    fn handle(&mut self, msg: TryParsingRegex, _ctx: &mut Self::Context) -> Self::Result {
        let futures = self.parser_service.try_parse(msg);
        futures.into_actor(self).boxed_local()
    }
}
