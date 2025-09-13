use actix::{Actor, Addr, AsyncContext, Context, MailboxError, Message, ResponseActFuture};
use actix::{ContextFutureSpawner, Handler, WrapFuture};
use arrow_array::RecordBatch;
use arrow_schema::Schema;
use std::collections::HashMap;
use std::fmt::Display;
use tracing::{error, trace};

#[derive(Clone, Message, Debug)]
#[rtype(result = "()")]
pub enum BroadcastActorWrapper {
    Real(HashMap<String, Addr<BroadcastActor>>), // Flight -> BroadcastActor
    #[cfg(test)]
    Mock(HashMap<String, Addr<MockBroadcastActor>>),
    Empty,
}

impl BroadcastActorWrapper {
    pub async fn regex_request(
        &self,
        regex_request: SubmitRegexRequest,
    ) -> Result<Value, RegexError> {
        match self {
            BroadcastActorWrapper::Real(broadcast_actors) => {
                if broadcast_actors.contains_key(regex_request.get_flight_id()) {
                    let broadcast_actor =
                        broadcast_actors.get(regex_request.get_flight_id()).unwrap();
                    if regex_request.is_try_parse() {
                        Ok(broadcast_actor
                            .send(TryParsingRegex::new(&regex_request))
                            .await
                            .unwrap()?)
                    } else {
                        Ok(broadcast_actor.send(regex_request).await.unwrap().unwrap())
                    }
                } else {
                    Err(RegexError::RegexIncorrect(
                        "Flight not found in registry".to_string(),
                    ))
                }
            }
            #[cfg(test)]
            BroadcastActorWrapper::Mock(_addr) => {
                todo!()
            }
            _ => {
                panic!("Invalid broadcast actor address")
            }
        }
    }

    pub fn record_batch_wrapper(&self, record_batch_wrapper: RecordBatchWrapper) {
        match self {
            BroadcastActorWrapper::Real(addr) => {
                if addr.contains_key(record_batch_wrapper.get_flight_name()) {
                    let addr = addr.get(record_batch_wrapper.get_flight_name()).unwrap();
                    addr.do_send(record_batch_wrapper);
                } else {
                    // error!(format!("Flight not found in registry - {}", regex_request.flight_id).as_str());
                    // Err(FlightError::FlightMissing("Flight not found in registry".to_string())).expect("TODO: panic message");
                    panic!("Flight not found in registry");
                }
            }
            #[cfg(test)]
            BroadcastActorWrapper::Mock(_addr) => {
                todo!()
            }
            _ => {}
        }
    }
}

#[derive(Clone)]
pub struct BroadcastActor {
    next_shard_idx: usize, // Index to keep track of the next shard to send messages to
    flight_name: String,
    registry_address: Addr<Registry>,
    parsers: Vec<ParserActorAddr>,
}

impl BroadcastActor {
    pub fn new(
        registry_address: Addr<Registry>,
        flight_name: String,
        parsers: Vec<ParserActorAddr>,
    ) -> BroadcastActor {
        Self {
            next_shard_idx: 0,
            flight_name: flight_name.to_string(),
            registry_address,
            parsers,
        }
    }

    pub fn get_registry_actor(&self) -> Addr<Registry> {
        self.registry_address.clone()
    }

    pub fn get_flight_name(&self) -> &str {
        &self.flight_name
    }
}

impl Actor for BroadcastActor {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        let address = ctx.address();
        let registry = self.get_registry_actor();
        let flight_name = self.get_flight_name().to_string();
        let mut map = HashMap::new();
        map.insert(flight_name, address.clone());
        let broadcast_actor_wrapped_single = BroadcastActorWrapper::Real(map);
        registry.do_send(RegisterBroadcastActor {
            broadcast_actor_wrapped_single,
        });
        trace!("BroadcastActor started");
    }
}

impl Handler<SubmitRegexRequest> for BroadcastActor {
    type Result = ResponseActFuture<Self, Result<Value, RegexError>>;
    fn handle(
        &mut self,
        regex_request: SubmitRegexRequest,
        _ctx: &mut Self::Context,
    ) -> Self::Result {
        // Correct: Clone the parsers to move them into the async block.
        let parsers = self.parsers.clone();
        let async_task = async move {
            let values = submit_regex_request(parsers, regex_request).await;
            let final_json = serde_json::json!({
                "message": "Processed results from multiple parsers",
                "results": values,
            });
            Ok(final_json)
        };

        // Correct: Use `into_actor(self)`. The `actix` framework handles the
        // ownership of the actor for the duration of the future.
        async_task.into_actor(self).boxed_local()
    }
}

/// The difference SubmitRegexRequest and TryParsingRegex is that,
/// SubmitRegexRequest simply submits the request and does not validate the regex against real data.
/// TryParsingRegex fetches the data from cache and applies the regex over the data
impl Handler<TryParsingRegex> for BroadcastActor {
    type Result = ResponseActFuture<Self, Result<Value, RegexError>>;

    fn handle(&mut self, msg: TryParsingRegex, _ctx: &mut Self::Context) -> Self::Result {
        let parsers = self.parsers.clone();
        let async_task = async move {
            let values = submit_regex_request(parsers, msg).await;
            let final_json = serde_json::json!({
                "message": "Processed results from multiple parsers",
                "results": values,
            });

            Ok(final_json)
        };

        async_task.into_actor(self).boxed_local()
    }
}

