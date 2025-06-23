use poros::config::yaml_reader::ServerType::INJEST;
use poros::servers::injest_server::InjestServer;
use poros::servers::server::Server;
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
    let args = Args::parse();
    print_version();
    // TODO: implement log rotation
    let file_writer = FileWriter::new("poros.log");
    let subscriber = get_subscribers("poros", "INFO", file_writer);
    init_subscriber(subscriber);

    let config = read_configuration();

    match config.server {
        INJEST => {
            let _ = InjestServer::start_server(&config).await;
        }
        _ => {
            info!("Query server is not implemented yet");
            return Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Query server is not implemented yet",
            ));
        }
    }
    Ok(())
}
