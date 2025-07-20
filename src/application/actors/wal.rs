// Write ahead log entries

use std::fs::{File, OpenOptions};

use actix::{Actor, Addr, AsyncContext, Context, Handler, Message};
use std::io::BufWriter;
use tracing::info;

use crate::application::actors::broadcast::RecordBatchWrapper;
#[cfg(test)]
pub(crate) use crate::application::actors::wal_test::test::MockWalActor;
use crate::core::utils::transformers::{
    build_flatbufmeta_with_logmeta, serialize_record_batch_full_ipc,
};
use crate::platform::registry::Registry;
use crate::platform::wal::writer::writer::write_wal_block;

#[derive(Clone, Message)]
#[rtype(result = "()")]
pub enum WalActorAddr {
    Real(Addr<WalActor>),
    #[cfg(test)]
    Mock(Addr<MockWalActor>),
    Empty,
}

pub struct WalActor {
    pub writer: BufWriter<File>,
    pub registry_address: Addr<Registry>,
}

impl Actor for WalActor {
    type Context = Context<Self>;

    fn started(&mut self, _ctx: &mut Self::Context) {
        let registry_address = self.registry_address.clone();
        let address = _ctx.address();
        actix_rt::spawn(async move {
            // let pool = settings_for_spawn.connection_pool().await;
            registry_address.do_send(WalActorAddr::Real(address));
        });
        info!("WAL actor started");
    }
}

impl WalActor {
    pub fn new(registry_address: Addr<Registry>) -> Self {
        info!("Creating new WAL entry actor");
        let file: File = OpenOptions::new()
            .create(true)
            .append(true)
            .open("/tmp/wal_entry.log")
            .expect("Failed to open WAL entry log file");
        WalActor {
            writer: BufWriter::new(file),
            registry_address,
        }
    }
}

// The actor will get messages as RecordBatchWrapper (regex applied/structured), it needs to write them to
// WAL files as Arrow IPC streams.

impl Handler<RecordBatchWrapper> for WalActor {
    type Result = ();

    fn handle(
        &mut self,
        record_batch_wrapper: RecordBatchWrapper,
        _ctx: &mut Self::Context,
    ) -> Self::Result {
        // Convert the RecordBatchWrapper to layout WalBlockHeader
        let metadata_bytes = build_flatbufmeta_with_logmeta(&record_batch_wrapper.metadata);
        info!("Length of metadata = {:?}", &metadata_bytes.len());

        let data_bytes = serialize_record_batch_full_ipc(&record_batch_wrapper);
        info!("Length of actual data: {:?}", &data_bytes.len());

        if let Err(e) = write_wal_block(&mut self.writer, &data_bytes, &metadata_bytes) {
            tracing::error!("Failed to write WAL block: {}", e);
        };
        info!("WAL block written successfully");
    }
}