#[cfg(test)]
pub(crate) use crate::application::actors::broadcast_actor::test::MockBroadcastActor;
use crate::application::actors::parser_actor::{SubmitRegexRequest, TryParsingRegex};
use crate::core::error::exception::regex::RegexError;
use crate::core::utils::transformers::flatten_list;
use crate::platform::registry::{ParserActor, ParserActorAddr, RegisterBroadcastActor, Registry};
use actix::fut::ActorFutureExt;
use futures_util::{FutureExt, SinkExt, future};
use serde_json::Value;
use std::sync::Arc;

#[derive(Debug, Clone, actix::Message)]
#[rtype(result = "()")]
pub struct RecordBatchWrapper {
    metadata: Metadata,
    data: Arc<RecordBatch>,
}

impl RecordBatchWrapper {
    pub fn new(metadata: Metadata, data: &RecordBatch) -> Self {
        let data = Arc::new(data.clone());
        Self { metadata, data }
    }

    pub fn get_flight_name(&self) -> &str {
        &self.metadata.get_flight_name()
    }

    pub fn get_data(&self) -> Arc<RecordBatch> {
        self.data.clone()
    }

    pub fn get_data_ref(&self) -> &RecordBatch {
        self.data.as_ref()
    }

    pub fn get_metadata(&self) -> &Metadata {
        &self.metadata
    }
}

impl Handler<RecordBatchWrapper> for BroadcastActor {
    type Result = ();

    fn handle(
        &mut self,
        record_batch: RecordBatchWrapper,
        ctx: &mut Self::Context,
    ) -> Self::Result {
        let mut next_shard_idx = self.next_shard_idx;
        let record_batch_cloned = record_batch.clone();
        let parser_addr = self.parsers.get(next_shard_idx).unwrap().clone(); // Get parser in a round robin fashion
        let no_of_parsers = self.parsers.len().clone();
        async move {
            match parser_addr {
                ParserActorAddr::Real(addr) => {
                    addr.send(record_batch_cloned.clone()).await.unwrap();
                }
                #[cfg(test)]
                ParserActorAddr::Mock(addr) => {
                    addr.send(record_batch_cloned.clone()).await.unwrap();
                }
                _ => {
                    error!("Invalid parser actor address")
                }
            };

            // Update the shard index for next time
            next_shard_idx = (next_shard_idx.wrapping_add(1)) % no_of_parsers;
        }
        .into_actor(self)
        .map(move |_, actor, _| {
            // Update the actor's shard index
            actor.next_shard_idx = next_shard_idx;
        })
        .spawn(ctx);
    }
}

#[derive(Debug, Clone)]
pub struct Metadata {
    pub flight_name: String,
    pub buffer_id: u64,
    pub schema: Arc<Schema>,
}

impl Metadata {
    pub fn new(flight_name: &str, buffer_id: u64, schema: Arc<Schema>) -> Self {
        Self {
            flight_name: flight_name.to_string(),
            buffer_id,
            schema,
        }
    }

    pub fn get_flight_name(&self) -> &str {
        &self.flight_name
    }
}

impl Display for Metadata {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Metadata(key: {}, Schema: {:?})",
            self.get_flight_name(),
            self.schema
        )
    }
}

pub async fn submit_regex_request<M>(parsers: Vec<ParserActorAddr>, regex_request: M) -> Vec<Value>
where
    M: Message<Result = Result<Value, RegexError>> + Clone + Send + 'static,
    ParserActor: Handler<M>, // 👈 tell the compiler ParserActor can handle this message
{
    let mut futures = Vec::new();
    // Correct: Iterate over the cloned `parsers` vector, not `self.parsers`.
    for parser_actor in parsers {
        match parser_actor {
            ParserActorAddr::Real(parser) => {
                let f = parser.send(regex_request.clone());
                futures.push(f);
            }
            #[cfg(test)]
            ParserActorAddr::Mock(_parser) => {}
            _ => {
                error!("Invalid parser actor address")
            }
        }
    }

    let results: Vec<Result<Result<Value, RegexError>, MailboxError>> =
        future::join_all(futures).await;

    let json_value = flatten_list(results);
    json_value
}

#[cfg(test)]
pub mod test {
    use super::*;
    use crate::api::http::messages::regex_messages::RegexHttpRequest;

    #[cfg(test)]
    pub struct MockBroadcastActor {
        pub registry_address: Addr<Registry>,
        pub data: Vec<RecordBatchWrapper>,
        pub regex_request: Vec<RegexHttpRequest>,
    }

    #[cfg(test)]
    impl Actor for MockBroadcastActor {
        type Context = Context<Self>;

        fn started(&mut self, ctx: &mut Self::Context) {}
    }

    #[cfg(test)]
    impl Handler<RegexHttpRequest> for MockBroadcastActor {
        type Result = Result<Value, RegexError>;

        fn handle(&mut self, _msg: RegexHttpRequest, _ctx: &mut Self::Context) -> Self::Result {
            self.regex_request.push(_msg.clone());
            Ok(Value::String("Regex submitted successfully".to_string()))
        }
    }

    #[cfg(test)]
    impl Handler<RecordBatchWrapper> for MockBroadcastActor {
        type Result = ();

        fn handle(&mut self, _msg: RecordBatchWrapper, _ctx: &mut Self::Context) -> Self::Result {
            todo!();
        }
    }
}
