use serde_derive::Deserialize;

#[derive(Deserialize)]
pub struct Flight {
    pub address: String,
    pub port: i32,
}
