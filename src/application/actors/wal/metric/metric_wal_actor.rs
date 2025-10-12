#[cfg(test)]
use crate::application::actors::wal::metric::metric_wal_actor::tests::MockWalMetricActor;
use crate::platform::registry::Registry;
use actix::{Actor, Addr, AsyncContext, Context, Message};
use std::fs::{File, OpenOptions};
use std::io::{BufWriter, Write};
use tokio::spawn;
use tracing::info;

#[derive(Clone, Message, Debug)]
#[rtype(result = "()")]
pub enum WalMetricActorWrapper {
    Real(Addr<WalMetricActor>),
    #[cfg(test)]
    Mock(Addr<MockWalMetricActor>),
    Empty,
}

pub struct WalMetricActor {
    pub write: BufWriter<File>,
    pub size: u128,
    pub wal_paths: Vec<String>,
    pub current_index: usize,
    pub registry_address: Addr<Registry>,
}

impl WalMetricActor {
    pub fn new(registry_address: Addr<Registry>) -> Self {
        let wal_paths = vec![
            "/tmp/meter_wal0.log".to_string(),
            "/tmp/meter_wal1.log".to_string(),
            "/tmp/meter_wal2.log".to_string(),
        ];

        let initial_index = 0;

        let file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&wal_paths[initial_index])
            .expect("Unable to open wal");

        WalMetricActor {
            write: BufWriter::new(file),
            size: 0,
            wal_paths,
            current_index: 0,
            registry_address,
        }
    }

    pub fn rotate_file(&mut self) {
        self.current_index = (self.current_index + 1) % self.wal_paths.len();
        let next_path = &self.wal_paths[self.current_index];

        let file = OpenOptions::new()
            .write(true)
            .truncate(true)
            .create(true)
            .open(&next_path)
            .expect("Unable to open wal");

        self.write = BufWriter::new(file);
        self.size = 0;

        info!("Rotated meter log file: {}", next_path);
    }
}

impl Actor for WalMetricActor {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        let registry_address = self.registry_address.clone();
        let address = ctx.address();
        spawn(async move {
            registry_address.do_send(WalMetricActorWrapper::Real(address));
        });
        info!("Metric WAL actor started");
    }

    fn stopped(&mut self, _ctx: &mut Self::Context) {
        self.write.flush().unwrap();
        info!("WAL meter actor stopped");
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    #[derive(Debug, Clone)]
    pub struct MockWalMetricActor {}

    impl MockWalMetricActor {
        pub fn new() -> Self {
            MockWalMetricActor {}
        }
    }

    impl Actor for MockWalMetricActor {
        type Context = Context<Self>;

        fn started(&mut self, _ctx: &mut Self::Context) {
            todo!()
        }
    }
}
