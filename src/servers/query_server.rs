use crate::api::http::health::get_health_endpoint_factory;
use crate::api::http::regex::{get_all_flights_factory, submit_new_pattern_factory};
use crate::config::yaml_reader::Settings;
use crate::core::error::exception::server_error::ServerError;
use crate::platform::actor_factory::InjestSystem;
use crate::servers::server::PorosServer;
use actix_web::web::{Data, ServiceConfig};
use actix_web::{App, HttpServer, web};
use std::net::TcpListener;
use std::sync::Arc;
use std::time::Duration;
use tokio::signal::ctrl_c;
use tokio::sync::oneshot;
use tokio::sync::oneshot::Sender;
use tracing::info;
use tracing_actix_web::TracingLogger;
use utoipa::OpenApi;
use utoipa_redoc::{Redoc, Servable};
use utoipa_swagger_ui::SwaggerUi;

pub struct QueryServer {
    pub actor_registry: Arc<dyn InjestSystem>,
}

impl PorosServer for QueryServer {
    type Error = ServerError;

    fn configure_routes(_config: &mut ServiceConfig)
    where
        Self: Sized,
    {
        _config
            .service(
                web::scope("/api/v1")
                    .service(get_health_endpoint_factory())
                    .service(submit_new_pattern_factory())
                    .service(get_all_flights_factory())
                    .service(Redoc::with_url("/redoc", ApiDoc::openapi())),
            )
            .service(
                SwaggerUi::new("/swagger-ui/{_:.*}")
                    .url("/api-docs/openapi.json", ApiDoc::openapi()),
            );
    }

    async fn bootstrap_server(
        self,
        config: &Settings,
    ) -> Result<
        (
            Self,
            impl Future<Output = Result<(), Self::Error>> + Send,
            Option<Sender<()>>,
        ),
        Self::Error,
    >
    where
        Self: Sized,
    {
        let listener = create_listener(config);
        let actor_registry = self.actor_registry.clone();
        match listener {
            Ok(listener) => {
                let server = HttpServer::new(move || {
                    App::new()
                        .wrap(TracingLogger::default())
                        .configure(Self::configure_routes)
                        .app_data(Data::new(actor_registry.clone()))
                })
                .listen(listener)?
                .workers(num_cpus::get())
                .keep_alive(Duration::from_secs(75))
                .run();

                let (shutdown_trigger, shutdown_receiver) = oneshot::channel::<()>();

                tokio::spawn(async move {
                    let _ = shutdown_hook(shutdown_receiver).await;
                });

                let server_future = async move {
                    let _shutdown_handle = tokio::spawn(async move {
                        block_until_shutdown_signal().await;
                        tracing::info!("shutdown signal received");
                        let _ = shutdown_trigger.send(());
                    });

                    let _ = server.await;
                    Ok(())
                };

                let query_server = QueryServer {
                    actor_registry: self.actor_registry,
                };
                Ok((query_server, server_future, None))
            }
            Err(error) => {
                panic!("Server fatal error - {error}");
            }
        }
    }

    async fn start_server(self, config: &Settings)
    where
        Self: Sized,
    {
        let (_server_instance, server_run_future, _shutdown_sender) = self
            .bootstrap_server(config)
            .await
            .expect("bootstrap failed");
        server_run_future.await.expect("server run failed");
    }
}

fn create_listener(_config: &Settings) -> Result<TcpListener, ServerError> {
    let listener = TcpListener::bind("127.0.0.1:8888").expect("Port busy. Please try again");
    Ok(listener)
}

async fn shutdown_hook(shutdown_receiver: oneshot::Receiver<()>) {
    tokio::select! {
        _ = shutdown_receiver => {
            info!("Query server is shutting down!");
        }
    }
}

#[derive(OpenApi)]
#[openapi(
    paths(
        crate::api::http::health::get_health_endpoint_factory,
        crate::api::http::regex::get_all_flights_factory,
        crate::api::http::regex::submit_new_pattern_factory
    ),
    components(
        schemas(crate::api::http::regex::FlightsList)
    ),
    tags(
        (name = "Health", description = "Health check"),
        (name = "Flights", description = "Flight listing"),
        (name = "Submit Patterns", description = "Submit new grok patterns")  
    ),
    servers(
        (url = "/api/v1", description = "API base path")
    )
)]
pub struct ApiDoc; // <--- Must be public, must derive OpenApi

pub async fn block_until_shutdown_signal() {
    use tokio::signal::unix::{SignalKind, signal};
    use tracing::info;
    let mut sigterm =
        signal(SignalKind::terminate()).expect("SIGTERM signal triggered before TERMINATE");

    tokio::select! {
        _ = ctrl_c() => info!("Received SIGINT"),
        _ = sigterm.recv() => info!("Received SIGTERM"),
    }
}
