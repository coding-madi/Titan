use actix::{Actor, Addr, AsyncContext, Context, Message};
use actix::{ContextFutureSpawner, Handler, WrapFuture};
use arrow_array::RecordBatch;
use arrow_schema::Schema;
use std::fmt::Display;
use tracing::trace;

#[derive(Clone, Message)]
#[rtype(result = "()")]
pub enum BroadcastActorAddr {
    Real(Addr<BroadcastActor>),
    #[cfg(test)]
    Mock(Addr<MockBroadcastActor>),
    Empty,
}

impl BroadcastActorAddr {
    pub fn regex_request(&self, regex_request: RegexRequest) {
        match self {
            BroadcastActorAddr::Real(addr) => addr.do_send(regex_request),
            #[cfg(test)]
            BroadcastActorAddr::Mock(addr) => addr.do_send(regex_request),
            _ => {}
        }
    }

    pub fn record_batch_wrapper(&self, record_batch_wrapper: RecordBatchWrapper) {
        match self {
            BroadcastActorAddr::Real(addr) => addr.do_send(record_batch_wrapper),
            #[cfg(test)]
            BroadcastActorAddr::Mock(addr) => addr.do_send(record_batch_wrapper),
            _ => {}
        }
    }
}

#[derive(Clone)]
pub struct BroadcastActor {
    pub next_shard_idx: usize, // Index to keep track of the next shard to send messages to
    pub registry_address: Addr<Registry>,
}

impl BroadcastActor {
    pub fn new(registry_address: Addr<Registry>) -> BroadcastActor {
        Self {
            next_shard_idx: 0,
            registry_address,
        }
    }
}

impl Actor for BroadcastActor {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        let address = ctx.address();
        let registry_address = self.registry_address.clone();
        registry_address.do_send(BroadcastActorAddr::Real(address));
        trace!("BroadcastActor started");
    }
}

// We need to initialize the actor and pass the handles to the Query server.
// The query server will then use these handles to send messages to the regex actors.
// The regex actors will then process the messages and return results to the query server.
impl Handler<RegexRequest> for BroadcastActor {
    type Result = Result<(), ValidationErrors>;

    fn handle(&mut self, regex_request: RegexRequest, _ctx: &mut Self::Context) -> Self::Result {
        // let x = self.registry_address.send(FetchParserActor);
        let x = self.clone();
        let fut = async move {
            match x.registry_address.send(FetchParserActor).await {
                Ok(parser) => match parser {
                    Ok(pattern) => match pattern {
                        ParserActorAddr::Real(parser) => {
                            for i in 0..parser.len() {
                                parser[i].do_send(regex_request.clone())
                            }
                        }
                        #[cfg(test)]
                        ParserActorAddr::Mock(add) => {
                            add[0].do_send(regex_request.clone());
                        }
                        _ => {}
                    },
                    _ => {}
                },
                _ => {}
            }
        };
        fut.into_actor(self).spawn(_ctx);
        Ok(())
    }
}

use crate::api::http::regex::RegexRequest;
use crate::platform::registry::{FetchParserActor, ParserActorAddr, Registry};
use std::sync::Arc;
use validator::ValidationErrors;

#[derive(Debug, Clone, actix::Message)]
#[rtype(result = "()")]
pub struct RecordBatchWrapper {
    pub metadata: Metadata,
    pub data: Arc<RecordBatch>,
}

impl Handler<RecordBatchWrapper> for BroadcastActor {
    type Result = ();

    fn handle(
        &mut self,
        record_batch: RecordBatchWrapper,
        _ctx: &mut Self::Context,
    ) -> Self::Result {
        let mut broadcast_actor = self.clone();
        let fut = async move {
            match broadcast_actor
                .registry_address
                .send(FetchParserActor)
                .await
            {
                Ok(parser) => match parser {
                    Ok(pattern) => match pattern {
                        ParserActorAddr::Real(parser) => {
                            let current_shard_idx = &mut broadcast_actor.next_shard_idx;
                            let current_shard_idx =
                                (current_shard_idx.wrapping_add(1)) % parser.len();
                            parser
                                .get(current_shard_idx)
                                .unwrap()
                                .do_send(record_batch.clone());
                        }
                        #[cfg(test)]
                        ParserActorAddr::Mock(add) => {}
                        _ => {}
                    },
                    _ => {}
                },
                _ => {}
            }
        };
        fut.into_actor(self).spawn(_ctx);
    }
}

#[derive(Debug, Clone)]
pub struct Metadata {
    pub flight: String,
    pub buffer_id: u64,
    pub schema: Arc<Schema>,
    pub service_id: String,
}

impl Display for Metadata {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Metadata(key: {}, Schema: {:?})",
            self.flight, self.schema
        )
    }
}

#[cfg(test)]
pub struct MockBroadcastActor {
    pub registry_address: Addr<Registry>,
    pub data: Vec<RecordBatchWrapper>,
    pub regex_request: Vec<RegexRequest>,
}

#[cfg(test)]
impl Actor for MockBroadcastActor {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        let registry_address = self.registry_address.clone();
        let address = ctx.address();
        registry_address.do_send(BroadcastActorAddr::Mock(address.clone()));
        trace!("MockBroadcastActor started");
    }
}

#[cfg(test)]
impl Handler<RegexRequest> for MockBroadcastActor {
    type Result = Result<(), ValidationErrors>;

    fn handle(&mut self, _msg: RegexRequest, _ctx: &mut Self::Context) -> Self::Result {
        self.regex_request.push(_msg.clone());
        Ok(())
    }
}

#[cfg(test)]
impl Handler<RecordBatchWrapper> for MockBroadcastActor {
    type Result = ();

    fn handle(&mut self, _msg: RecordBatchWrapper, _ctx: &mut Self::Context) -> Self::Result {
        todo!();
    }
}
