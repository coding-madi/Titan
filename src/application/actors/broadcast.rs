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

impl Handler<RegexRequest> for BroadcastActor {
    type Result = Result<(), ValidationErrors>;

    fn handle(&mut self, regex_request: RegexRequest, ctx: &mut Self::Context) -> Self::Result {
        let registry_addr = self.registry_address.clone();
        let request = regex_request.clone();

        // Spawn an async future in actor context
        async move {
            // Send message to fetch ParserActorAddr
            let parser_addr_res = registry_addr.send(FetchParserActor).await;

            let parser_addr = match parser_addr_res {
                Ok(Ok(parser)) => parser,
                _ => {
                    // Could log error here or handle failure more gracefully
                    return;
                }
            };

            match parser_addr {
                ParserActorAddr::Real(parsers) => {
                    for parser in parsers {
                        parser.do_send(request.clone());
                    }
                }
                #[cfg(test)]
                ParserActorAddr::Mock(parsers) => {
                    if let Some(first) = parsers.get(0) {
                        first.do_send(request.clone());
                    }
                }
                _ => {}
            }
        }
        .into_actor(self)
        .spawn(ctx);

        Ok(())
    }
}

use crate::api::http::regex::RegexRequest;
use crate::platform::registry::{FetchParserActor, ParserActorAddr, Registry};
use actix::fut::ActorFutureExt;
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
        ctx: &mut Self::Context,
    ) -> Self::Result {
        let registry_addr = self.registry_address.clone();
        let mut next_shard_idx = self.next_shard_idx;
        let record_batch_cloned = record_batch.clone();

        async move {
            let parser_result = registry_addr.send(FetchParserActor).await;

            let parsers = match parser_result {
                Ok(Ok(ParserActorAddr::Real(p))) if !p.is_empty() => p,
                _ => return, // No valid parsers available
            };

            // Round-robin shard index update and send
            let idx = next_shard_idx % parsers.len();
            parsers[idx].do_send(record_batch_cloned);

            // Update the shard index for next time
            next_shard_idx = (next_shard_idx.wrapping_add(1)) % parsers.len();
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
