// Write ahead log entries

use actix::{Actor, Context, Handler};

use crate::actors::broadcast::RecordBatchWrapper;

pub struct WalEntry {}

impl Actor for WalEntry {
    type Context = Context<Self>;
}

impl WalEntry {
    pub fn new() -> Self {
        WalEntry {}
    }
}

// The actor will get messages as RecordBatchWrapper (regex applied/structured), it needs to write them to
// WAL files as Arrow IPC streams.

impl Handler<RecordBatchWrapper> for WalEntry {
    type Result = ();

    fn handle(&mut self, _msg: RecordBatchWrapper, _ctx: &mut Self::Context) -> Self::Result {
        todo!()
    }
}
