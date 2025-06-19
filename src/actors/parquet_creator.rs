// This actor reads Arrow IPC Wal files
// It then constructs a new Arrow buffer, that is partitioned based on the configuration

// TODO: Vectorized read of the Wal file. Also, read some fields from metadata for grouping.
// This should be done on a mmap file and the metadata should be stored in Flatbuf.

use actix::Actor;

pub struct LogWriter {}

impl LogWriter {
    pub fn new() -> Self {
        LogWriter {}
    }

    pub fn write(&self, data: &[u8]) {
        // Here we would write the data to the WAL file
        // For now, we just print the data
        println!("Writing to WAL: {:?}", data);
    }
}

impl Actor for LogWriter {
    type Context = actix::Context<Self>;
}
