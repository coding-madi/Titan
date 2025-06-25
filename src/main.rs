use poros::config::yaml_reader::ServerType::{ALL, INJEST, QUERY};
use poros::servers::injest_server::injest_server::InjestServer;
use poros::servers::server::PorosServer;
use poros::version::print_version;
use poros::{
    config::yaml_reader::read_configuration,
    logging::{
        file_writer::FileWriter,
        subscriber::{get_subscribers, init_subscriber},
    },
};
use tracing::info;

use clap::Parser;
use poros::servers::full_server::full_server::FullServer;
use poros::servers::query_server::query_server::QueryServer;

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
    let file_writer = FileWriter::new("poros.log");
    let subscriber = get_subscribers("poros", "INFO", file_writer);
    init_subscriber(subscriber);

    let config = read_configuration();
    let pool = config.database.connection_pool().await;
    info!(
        "Postgres database connection pool: {}",
        &config.database.database_name
    );
    match config.server {
        INJEST => {
            let injest_server: InjestServer = InjestServer {
                _shutdown_handler: None,
                pool: pool.clone(),
            };
            let _ = InjestServer::start_server(injest_server, &config).await;
        }
        QUERY => {
            let query_server: QueryServer = QueryServer {};
            let _ = QueryServer::start_server(query_server, &config).await;
        }
        ALL => {
            let injest_server: InjestServer = InjestServer {
                _shutdown_handler: None,
                pool: pool.clone(),
            };
            let query_server: QueryServer = QueryServer {};
            let all = FullServer {
                pool: pool.clone(),
                query_server: Some(query_server),
                injest_server: Some(injest_server),
                injest_server_shutdown_sender: None,
            };

            let _ = FullServer::start_server(all, &config).await;
        }
    }
    Ok(())
}
