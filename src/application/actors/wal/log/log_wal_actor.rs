use actix::{Actor, Addr, AsyncContext, Context, Message};
use std::fs::{File, OpenOptions};
use std::io::{BufWriter, Write};
use tokio::spawn;
use tracing::info;

#[cfg(test)]
pub(crate) use crate::application::actors::tests::wal_test::test::MockWalActor;
use crate::platform::registry::Registry;

#[derive(Clone, Message, Debug)]
#[rtype(result = "()")]
pub enum WalActorWrapper {
    Real(Addr<WalActor>),
    #[cfg(test)]
    Mock(Addr<MockWalActor>),
    Empty,
}

pub struct WalActor {
    pub(crate) writer: BufWriter<File>,
    pub(crate) registry_address: Addr<Registry>,
    pub(crate) size: u128,
    wal_paths: Vec<String>,
    current_index: usize,
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

    pub(crate) fn rotate_file(&mut self) {
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
            registry_address.do_send(WalActorWrapper::Real(address));
        });
        info!("WAL log actor started");
    }

    fn stopped(&mut self, _ctx: &mut Self::Context) {
        self.writer.flush().unwrap();
        info!("WAL actor stopped");
    }
}
