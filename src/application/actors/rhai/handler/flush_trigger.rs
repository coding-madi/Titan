use crate::application::actors::rhai::rhai_actor::RhaiActor;
use actix::{Handler, Message};
use std::cmp::max;
use tracing::info;
use crate::application::actors::iceberg::handler::flush_instruction::FlushInstruction;
use crate::application::actors::iceberg::iceberg_actor::IcebergActorAddr;
use crate::platform::registry::{FetchIcebergActor, FetchIcebergMeterActor};

#[derive(Message)]
#[rtype(result = "()")]
pub struct FlushTrigger {
    pub(crate) purge_window: i64,
    pub(crate) new_watermark: i64,
}

/// Always flush using a handler to prevent race conditions and handle data sequentially
impl Handler<FlushTrigger> for RhaiActor {
    type Result = ();

    fn handle(&mut self, msg: FlushTrigger, _ctx: &mut Self::Context) -> Self::Result {
        // Remove and take ownership of the window batches
        if let Some(batches) = self.window_state.window_state.remove(&msg.purge_window) {
            // TODO: process batches: execute rules, write to WAL, or forward downstream.
            // Example placeholder:
            // self.rhai_executor.process_batches(batches);
            // For now we just log the count
            let registry = self.registry_address.clone();
            let batch_cloned = batches.clone();
            tokio::spawn(async move {
                let iceberg_meter_resp = registry.send(FetchIcebergMeterActor).await;
                let Ok(Ok(iceberg_meter)) = iceberg_meter_resp else { return; };

                match iceberg_meter {
                    IcebergActorAddr::Real(iceberg_meter_actor) => {
                        let x = iceberg_meter_actor.send(batch_cloned.first().unwrap().clone()).await;
                        let flush = iceberg_meter_actor.send(FlushInstruction).await;
                        println!("{:?}", x);
                    }
                    #[cfg(test)]
                    IcebergActorAddr::Mock(_) => {}
                    IcebergActorAddr::Empty => {}
                }
            });



            info!(
                "Flushing window {} with {} record batches",
                msg.purge_window,
                batches.len()
            );
        } else {
            info!("No batches found for window {}", msg.purge_window);
        }

        // Advance watermark to at least new_watermark
        self.window_state.watermark = max(self.window_state.watermark, msg.new_watermark);

        // Remove the scheduled mark so the window can be scheduled again in future if needed
        self.window_state
            .scheduled_windows
            .remove(&msg.purge_window);
    }
}
