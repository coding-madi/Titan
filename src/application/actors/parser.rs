use actix::{
    Actor, Addr, AsyncContext, Context, ContextFutureSpawner, Handler, Message, WrapFuture,
};
use arrow::datatypes::Schema;
use arrow_array::{Array, BooleanArray, StringArray};
use std::collections::HashMap;
use validator::ValidationErrors;

pub(crate) use crate::api::http::regex::{Pattern, RegexRequest};
use crate::application::actors::broadcast::RecordBatchWrapper;

#[derive(Message, Clone)]
#[rtype(result = "()")]
pub enum ParserActorAddr {
    Real(Vec<Addr<ParsingActor>>),
    #[cfg(test)]
    Mock(Vec<Addr<MockParsingActor>>),
    Empty,
}

#[derive(Clone)]
pub struct ParsingActor {
    pub patterns: HashMap<String, Vec<Pattern>>, // flight_id → patterns
    pub schema: HashMap<String, Schema>,         // service_id → schema
    pub registry_address: Addr<Registry>,
}

impl ParsingActor {
    pub fn default(registry_address: Addr<Registry>) -> Self {
        Self {
            patterns: HashMap::new(),
            schema: HashMap::new(),
            registry_address,
        }
    }

    pub fn new(team_id: String, registry_address: Addr<Registry>) -> Self {
        let patterns = get_patterns_from_database(&team_id);
        let schema = get_flight_and_schemas(&team_id);

        Self {
            patterns,
            schema,
            registry_address,
        }
    }
}

impl Actor for ParsingActor {
    type Context = Context<Self>;

    fn started(&mut self, _ctx: &mut Self::Context) {
        let registry_address = self.registry_address.clone();
        let address = _ctx.address();
        // let pool = settings_for_spawn.connection_pool().await;
        registry_address.do_send(ParserActorAddr::Real(vec![address]));
        trace!("ParsingActor started")
    }
}

// Handle regex rule registration
impl Handler<RegexRequest> for ParsingActor {
    type Result = Result<(), ValidationErrors>;

    fn handle(&mut self, msg: RegexRequest, _ctx: &mut Self::Context) -> Self::Result {
        println!("Received RegexRule in parser: {:?}", msg);
        self.patterns.insert(msg.flight_id, msg.pattern);
        Ok(())
    }
}

use arrow_array::builder::BooleanBuilder;
use futures_util::SinkExt;
use log::error;

// Handle incoming data for parsing
impl Handler<RecordBatchWrapper> for ParsingActor {
    type Result = ();

    fn handle(&mut self, record: RecordBatchWrapper, _ctx: &mut Self::Context) -> Self::Result {
        let service_id = &record.metadata.service_id;
        let parser = self.clone();
        let registry = self.registry_address.clone();
        let fut = async move {
            let registry_address = parser.registry_address.clone();
            let Ok(result) = registry_address.send(FetchWalActor).await else {
                error!("Failed to fetch WalActorAddr from Registry");
                return;
            };
            let Ok(address) = result else {
                error!("Failed to fetch WalActorAddr from Registry");
                return;
            };

            // Handle mock cases
            let Real(actor) = registry.send(FetchIcebergActor).await.unwrap().unwrap() else {
                return;
            };

            actor.send(record.clone()).await.unwrap();

            match address {
                WalActorAddr::Real(wal_actors) => {
                    wal_actors.do_send(record);
                }
                #[cfg(test)]
                WalActorAddr::Mock(wal_actors) => {
                    wal_actors.do_send(record);
                }
                _ => {}
            }
        };

        fut.into_actor(self).spawn(_ctx);
    }
}

use crate::application::actors::iceberg::IcebergActorAddr::Real;
use crate::application::actors::wal::WalActorAddr;
use crate::platform::registry::{FetchIcebergActor, FetchWalActor, Registry};
use regex::Regex;
use tracing::trace;

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
fn get_patterns_from_database(_team_id: &String) -> HashMap<String, Vec<Pattern>> {
    unimplemented!()
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
    pub regex: Vec<RegexRequest>,
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

    fn started(&mut self, _ctx: &mut Self::Context) {
        let registry_address = self.registry_address.clone();
        let address = _ctx.address();
        registry_address.do_send(ParserActorAddr::Mock(vec![address.clone()]));
        trace!("MockParsingActor started")
    }
}

#[cfg(test)]
impl Handler<RecordBatchWrapper> for MockParsingActor {
    type Result = ();
    fn handle(&mut self, _: RecordBatchWrapper, _: &mut Self::Context) -> Self::Result {
        todo!()
    }
}

#[cfg(test)]
impl Handler<RegexRequest> for MockParsingActor {
    type Result = Result<(), ValidationErrors>;
    fn handle(&mut self, regex: RegexRequest, _: &mut Self::Context) -> Self::Result {
        println!("Actor for parsing received RegexRule: {:?}", regex);
        self.regex.push(regex);
        Ok(())
    }
}

#[cfg(test)]
#[derive(Message, Clone)]
#[rtype(result = "Vec<RegexRequest>")]
pub struct DumpRegex;

#[cfg(test)]
impl Handler<DumpRegex> for MockParsingActor {
    type Result = Vec<RegexRequest>;

    fn handle(&mut self, msg: DumpRegex, ctx: &mut Self::Context) -> Self::Result {
        self.regex.clone()
    }
}
