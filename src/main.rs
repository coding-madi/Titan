use poros::config::yaml_reader::ServerType::INJEST;
use poros::servers::injest_server::InjestServer;
use poros::servers::server::Server;
use poros::{
    config::yaml_reader::read_configuration,
    logging::{
        file_writer::FileWriter,
        subscriber::{get_subscribers, init_subscriber},
    },
};
use tracing::info;
#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // TODO: implement log rotation
    let file_writer = FileWriter::new("rs_cd.log");
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
