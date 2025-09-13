use crate::api::http::messages::regex_messages::RegexHttpRequest;
use crate::application::service::parser_service::ParserService;
use crate::core::error::exception::regex::RegexError;
use actix::{
    Actor, ActorFutureExt, Addr, AsyncContext, Context, ContextFutureSpawner, Handler, Message,
    ResponseActFuture, WrapFuture,
};
use arrow::datatypes::Schema;
use arrow_array::{Array, BooleanArray, StringArray};
use regex::Regex;
use serde_json::Value;
use std::collections::HashMap;
use tracing::trace;

use crate::application::actors::broadcast_actor::RecordBatchWrapper;

#[derive(Message, Clone, Debug)]
#[rtype(result = "()")]
pub enum ParserActorAddr {
    Real(Addr<ParserActor>),
    #[cfg(test)]
    Mock(Addr<MockParsingActor>),
    Empty,
}

#[derive(Clone)]
pub struct ParserActor {
    pub parser_service: ParserService,
}

impl ParserActor {
    pub fn new(parser_service: ParserService) -> ParserActor {
        Self { parser_service }
    }
}

impl Actor for ParserActor {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        let registry = self.parser_service.get_registry_address();
        registry.do_send(ParserActorAddr::Real(ctx.address()));
        trace!("Parser actor started");
    }
}

#[derive(Message, Debug, Clone)]
#[rtype(result = "Result<Value, RegexError>")]
pub struct SubmitRegexRequest {
    name: String,
    flight_id: String,
    log_group: String,
    pattern: Vec<Pattern>,
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

use crate::core::parser::messages::parser::Pattern;
#[cfg(test)]
use crate::platform::registry::Registry;
use arrow_array::builder::BooleanBuilder;
use log::info;

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

// Handle incoming data for parsing
impl Handler<RecordBatchWrapper> for ParserActor {
    type Result = ();

    // TODO
    fn handle(&mut self, record: RecordBatchWrapper, _ctx: &mut Self::Context) -> Self::Result {
        let fut = self.parser_service.handle_message(record);
        fut.into_actor(self).spawn(_ctx);
    }
}

#[allow(dead_code)]
fn fast_regex_match(text_array: &StringArray, pattern: &str) -> Result<BooleanArray, String> {
    let regex = Regex::new(pattern).map_err(|e| format!("Invalid regex: {e}"))?;
    let mut builder = BooleanBuilder::new();

    for i in 0..text_array.len() {
        if text_array.is_null(i) {
            builder.append_null();
        } else {
            builder.append_value(regex.is_match(text_array.value(i)));
        }
    }

    Ok(builder.finish())
}

#[allow(dead_code)]
fn get_flight_and_schemas(_team_id: &String) -> HashMap<String, Schema> {
    unimplemented!()
}

#[cfg(test)]
#[derive(Clone)]
pub struct MockParsingActor {
    pub registry_address: Addr<Registry>,
    pub data: Vec<RecordBatchWrapper>,
    pub regex: Vec<SubmitRegexRequest>,
}

#[cfg(test)]
impl MockParsingActor {
    pub fn new(registry_address: Addr<Registry>) -> Self {
        Self {
            registry_address,
            data: vec![],
            regex: vec![],
        }
    }
}

#[cfg(test)]
impl Actor for MockParsingActor {
    type Context = Context<Self>;

    fn started(&mut self, _ctx: &mut Self::Context) {}
}

#[cfg(test)]
impl Handler<RecordBatchWrapper> for MockParsingActor {
    type Result = ();
    fn handle(&mut self, _: RecordBatchWrapper, _: &mut Self::Context) -> Self::Result {
        todo!()
    }
}

#[cfg(test)]
#[derive(Message, Clone)]
#[rtype(result = "Vec<SubmitRegexRequest>")]
pub struct DumpRegex;

#[cfg(test)]
impl Handler<DumpRegex> for MockParsingActor {
    type Result = Vec<SubmitRegexRequest>;

    fn handle(&mut self, _msg: DumpRegex, _ctx: &mut Self::Context) -> Self::Result {
        self.regex.clone()
    }
}
