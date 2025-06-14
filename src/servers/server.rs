use std::io;

use tokio::sync::oneshot::Sender;

use crate::config::yaml_reader::Settings;

pub trait Server {
    async fn configure_routes(config: &Settings)
    where
        Self: Sized;

    fn bootstrap_server(
        config: &Settings,
    ) -> Result<
        (
            Self,
            impl Future<Output = io::Result<()>> + Send,
            Option<Sender<()>>,
        ),
        io::Error,
    >
    where
        Self: Sized;

    async fn start_server(config: &Settings)
    where
        Self: Sized;
}
