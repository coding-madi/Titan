use std::error::Error;
use std::sync::Arc;
use tonic::async_trait;

pub struct Schema {
    pub flight_name: String,
    pub schema: Arc<arrow::datatypes::Schema>,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
}

#[async_trait]
pub trait SchemaRepository {
    async fn insert_schema(&self, schema: Schema) -> Result<(), Box<dyn Error>>;

    async fn get_schema(&self, flight_name: &str) -> Result<Option<Schema>, Box<dyn Error>>;
    async fn list_schemas(&self) -> Result<Vec<Schema>, Box<dyn Error>>;
    async fn delete_schema(&self, flight_name: &str) -> Result<(), Box<dyn Error>>;
}
