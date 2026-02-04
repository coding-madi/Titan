use std::fs::OpenOptions;
use std::path::Path;
use secrecy::{ExposeSecret, SecretString};
use serde_derive::Deserialize;
use tracing::error;
use tracing::log::info;

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
                let file_path = format!("{}.db", self.database_name);
                ensure_file_exists(&file_path);
                format!("sqlite://{}", &file_path)
            }
        }
    }
}

fn ensure_file_exists(path: &str)  {
    let file_path = Path::new(path);

    if !file_path.exists() {
        println!("File does not exist. Creating: {}", path);
        let file_path_string = file_path.to_str().unwrap();
        // Create the file (write mode creates if missing)
        match OpenOptions::new()
            .write(true)
            .create(true)
            .open(file_path) {
            Ok(_) => {
                info!("File created");
            },
            Err(e) => {
                panic!("Database file could not be created {}", &file_path_string);

            }
        }
    }
}

