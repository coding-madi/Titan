// Write ahead log entries

use std::fs::{File, OpenOptions};

use actix::{Actor, Context, Handler};
use std::io::BufWriter;
use tracing::info;

use crate::{
    actors::broadcast_actor::RecordBatchWrapper,
    utils::transformers::{build_flatbufmeta_with_logmeta, serialize_record_batch_full_ipc},
    wal::writer::writer::write_wal_block,
};

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
        // Convert the RecordBatchWrapper to layout WalBlockHeader
        //
        let metadata_bytes = build_flatbufmeta_with_logmeta(&record_batch_wrapper.metadata);

        info!("Length of metadata = {:?}", &metadata_bytes.len());

        // let data_bytes = serialize_record_batch_without_schema(&record_batch_wrapper);

        let data_bytes = serialize_record_batch_full_ipc(&record_batch_wrapper);

        info!("Length of actual data: {:?}", &data_bytes.len());

        if let Err(e) = write_wal_block(&mut self.writer, &data_bytes, &metadata_bytes) {
            tracing::error!("Failed to write WAL block: {}", e);
        };
        info!("WAL block written successfully");
    }
}
