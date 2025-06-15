use poros::servers::server::Server;
use poros::{
    config::yaml_reader::read_configuration,
    logging::{
        file_writer::FileWriter,
        subscriber::{get_subscribers, init_subscriber},
    },
};

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // TODO: implement log rotation
    let file_writer = FileWriter::new("rs_cd.log");
    let subscriber = get_subscribers("poros", "INFO", file_writer);
    init_subscriber(subscriber);

    let config = read_configuration();

    match config.server {
        poros::config::yaml_reader::ServerType::INJEST => {
            let (_server, server_future, shutdown_handler) =
                poros::servers::injest_server::InjestServer::bootstrap_server(&config)
                    .await
                    .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
            tracing::info!("Starting Injest server at {}", config.flight.address);
            server_future
                .await
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
            tracing::info!("Injest server stopped");
            if let Some(shutdown) = shutdown_handler {
                shutdown.send(()).unwrap_or(());
            }
        }
        _ => {
            tracing::info!("Query server is not implemented yet");
            return Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Query server is not implemented yet",
            ));
        }
    }
    Ok(())
}
