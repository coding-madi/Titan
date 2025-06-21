// Write ahead log entries

use std::{
    fs::{File, OpenOptions},
    sync::Arc,
};

use actix::{Actor, Context, Handler};
use std::io::BufWriter;
use tracing::{info, trace};

use crate::{actors::broadcast::RecordBatchWrapper, wal::writer::writer::write_wal_block};

pub struct WalEntry {
    pub writer: BufWriter<File>,
}

impl Actor for WalEntry {
    type Context = Context<Self>;
}

impl WalEntry {
    pub fn new() -> Self {
        info!("Creating new WAL entry actor");
        let file: File = OpenOptions::new()
            .write(true)
            .create(true)
            .append(true)
            .open("/tmp/wal_entry.log")
            .expect("Failed to open WAL entry log file");
        WalEntry {
            writer: BufWriter::new(file),
        }
    }
}

// The actor will get messages as RecordBatchWrapper (regex applied/structured), it needs to write them to
// WAL files as Arrow IPC streams.

impl Handler<RecordBatchWrapper> for WalEntry {
    type Result = ();

    fn handle(
        &mut self,
        record_batch_wrapper: RecordBatchWrapper,
        _ctx: &mut Self::Context,
    ) -> Self::Result {
        info!("Received RecordBatchWrapper for WAL entry");
        trace!("Writing WAL block asynchronously");
        let numbers_vec: Vec<u8> = (1..=48).collect();
        let numbers_array: [u8; 48] = numbers_vec
            .try_into()
            .expect("Vec has incorrect length for fixed-size array");
        if let Err(e) = write_wal_block(
            &mut self.writer,
            &Arc::new(record_batch_wrapper),
            &numbers_array,
        ) {
            tracing::error!("Failed to write WAL block: {}", e);
        };
        info!("WAL block written successfully");
    }
}
