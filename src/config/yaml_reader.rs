use crate::config::database::DatabaseSettings;
use crate::config::flight::Flight;
use config::{Config, Environment, File};
use serde_derive::Deserialize;

#[derive(Deserialize)]
pub struct Settings {
    pub database: DatabaseSettings,
    pub server: ServerType,
    pub flight: Flight,
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
        .add_source(Environment::with_prefix("APP").separator("__"))
        .build()
        .unwrap();

    config.try_deserialize().unwrap()
}
