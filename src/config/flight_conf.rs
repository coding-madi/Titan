use serde_derive::Deserialize;

#[derive(Deserialize)]
pub struct FlightConf {
    pub address: String,
    pub port: i32,
}
