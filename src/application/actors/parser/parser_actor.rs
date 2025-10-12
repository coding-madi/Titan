use crate::application::service::parser_service::ParserService;
use actix::{
    Actor, Addr, AsyncContext, Context, ContextFutureSpawner, Handler, Message, ResponseActFuture,
    WrapFuture,
};
use arrow::datatypes::Schema;
use arrow_array::{Array, BooleanArray, StringArray};
use regex::Regex;
use std::collections::HashMap;

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
    pub flight_name: String,
    pub parser_service: ParserService,
    pub rhai_actor_addr: Addr<RhaiActor>,
}

impl ParserActor {
    pub fn new(
        flight_name: String,
        parser_service: ParserService,
        rhai_actor_addr: Addr<RhaiActor>,
    ) -> ParserActor {
        Self {
            flight_name,
            parser_service,
            rhai_actor_addr,
        }
    }
}

impl Actor for ParserActor {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        let registry = self.parser_service.get_registry_address();
        registry.do_send(ParserActorReady {
            flight_name: self.flight_name.clone(),
            parser_actor_addr: ParserActorAddr::Real(ctx.address()),
        });
        info!("Parser actor started");
    }
}

#[cfg(test)]
use crate::application::actors::broadcaster::broadcast_actor::RecordBatchWrapper;
use crate::application::actors::messages::registeration::ParserActorReady;
pub(crate) use crate::application::actors::parser::handlers::submit_regex::SubmitRegexRequest;
use crate::application::actors::rhai::rhai_actor::RhaiActor;
#[cfg(test)]
use crate::platform::registry::Registry;
use arrow_array::builder::BooleanBuilder;
use tracing::info;

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
