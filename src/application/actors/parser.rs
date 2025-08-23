use actix::{Actor, ActorFutureExt, Addr, AsyncContext, Context, ContextFutureSpawner, Handler, MailboxError, Message, ResponseActFuture, WrapFuture};
use arrow::datatypes::Schema;
use arrow_array::{Array, BooleanArray, StringArray};
use std::collections::HashMap;
use validator::{Validate, ValidationErrors};

use crate::application::actors::broadcast::{BroadcastActorWrapper, RecordBatchWrapper};

#[derive(Message, Clone)]
#[rtype(result = "()")]
pub enum ParserActorAddr {
    Real(Addr<ParserActor>),
    #[cfg(test)]
    Mock(Addr<MockParsingActor>),
    Empty,
}

#[derive(Clone)]
pub struct ParserActor {
    flight_name: String,
    pub patterns: HashMap<String, Vec<Pattern>>, // flight_id → patterns
    pub registry_address: Addr<Registry>,
    pub rhai_meter_actor: Option<Addr<RhaiActor>>, // optional for now
}

impl Actor for ParserActor {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        let address = ctx.address();
        let registry_address = self.registry_address.clone();
        registry_address.do_send(ParserActorAddr::Real(ctx.address()));
        trace!("Parser actor started");
        self.rhai_meter_actor =
            Some(RhaiActor::new(self.flight_name.clone(), self.registry_address.clone()).start());
    }
}

impl ParserActor {
    pub fn default(flight_name: String, registry_address: Addr<Registry>) -> Self {
        Self {
            flight_name,
            patterns: HashMap::new(),
            registry_address,
            rhai_meter_actor: None,
        }
    }

    pub fn new(flight_name: String, registry_address: Addr<Registry>) -> Self {
        // let patterns = get_patterns_from_database(&flight_name);
        let patterns = HashMap::new();
        // let schema = get_flight_and_schemas(&flight_name);

        Self {
            flight_name,
            patterns,
            registry_address,
            rhai_meter_actor: None,
        }
    }
}

#[derive(Message, Debug, Clone)]
#[rtype(result = "Result<Value, ()>")]
pub struct SubmitRegexRequest {
    pub name: String,
    pub flight_id: String,
    pub log_group: String,
    pub pattern: Vec<Pattern>,
    pub try_parse: bool,
}

impl SubmitRegexRequest {
    pub fn new(regex_request: &RegexHttpRequest) -> Self {
        Self {
            name: regex_request.name.clone(),
            flight_id: regex_request.flight_id.clone(),
            log_group: regex_request.log_group.clone(),
            pattern: regex_request.pattern.clone(),
            try_parse: true,
        }
    }
}

// Handle regex rule registration
impl Handler<SubmitRegexRequest> for ParserActor {
    type Result = Result<Value, ()>;

    // TODO - compile the regex and save
    fn handle(&mut self, msg: SubmitRegexRequest, _ctx: &mut Self::Context) -> Self::Result {
        println!("Received RegexRule in parser: {:?}", msg);
        self.patterns.insert(msg.flight_id.clone(), msg.pattern);
        Ok(Value::String(format!("successfully submitted regex rule - {} for the flight stream - {}", &msg.name, &msg.flight_id)))
    }
}

use arrow_array::builder::BooleanBuilder;
use dashmap::DashMap;
use futures_util::{SinkExt, future};
use log::error;

// Handle incoming data for parsing
impl Handler<RecordBatchWrapper> for ParserActor {
    type Result = ();

    // Tasks to be done

    fn handle(&mut self, record: RecordBatchWrapper, _ctx: &mut Self::Context) -> Self::Result {
        let _service_id = &record.metadata.service_id;
        let registry_address = self.registry_address.clone();
        let registry = registry_address.clone();
        let rhai_meter = self.rhai_meter_actor.clone();
        let fut = async move {
            // let registry_address = registry_address.clone();
            let Ok(result) = registry_address.send(FetchWalActor).await else {
                error!("Failed to fetch WalActorAddr from Registry");
                return;
            };
            let Ok(address) = result else {
                error!("Failed to fetch WalActorAddr from Registry");
                return;
            };

            // Handle mock cases
            let Real(iceberg_actor) = registry.send(FetchIcebergActor).await.unwrap().unwrap()
            else {
                return;
            };

            iceberg_actor.send(record.clone()).await.unwrap();

            match address {
                WalActorWrapper::Real(wal_actors) => {
                    wal_actors.do_send(record.clone());
                }
                #[cfg(test)]
                WalActorWrapper::Mock(wal_actors) => {
                    wal_actors.do_send(record.clone());
                }
                _ => {}
            }
            rhai_meter.unwrap().do_send(record.clone());
        };
        fut.into_actor(self).spawn(_ctx);
    }
}

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
        let try_parsing_regex = msg.try_parsing.clone();
        let registry = self.registry_address.clone();
        let flight_name = msg.flight_name.clone();

        let futures = async move {
            let iceberg_actor = registry
                .send(FetchIcebergActor)
                .await
                .map_err(|_| {
                    std::io::Error::new(std::io::ErrorKind::Other, "Registry mailbox closed")
                })
                .unwrap()
                .unwrap();

            let mut futures = vec![];

            if try_parsing_regex {
                let future = iceberg_actor.get_buffer(flight_name);
                futures.push(future);
            }
            let results: Vec<Result<Vec<RecordBatchWrapper>, core::fmt::Error>> =
                future::join_all(futures).await;

            // let values: Vec<Value> = results
            //     .into_iter()
            //     .filter_map(|outer| match outer {
            //         Ok(inner) => {
            //             Some(apply_regex(inner, msg).await)
            //         },
            //         Err(e) => {
            //             eprintln!("outer error: {e}");
            //             None
            //         }
            //     })
            //     .collect();

            let futures: Vec<_> = results
                .into_iter()
                .filter_map(|outer| match outer {
                    Ok(inner) => {
                        // returns a future
                        Some(apply_regex(inner, msg.clone()))
                    }
                    Err(e) => {
                        eprintln!("outer error: {e}");
                        None
                    }
                })
                .collect();

            // Now await them all
            let values: Vec<Value> = future::join_all(futures).await;

            // let results: Vec<Result<Result<Value, RegexError>, MailboxError>> =
            //     future::join_all(futures).await;

            // let values: Vec<Value> = results
            //     .into_iter()
            //     .filter_map(|outer| match outer {
            //         Ok(inner) => inner.ok(), // keep only Ok(Value)
            //         Err(e) => {
            //             eprintln!("outer error: {e}");
            //             None
            //         }
            //     })
            //     .collect();

            let final_json = serde_json::json!({
                "message": "Processed results from multiple parsers",
                "results": values
            });

            Ok(final_json)
        };

        futures.into_actor(self).boxed_local()
    }
}

use crate::api::http::messages::regex_messages::{Pattern, RegexHttpRequest};
use crate::application::actors::iceberg::IcebergActorAddr::Real;
use crate::application::actors::rhai_meter::RhaiActor;
use crate::application::actors::wal::WalActorWrapper;
use crate::core::error::exception::flight::FlightError;
use crate::core::error::exception::iceberg_error::IcebergError;
use crate::core::error::exception::regex::RegexError;
use crate::platform::registry::{FetchIcebergActor, FetchWalActor, Registry};
use regex::Regex;
use serde_derive::{Deserialize, Serialize};
use serde_json::Value;
use tracing::trace;
use utoipa::ToSchema;
use crate::core::utils::regex::apply_regex;

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
