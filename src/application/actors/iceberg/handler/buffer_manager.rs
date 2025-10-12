use crate::application::actors::broadcaster::broadcast_actor::RecordBatchWrapper;
use crate::application::actors::iceberg::iceberg_actor::IcebergActor;
use crate::core::error::exception::buffer_error::BufferError;
use actix::{Handler, Message, ResponseFuture};
use std::fmt::Error;

#[derive(Message)]
#[rtype(result = "Result<Vec<RecordBatchWrapper>, BufferError>")]
pub struct GetBuffer {
    stream: String,
}

impl GetBuffer {
    pub fn new(stream: String) -> Self {
        Self { stream }
    }
}

impl Handler<GetBuffer> for IcebergActor {
    type Result = ResponseFuture<Result<Vec<RecordBatchWrapper>, BufferError>>;
    fn handle(&mut self, msg: GetBuffer, _ctx: &mut Self::Context) -> Self::Result {
        let buffer_manager = self.buffer_manager.clone();

        Box::pin(async move {
            buffer_manager
                .get(&msg.stream)
                .ok_or_else(|| BufferError::BufferMissing(msg.stream.clone()))
        })
    }
}
