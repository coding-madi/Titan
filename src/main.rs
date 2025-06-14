use actix_web::{App, HttpServer, web};

#[tokio::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| App::new().route("/", web::get().to(|| async { "Hello, World!" })))
        .bind("0.0.0.0:8888")?
        .run()
        .await
}
