use std::sync::Arc;

use actix::Actor;
use arrow_flight::flight_service_server::FlightServiceServer;
use tokio::{
    signal,
    sync::oneshot::{self, Sender},
};
use tracing::info;

use crate::{
    actors::broadcast::Broadcaster,
    config::yaml_reader::Settings,
    exception::server_error::ServerError,
    servers::{server::Server, service::flight_service::LogFlightServer},
};
pub struct InjestServer {
    _shutdown_handler: Option<Sender<()>>, // Hold the sender, else the sender is dropped and the receiver receives a None value and stops the server.
                                           // TODO: add the postgres database connection pool
}

impl Server for InjestServer {
    type Error = ServerError;

    fn configure_routes(_config: &Settings)
    where
        Self: Sized,
    {
        todo!()
    }

    fn bootstrap_server(
        config: &Settings,
    ) -> impl Future<
        Output = Result<
            (
                Self,
                impl Future<Output = Result<(), Self::Error>> + Send, // The Http server returns a Future<Result<()>, Err>, so we have to pass a dummy future for Tonic
                Option<Sender<()>>,
            ),
            Self::Error,
        >,
    > + Send
    where
        Self: Sized,
    {
        async move {
            let s = config.flight.address.trim();
            let addr = config.flight.address.parse().expect(
                format!(
                    "Failed to parse the flight server address: {}",
                    &config.flight.address
                )
                .as_str(),
            );

            let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();

            let broadcast_actor = Broadcaster::new().start();
            let log_flight_server = LogFlightServer::new(Arc::new(broadcast_actor));
            let server = tonic::transport::Server::builder()
                .max_concurrent_streams(128) // Optional
                .accept_http1(false)
                .add_service(FlightServiceServer::new(log_flight_server))
                .serve_with_shutdown(addr, Self::shutdown_handler(shutdown_rx));

            // --- NEW: Spawn a task to listen for Ctrl+C ---
            tokio::spawn(async move {
                signal::ctrl_c().await.expect("Failed to listen for Ctrl+C");
                tracing::info!("Ctrl+C received! Initiating graceful shutdown...");
                // Send the shutdown signal to the server
                shutdown_tx.send(()).unwrap_or_else(|_| {
                    tracing::warn!("Failed to send shutdown signal: Receiver already dropped.");
                });
            });

            // Start the shutdown handler as a future, we need to map the error to ServerError and still return a Future<Result<(), ServerError>>
            let server_run_future = async move {
                info!("Flight server started at: {}", s);
                server.await.map_err(|e| ServerError::TonicTransport(e))
            };

            Ok((
                InjestServer {
                    _shutdown_handler: None,
                },
                server_run_future,
                None::<oneshot::Sender<()>>,
            ))
        }
    }

    fn start_server(config: &Settings) -> impl Future<Output = ()> + Send
    where
        Self: Sized,
    {
        async move {
            match InjestServer::bootstrap_server(config).await {
                Ok((_server_instance, server_run_future, _shutdown_sender)) => {
                    println!("Server successfully bootstrapped. Running...");
                    server_run_future.await.unwrap_or_else(|e| {
                        eprintln!("Server runtime error: {}", e);
                    });
                    println!("Server has shut down.");
                }
                Err(e) => {
                    eprintln!("Failed to bootstrap server: {}", e);
                }
            }
        }
    }
}

use tokio::sync::oneshot::Receiver;
impl InjestServer {
    async fn shutdown_handler(shutdown_rx: Receiver<()>) {
        shutdown_rx.await.ok();
        info!("Flight server shutdown gracefully!")
    }
}
