// This actor reads Arrow IPC Wal files
// It then constructs a new Arrow buffer, that is partitioned based on the configuration

// TODO: Vectorized read of the Wal file. Also, read some fields from metadata for grouping.
// This should be done on a mmap file and the metadata should be stored in Flatbuf.

use crate::platform::registry::Registry;
#[cfg(test)]
use actix::Context;
use actix::{Actor, Addr, AsyncContext, Handler, Message};

#[derive(Clone, Message)]
#[rtype(result = "()")]
pub enum IcebergActorAddr {
    Real(Addr<IcebergActor>),
    #[cfg(test)]
    Mock(Addr<MockIcebergActor>),
    Empty,
}

#[derive(Clone)]
pub struct IcebergActor {
    _table: String,
    _schema: String,
    _partition_fields: Vec<String>,
    registry_address: Addr<Registry>,
}

impl IcebergActor {
    pub fn new(
        _table: String,
        _schema: String,
        _partition_fields: Vec<String>,
        registry_address: Addr<Registry>,
    ) -> Self {
        IcebergActor {
            _table,
            _schema,
            _partition_fields,
            registry_address,
        }
    }

    pub fn default(registry_address: Addr<Registry>) -> Self {
        IcebergActor {
            _table: "log".to_string(),
            _schema: "Schema".to_string(),
            _partition_fields: vec!["service".to_string(), "log_name".to_string()],
            registry_address,
        }
    }

    pub fn write(&self, _data: &[u8]) {
        // Here we would write the data to the WAL file
        // For now, we just print the data
    }
}

impl Actor for IcebergActor {
    type Context = actix::Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        let address = ctx.address();
        let registry_address = self.registry_address.clone();
        actix_rt::spawn(async move {
            // let pool = settings_for_spawn.connection_pool().await;
            registry_address.do_send(IcebergActorAddr::Real(address));
        });
        println!("Started IcebergActor");
    }
}

pub struct FlushInstruction;

impl Message for FlushInstruction {
    type Result = Result<(), String>;
}

// Trigger a flush to the iceberg table
impl Handler<FlushInstruction> for IcebergActor {
    type Result = Result<(), String>;

    fn handle(&mut self, _msg: FlushInstruction, _ctx: &mut Self::Context) -> Self::Result {
        todo!()
    }
}

#[cfg(test)]
#[derive(Clone)]
pub struct MockIcebergActor {
    pub(crate) registry_address: Addr<Registry>,
}

#[cfg(test)]
impl Actor for MockIcebergActor {
    type Context = Context<Self>;
}

#[cfg(test)]
impl Handler<FlushInstruction> for MockIcebergActor {
    type Result = Result<(), String>;

    fn handle(&mut self, _msg: FlushInstruction, _ctx: &mut Self::Context) -> Self::Result {
        todo!()
    }
}
