// This actor reads Arrow IPC Wal files
// It then constructs a new Arrow buffer, that is partitioned based on the configuration

// TODO: Vectorized read of the Wal file. Also, read some fields from metadata for grouping.
// This should be done on a mmap file and the metadata should be stored in Flatbuf.

use actix::{Actor, Handler, Message};

pub struct IcebergWriter {
    _table: String,
    _schema: String,
    _partition_fields: Vec<String>,
}

impl IcebergWriter {
    pub fn new(_table: String, _schema: String, _partition_fields: Vec<String>) -> Self {
        IcebergWriter {
            _table,
            _schema,
            _partition_fields,
        }
    }

    pub fn default() -> Self {
        IcebergWriter {
            _table: "log".to_string(),
            _schema: "Schema".to_string(),
            _partition_fields: vec!["service".to_string(), "log_name".to_string()],
        }
    }

    pub fn write(&self, _data: &[u8]) {
        // Here we would write the data to the WAL file
        // For now, we just print the data
    }
}

impl Actor for IcebergWriter {
    type Context = actix::Context<Self>;
}

pub struct FlushInstruction;

impl Message for FlushInstruction {
    type Result = Result<(), String>;
}

// Trigger a flush to the iceberg table
impl Handler<FlushInstruction> for IcebergWriter {
    type Result = Result<(), String>;

    fn handle(&mut self, _msg: FlushInstruction, _ctx: &mut Self::Context) -> Self::Result {
        todo!()
    }
}
