use crate::application::actors::broadcaster::broadcast_actor::RecordBatchWrapper;
use crate::application::actors::iceberg::iceberg_actor::IcebergActor;
use actix::Handler;

/// This handler will send the data to the buffer manager over tokio async channels.
/// This enabled us to have back-pressuring per flight.
impl Handler<RecordBatchWrapper> for IcebergActor {
    type Result = ();

    fn handle(&mut self, msg: RecordBatchWrapper, _ctx: &mut Self::Context) {
        let buffer_manager = self.buffer_manager.clone(); // Arc<BufferManager>

        tokio::spawn(async move {
            let flight = msg.get_flight_name();

            // Get or create a channel for this flight
            let sender = buffer_manager.get_or_create_sender(&*flight);
            // send async over tokio channels
            let _ = sender.send(msg).await;
        });
    }
}
