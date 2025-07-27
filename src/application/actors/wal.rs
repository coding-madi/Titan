use std::fs::{File, OpenOptions};
use std::io::{BufWriter, Write};
use std::time::Duration;

use actix::{Actor, Addr, AsyncContext, Context, Handler, Message};
use actix_rt::spawn;
use tracing::info;

use crate::application::actors::broadcast::RecordBatchWrapper;
use crate::application::actors::iceberg::{FlushInstruction, IcebergActorAddr};
#[cfg(test)]
pub(crate) use crate::application::actors::wal_test::test::MockWalActor;
use crate::core::utils::transformers::{
    build_flatbufmeta_with_logmeta, serialize_record_batch_full_ipc,
};
use crate::platform::registry::{FetchIcebergActor, Registry};
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
    pub size: u128,
    pub wal_paths: Vec<String>,
    pub current_index: usize,
}

impl WalActor {
    pub fn new(registry_address: Addr<Registry>) -> Self {
        let wal_paths = vec![
            "/tmp/wal0.log".to_string(),
            "/tmp/wal1.log".to_string(),
            "/tmp/wal2.log".to_string(),
        ];
        let initial_index = 0;

        let file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&wal_paths[initial_index])
            .expect("Failed to open WAL log file");

        WalActor {
            writer: BufWriter::new(file),
            registry_address,
            size: 0,
            wal_paths,
            current_index: initial_index,
        }
    }

    fn rotate_file(&mut self) {
        self.current_index = (self.current_index + 1) % self.wal_paths.len();
        let next_path = &self.wal_paths[self.current_index];

        let file = OpenOptions::new()
            .write(true)
            .truncate(true)
            .create(true)
            .open(next_path)
            .expect("Failed to rotate WAL file");

        self.writer = BufWriter::new(file);
        self.size = 0;

        info!("Rotated WAL to file: {}", next_path);
    }
}

impl Actor for WalActor {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        let registry_address = self.registry_address.clone();
        let address = ctx.address();
        spawn(async move {
            registry_address.do_send(WalActorAddr::Real(address));
        });
        info!("WAL actor started");
    }

    fn stopped(&mut self, _ctx: &mut Self::Context) {
        self.writer.flush().unwrap();
        info!("WAL actor stopped");
    }
}

impl Handler<RecordBatchWrapper> for WalActor {
    type Result = ();

    fn handle(&mut self, record_batch_wrapper: RecordBatchWrapper, ctx: &mut Self::Context) -> Self::Result {
        let metadata_bytes = build_flatbufmeta_with_logmeta(&record_batch_wrapper.metadata);
        let data_bytes = serialize_record_batch_full_ipc(&record_batch_wrapper);

        if let Err(e) = write_wal_block(&mut self.writer, &data_bytes, &metadata_bytes) {
            tracing::error!("Failed to write WAL block: {}", e);
            return;
        }

        self.size += data_bytes.len() as u128 + metadata_bytes.len() as u128;
        info!("Wrote {} bytes to WAL file", self.size);

        // If threshold exceeded, send flush + rotate
        if self.size > 400_000_000 {
            let registry_address = self.registry_address.clone();
            let myself = ctx.address();

            spawn(async move {
                match registry_address.send(FetchIcebergActor).await.unwrap_or(Err(())) {
                    Ok(IcebergActorAddr::Real(iceberg)) => {
                        let _ = iceberg.send(FlushInstruction {}).await;
                        info!("Sent flush instruction to Iceberg actor");
                    }
                    _ => info!("Iceberg actor not found"),
                }

                myself.do_send(RotateWAL {});
            });
        }
    }
}

#[derive(Message)]
#[rtype(result = "()")]
struct RotateWAL;

impl Handler<RotateWAL> for WalActor {
    type Result = ();

    fn handle(&mut self, _msg: RotateWAL, ctx: &mut Self::Context) -> Self::Result {
        ctx.run_later(Duration::from_secs(0), |actor, _ctx| {
            actor.rotate_file();
        });
    }
}