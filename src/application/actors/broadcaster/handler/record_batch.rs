use crate::application::actors::broadcaster::broadcast_actor::{BroadcastActor, Metadata};
use crate::platform::registry::ParserActorAddr;
use actix::ActorFutureExt;
use actix::{ContextFutureSpawner, Handler, WrapFuture};
use arrow_array::RecordBatch;
use std::sync::Arc;
use tracing::{error, info};

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

use crate::application::actors::wal::metric::handler::record_batch::MetricRecordBatch;
impl From<MetricRecordBatch> for RecordBatchWrapper {
    fn from(value: MetricRecordBatch) -> Self {
        RecordBatchWrapper::new(value.get_metadata().clone(), value.get_data().as_ref())
    }
}

/// The parsing is a stateless operation, so it being round-robin distributed.
impl Handler<RecordBatchWrapper> for BroadcastActor {
    type Result = ();

    fn handle(
        &mut self,
        record_batch: RecordBatchWrapper,
        ctx: &mut Self::Context,
    ) -> Self::Result {
        info!("Received record batch in broadcast actor");
        let mut next_shard_idx = self.next_shard_idx;
        let record_batch_cloned = record_batch.clone();
        let parser_addr = self.parsers.get(next_shard_idx).unwrap().clone(); // Get parser in a round robin fashion
        let no_of_parsers = self.parsers.len().clone();
        async move {
            match parser_addr {
                ParserActorAddr::Real(addr) => {
                    info!("Sending record batch to parser actor");
                    addr.do_send(record_batch_cloned.clone());
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
