use crate::config::database_conf::DatabaseConf;
use crate::config::flight_conf::FlightConf;
use config::{Config, Environment, File};
use serde_derive::Deserialize;

#[derive(Deserialize)]
pub struct Settings {
    pub database: DatabaseConf,
    pub server: ServerType,
    pub flight: FlightConf,
    pub storage: Storage,
}

#[derive(Deserialize, Clone)]
pub struct Storage {
    pub warehouse: String,
    pub namespace: String,
    pub object_storage: ObjectStorage,
}

#[derive(Deserialize, Clone)]
pub enum ObjectStorage {
    S3(S3Properties),
    GCS(GCSProperties),
}

#[derive(Deserialize, Clone)]
pub struct S3Properties {
    pub aws_region: String,
    pub aws_endpoint: String,
    pub aws_access_key_id: String,
    pub aws_secret_access_key: String,
    pub path_style_access: bool,
}

#[derive(Deserialize, Clone)]
pub struct GCSProperties {
    pub path_style_access: bool,
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
