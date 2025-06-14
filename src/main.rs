use actix_web::{App, HttpServer, web};
use poros::logging::{file_writer::FileWriter, subscriber::{get_subscribers, init_subscriber}};

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let file_writer  =FileWriter::new("rs_cd.log");
    let subscriber = get_subscribers("poros", "INFO", file_writer);
    init_subscriber(subscriber);

    HttpServer::new(|| App::new().route("/", web::get().to(|| async { 
        tracing::info!("Hello world invoked");
        "Hello, World!" 
        })))
        .bind("0.0.0.0:8888")?
        .run()
        .await
}
