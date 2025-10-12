use crate::application::actors::broadcaster::broadcast_actor::Metadata;
use crate::application::actors::iceberg::iceberg_actor::IcebergActorAddr;
use crate::application::actors::wal::metric::handler::rotate::RotateWAL;
use crate::application::actors::wal::metric::metric_wal_actor::WalMetricActor;
use crate::core::error::exception::actor_errors::ActorError::ActorError;
use crate::core::utils::transformers::{
    build_flatbufmeta_with_logmeta, serialize_record_batch_full_ipc,
};
use crate::platform::registry::FetchIcebergActor;
use crate::platform::wal::writer::writer::write_wal_block;
use actix::{AsyncContext, Handler, Message};
use arrow_array::RecordBatch;
use console_subscriber::spawn;
use std::sync::Arc;
use tracing::info;
#[derive(Message, Debug, Clone)]
#[rtype(result = "()")]
pub struct MetricRecordBatch {
    metadata: Metadata,
    data: Arc<RecordBatch>,
}

impl MetricRecordBatch {
    pub fn new(metadata: Metadata, data: Arc<RecordBatch>) -> Self {
        Self { metadata, data }
    }

    pub fn get_data(&self) -> Arc<RecordBatch> {
        Arc::clone(&self.data)
    }

    pub fn get_metadata(&self) -> &Metadata {
        &self.metadata
    }
}

impl Handler<MetricRecordBatch> for WalMetricActor {
    type Result = ();

    fn handle(
        &mut self,
        metric_record_batch: MetricRecordBatch,
        ctx: &mut Self::Context,
    ) -> Self::Result {
        let metadata_bytes = build_flatbufmeta_with_logmeta(metric_record_batch.get_metadata());
        let data_bytes = serialize_record_batch_full_ipc(&metric_record_batch.into());

        if let Err(e) = write_wal_block(&mut self.write, &data_bytes, &metadata_bytes) {
            tracing::error!("WAL block write failure: {}", e);
            return;
        }

        self.size += data_bytes.len() as u128 + metadata_bytes.len() as u128;
        info!(
            "Wrote {} Mbs to Metric WAL file",
            data_bytes.len() / (1024 * 1024)
        );

        if self.size > 4_000_000 {
            let registry_address = self.registry_address.clone();
            let myself = ctx.address();

            tokio::spawn(async move {
                match registry_address
                    .send(FetchIcebergActor)
                    .await
                    .unwrap_or(Err(ActorError("Actor missing".to_string())))
                {
                    Ok(IcebergActorAddr::Real(iceberg)) => {
                        let _ = iceberg.do_send(crate::application::actors::iceberg::handler::flush_instruction::FlushInstruction {});
                        tracing::info!("Sent flush instruction to Iceberg actor");
                    }
                    _ => tracing::info!("Iceberg actor not found"),
                }

                myself.do_send(RotateWAL {});
            });
        };
    }
}
