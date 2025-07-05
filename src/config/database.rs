use secrecy::{ExposeSecret, SecretString};
use serde_derive::Deserialize;
#[derive(Deserialize, Clone)]
pub struct DatabaseSettings {
    pub database_type: DatabaseType,
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: SecretString, // serde feature needed in secrecy crate
    pub database_name: String,
    pub max_active_connections: u32,
}

#[derive(Deserialize, Clone)]
pub enum DatabaseType {
    Postgres,
    Sqlite,
}

impl DatabaseType {
    pub fn as_str(&self) -> &'static str {
        match self {
            DatabaseType::Postgres => "postgres",
            DatabaseType::Sqlite => "sqlite",
        }
    }
}

impl DatabaseSettings {
    pub fn connection_string(&self) -> String {
        match self.database_type {
            DatabaseType::Postgres => {
                format!(
                    "postgres://{}:{}@{}:{}/{}",
                    self.username,
                    self.password.expose_secret(),
                    self.host,
                    self.port,
                    self.database_name
                )
            }
            DatabaseType::Sqlite => {
                format!("sqlite://{}.db", self.database_name)
            }
        }
    }
}
