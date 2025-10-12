use crate::application::actors::broadcaster::broadcast_actor::RecordBatchWrapper;
use crate::application::actors::iceberg::handler::flush_instruction::FlushInstruction;
use crate::application::actors::iceberg::iceberg_actor::IcebergActorAddr;
use crate::application::actors::wal::log::handler::wal_rotation::RotateWAL;
use crate::application::actors::wal::log::log_wal_actor::WalActor;
use crate::core::error::exception::actor_errors::ActorError::ActorError;
use crate::core::utils::transformers::{
    build_flatbufmeta_with_logmeta, serialize_record_batch_full_ipc,
};
use crate::platform::registry::FetchIcebergActor;
use crate::platform::wal::writer::writer::write_wal_block;
use actix::{AsyncContext, Handler};
use tokio::spawn;
use tracing::info;
impl Handler<RecordBatchWrapper> for WalActor {
    type Result = ();

    fn handle(
        &mut self,
        record_batch_wrapper: RecordBatchWrapper,
        ctx: &mut Self::Context,
    ) -> Self::Result {
        let metadata_bytes = build_flatbufmeta_with_logmeta(&record_batch_wrapper.get_metadata());
        let data_bytes = serialize_record_batch_full_ipc(&record_batch_wrapper);

        if let Err(e) = write_wal_block(&mut self.writer, &data_bytes, &metadata_bytes) {
            tracing::error!("Failed to write WAL block: {}", e);
            return;
        }

        self.size += data_bytes.len() as u128 + metadata_bytes.len() as u128;
        info!("Wrote {} MB to WAL file", self.size / (1024 * 1024));

        // If threshold exceeded, send flush + rotate
        if self.size > 400_000_000 {
            let registry_address = self.registry_address.clone();
            let myself = ctx.address();

            spawn(async move {
                match registry_address
                    .send(FetchIcebergActor)
                    .await
                    .unwrap_or(Err(ActorError("Actor missing".to_string())))
                {
                    Ok(IcebergActorAddr::Real(iceberg)) => {
                        let _ = iceberg.do_send(FlushInstruction {});
                        info!("Sent flush instruction to Iceberg actor");
                    }
                    _ => info!("Iceberg actor not found"),
                }

                myself.do_send(RotateWAL {});
            });
        }
    }
}
