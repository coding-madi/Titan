use actix_web::web::ServiceConfig;
use tokio::sync::oneshot::Sender;

use crate::config::yaml_reader::Settings;

pub trait PorosServer {
    type Error: std::error::Error + Send + Sync + 'static;

    fn configure_routes(config: &mut ServiceConfig)
    where
        Self: Sized;

    fn bootstrap_server(
        self,
        config: &Settings,
    ) -> impl Future<
        Output = Result<
            (
                Self,
                impl Future<Output = Result<(), Self::Error>> + Send,
                Option<Sender<()>>,
            ),
            Self::Error,
        >,
    > + Send
    where
        Self: Sized;

    fn start_server(self, config: &Settings) -> impl Future<Output = ()> + Send
    where
        Self: Sized;
}
