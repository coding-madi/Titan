use serde_derive::Deserialize;

#[derive(Deserialize, Clone)]
pub struct FlightConf {
    pub address: String,
    pub port: i32,
}
