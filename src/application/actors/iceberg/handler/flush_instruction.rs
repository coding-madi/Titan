use crate::application::actors::iceberg::iceberg_actor::IcebergActor;
#[cfg(test)]
use crate::application::actors::iceberg::iceberg_actor::MockIcebergActor;
use crate::core::catalog::iceberg_operations::flush_buffer_to_table;
use crate::core::error::exception::iceberg_error::IcebergError;
use actix::{Handler, Message};
use log::error;
use tracing::info;

/// This flush is called by the WAL actor. A flush is triggered in every log rotation
#[derive(Message)]
#[rtype(result = "Result<(), IcebergError>")]
pub struct FlushInstruction;

impl Handler<FlushInstruction> for IcebergActor {
    type Result = Result<(), IcebergError>;

    // TODO - Handle errors and retries
    // TODO - Send errors to alerting system
    fn handle(&mut self, _msg: FlushInstruction, _ctx: &mut Self::Context) -> Self::Result {
        info!("Received FlushInstruction. Attempting to flush buffered data.");
        let buffer_manager = self.buffer_manager.clone();
        let catalog = self.catalog.clone();
        let namespace = self.namespace.clone();
        // Fire-and-forget async flush
        tokio::spawn(async move {
            if let Err(e) = flush_buffer_to_table(
                buffer_manager.clone(),
                catalog.clone(),
                namespace.clone(),
                None,
            )
            .await
            {
                // You can log or send to an error-reporting actor here
                error!("Flush failed: {:?}", e);
            }
        });
        Ok(())
    }
}

#[cfg(test)]
impl Handler<FlushInstruction> for MockIcebergActor {
    type Result = Result<(), IcebergError>;

    fn handle(&mut self, _msg: FlushInstruction, _ctx: &mut Self::Context) -> Self::Result {
        todo!()
    }
}
