use crate::application::actors::broadcaster::broadcast_actor::RecordBatchWrapper;
use crate::platform::registry::ParserActor;
use actix::{ContextFutureSpawner, Handler, WrapFuture};
use tracing::info;

// Handle incoming data for parsing
impl Handler<RecordBatchWrapper> for ParserActor {
    type Result = ();

    // TODO
    fn handle(&mut self, record: RecordBatchWrapper, _ctx: &mut Self::Context) -> Self::Result {
        info!("Received record batch for parsing !!");
        let fut = self.parser_service.handle_message(record);
        fut.into_actor(self).spawn(_ctx);
    }
}
