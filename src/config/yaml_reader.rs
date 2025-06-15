use config::{Config, File};
use secrecy::SecretString;
use serde_derive::Deserialize;

#[derive(Deserialize)]
pub struct DatabaseSettings {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: SecretString, // serde feature needed in secrecy crate
    pub database_name: String,
}

#[derive(Deserialize)]
pub struct Settings {
    pub database: DatabaseSettings,
    pub server: ServerType,
    pub flight: Flight,
}

#[derive(Deserialize)]
pub struct Flight {
    pub address: String,
}

#[derive(Deserialize)]
pub enum ServerType {
    QUERY,
    INJEST,
    ALL,
}

pub fn read_configuration() -> Settings {
    let config = Config::builder()
        .add_source(File::with_name("application.yml"))
        .build()
        .unwrap();

    let config = config.try_deserialize().unwrap();
    config
}
