use std::io;

use tokio::sync::oneshot::Sender;

use crate::config::yaml_reader::Settings;

pub trait Server {
    fn configure_routes(config: &Settings)
    where
        Self: Sized;

    fn bootstrap_server(
        config: &Settings,
    ) -> impl Future<
        Output = Result<
            (
                Self,
                impl Future<Output = io::Result<()>> + Send,
                Option<Sender<()>>,
            ),
            io::Error,
        >,
    > + Send
    where
        Self: Sized;

    fn start_server(config: &Settings) -> impl Future<Output = ()> + Send
    where
        Self: Sized;
}
