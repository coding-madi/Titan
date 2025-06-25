use secrecy::{ExposeSecret, SecretString};
use serde_derive::Deserialize;
use sqlx::postgres::PgPoolOptions;
use sqlx::{Pool, Postgres};
#[derive(Deserialize, Clone)]
pub struct DatabaseSettings {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: SecretString, // serde feature needed in secrecy crate
    pub database_name: String,
    pub max_active_connections: u32,
}

impl DatabaseSettings {
    pub fn connection_string(&self) -> String {
        format!(
            "postgres://{}:{}@{}:{}/{}",
            self.username,
            self.password.expose_secret(),
            self.host,
            self.port,
            self.database_name
        )
    }

    pub async fn connection_pool(&self) -> Pool<Postgres> {
        let connection_string = self.connection_string();
        PgPoolOptions::new()
            .max_connections(self.max_active_connections)
            .connect(connection_string.as_str())
            .await
            .expect("Failed to connect to postgres")
    }
}
