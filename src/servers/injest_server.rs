use actix_web::web::ServiceConfig;
use arrow_flight::flight_service_server::FlightServiceServer;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::{
    signal,
    sync::oneshot::{self, Sender},
};
use tonic::transport::Server;
use tracing::info;

pub struct InjestServer {
    pub actor_registry: Arc<dyn InjestSystem>,
    pub pool: Box<dyn DatabasePool + Send + Sync>,
    pub _shutdown_handler: Option<Sender<()>>, // Hold the sender, else the sender is dropped and the receiver receives a None value and stops the server. // TODO: add the postgres database connection pool
}

impl InjestServer {
    pub fn new(
        &self,
        actor_registry: Arc<dyn InjestSystem>,
        pool: Box<dyn DatabasePool>,
        _shutdown_handler: Option<Sender<()>>,
    ) -> Self {
        InjestServer {
            actor_registry,
            pool,
            _shutdown_handler,
        }
    }

    pub async fn start(self, config: &Settings) {
        InjestServer::start_server(self, &config).await;
    }
}

fn get_flight_server_endpoint(config: &Settings) -> SocketAddr {
    let host_name = config.flight.address.trim();
    let host_port = config.flight.port;

    let socker_address: SocketAddr = format!("{}:{}", &host_name, &host_port).parse().expect(
        format!(
            "Failed to parse the flight server address: {}:{} ",
            &host_name, host_port
        )
        .as_str(),
    );
    info!("Flight server address: {}", &socker_address);
    socker_address
}

impl PorosServer for InjestServer {
    type Error = ServerError;

    fn configure_routes(_config: &mut ServiceConfig)
    where
        Self: Sized,
    {
        todo!()
    }

    fn bootstrap_server(
        self: Self,
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
            let flight_address = get_flight_server_endpoint(config);
            let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();
            let log_flight_server = LogFlightServer::new(self.actor_registry.clone());

            let server = Server::builder()
                .max_concurrent_streams(128) // Optional
                .accept_http1(false)
                .add_service(FlightServiceServer::new(log_flight_server))
                .serve_with_shutdown(flight_address, Self::shutdown_handler(shutdown_rx));

            // --- NEW: Spawn a task to listen for Ctrl+C ---
            tokio::spawn(async move {
                signal::ctrl_c().await.expect("Failed to listen for Ctrl+C");
                info!("Ctrl+C received! Initiating graceful shutdown...");
                // Send the shutdown signal to the server
                shutdown_tx.send(()).unwrap_or_else(|_| {
                    tracing::warn!("Failed to send shutdown signal: Receiver already dropped.");
                });
            });

            // Start the shutdown handler as a future, we need to map the error to ServerError and still return a Future<Result<(), ServerError>>
            let server_run_future = async move {
                info!("Flight server started at: {}", flight_address);
                server.await.map_err(|e| ServerError::TonicTransport(e))
            };

            Ok((
                Self {
                    actor_registry: self.actor_registry,
                    pool: self.pool,
                    _shutdown_handler: None,
                },
                server_run_future,
                None::<Sender<()>>,
            ))
        }
    }

    fn start_server(self: Self, config: &Settings) -> impl Future<Output = ()> + Send
    where
        Self: Sized,
    {
        async move {
            match InjestServer::bootstrap_server(self, config).await {
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

use crate::api::flight::service::LogFlightServer;
use crate::config::yaml_reader::Settings;
use crate::core::db::factory::database_factory::DatabasePool;
use crate::core::error::exception::server_error::ServerError;
use crate::platform::actor_factory::InjestSystem;
use crate::servers::server::PorosServer;
use tokio::sync::oneshot::Receiver;

impl InjestServer {
    async fn shutdown_handler(shutdown_rx: Receiver<()>) {
        shutdown_rx.await.ok();
        info!("Flight server shutdown gracefully!")
    }
}
