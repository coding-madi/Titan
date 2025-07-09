use poros::config::yaml_reader::ServerType::{ALL, INJEST, QUERY};
use poros::config::yaml_reader::read_configuration;
use poros::version::print_version;
use std::sync::Arc;

use clap::Parser;
use poros::application::actors::db::ReposReady;
use poros::application::actors::init::init_actors;
use poros::core::db::init_repositories;
use poros::core::logging::file_writer::FileWriter;
use poros::core::logging::subscriber::{get_subscribers, init_subscriber};
use poros::platform::actor_factory::InjestSystem;
use poros::servers::full_server::FullServer;
use poros::servers::injest_server::InjestServer;
use poros::servers::query_server::QueryServer;
use poros::servers::server::PorosServer;

/// Simple Rust application demonstrating version display with clap.
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)] // Use `version` here!
struct Args {
    // You can add other arguments here if your application needs them
    // For example:
    // #[arg(short, long, default_value_t = 1, help = "Number of times to greet")]
    // count: u8,
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let _args = Args::parse();
    print_version();
    // TODO: implement log rotation
    // Log file appender and tracer configuration
    let file_writer = FileWriter::new("poros.log");
    let subscriber = get_subscribers("poros", "INFO", file_writer);
    init_subscriber(subscriber);
    let config = read_configuration();

    let repositories = init_repositories(&config).await;

    let actor_registry: Arc<dyn InjestSystem> = init_actors(&config, repositories.clone()).await;

    actor_registry.get_db().do_send(ReposReady {
        repos: repositories.clone(),
    });

    match config.server {
        // Flight server initialization
        INJEST => {
            let injest_server = InjestServer {
                actor_registry,
                _shutdown_handler: None,
                repos: repositories.clone(),
            };
            let _ = InjestServer::start_server(injest_server, &config).await;
        }
        // Query server initialization
        QUERY => {
            let query_server = QueryServer { actor_registry };
            let _ = QueryServer::start_server(query_server, &config).await;
        }
        // Both query and flight servers initialization
        ALL => {
            let injest_server: InjestServer = InjestServer {
                actor_registry: actor_registry.clone(),
                _shutdown_handler: None,
                repos: repositories.clone(),
            };
            let query_server: QueryServer = QueryServer { actor_registry };
            let all = FullServer {
                repos: repositories.clone(),
                query_server: Some(query_server),
                injest_server: Some(injest_server),
                _injest_server_shutdown_sender: None,
            };

            let _ = FullServer::start_server(all, &config).await;
        }
    }
    Ok(())
}
