use actix::{Actor, Addr, AsyncContext, Context, MailboxError, Message, ResponseActFuture};
use actix::{ContextFutureSpawner, Handler, WrapFuture};
use arrow_array::RecordBatch;
use arrow_schema::Schema;
use std::fmt::Display;
use tracing::{error, info, trace};

#[derive(Clone, Message, Debug)]
#[rtype(result = "()")]
pub enum BroadcastActorWrapper {
    Real(Addr<BroadcastActor>), // Flight -> BroadcastActor
    #[cfg(test)]
    Mock(Addr<MockBroadcastActor>),
    Empty,
}

impl BroadcastActorWrapper {
    pub async fn regex_request(
        &self,
        regex_request: SubmitRegexRequest,
    ) -> Result<Value, RegexError> {
        match self {
            BroadcastActorWrapper::Real(broadcast_actor) => {
                if regex_request.is_try_parse() {
                    Ok(broadcast_actor
                        .send(TryParsingRegex::new(&regex_request))
                        .await
                        .unwrap()?)
                } else {
                    broadcast_actor.send(regex_request).await.unwrap()
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
                addr.do_send(record_batch_wrapper);
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
    pub(crate) next_shard_idx: usize, // Index to keep track of the next shard to send messages to
    flight_name: String,
    registry_address: Addr<Registry>,
    pub(crate) parsers: Vec<ParserActorAddr>,
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
        let broadcast_actor_wrapped_single = BroadcastActorWrapper::Real(address.clone());
        registry.do_send(RegisterBroadcastActor {
            flight_name,
            broadcast_actor_wrapped_single,
        });
        info!("BroadcastActor started");
    }
}

#[cfg(test)]
pub(crate) use crate::application::actors::broadcaster::broadcast_actor::test::MockBroadcastActor;
pub(crate) use crate::application::actors::broadcaster::handler::record_batch::RecordBatchWrapper;
use crate::application::actors::parser::handlers::try_parsing_regex::TryParsingRegex;
use crate::application::actors::parser::parser_actor::SubmitRegexRequest;
use crate::core::error::exception::regex::RegexError;
use crate::core::utils::transformers::flatten_list;
use crate::platform::registry::{ParserActor, ParserActorAddr, RegisterBroadcastActor, Registry};
use actix::fut::ActorFutureExt;
use futures_util::{FutureExt, SinkExt, future};
use serde_json::Value;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct Metadata {
    pub flight_name: String,
    pub buffer_id: u64,
    pub schema: Arc<Schema>,
    pub oldest_timestamp: Option<i64>,
    pub newest_timestamp: Option<i64>,
}

impl Metadata {
    pub fn new(
        flight_name: &str,
        buffer_id: u64,
        schema: Arc<Schema>,
        oldest_timestamp: Option<i64>,
        newest_timestamp: Option<i64>,
    ) -> Self {
        Self {
            flight_name: flight_name.to_string(),
            buffer_id,
            schema,
            oldest_timestamp,
            newest_timestamp,
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
    ParserActor: Handler<M>, // 👈 tell the compiler ParserActor can handle this handler
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
