use arrow_flight::{SchemaAsIpc, SchemaResult};
use arrow_ipc::writer::IpcWriteOptions;
use std::error::Error;
use std::sync::Arc;
use tonic::{Status, async_trait};

pub struct Schema {
    pub flight_name: String,
    pub schema: Arc<arrow::datatypes::Schema>,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
}

#[async_trait]
pub trait SchemaRepository: Send + Sync {
    async fn insert_schema(&self, schema: Schema) -> Result<(), Box<dyn Error>>;
    async fn get_schema(&self, flight_name: &str) -> Result<Option<Schema>, Box<dyn Error>>;
    async fn list_schemas(&self) -> Result<Vec<Schema>, Box<dyn Error>>;
    async fn delete_schema(&self, flight_name: &str) -> Result<(), Box<dyn Error>>;

    async fn convert_schema_to_bytes(&self, schema: &Schema) -> Result<Vec<u8>, Box<dyn Error>> {
        let ipc_options = IpcWriteOptions::default();
        let schema_ipc = SchemaAsIpc::new(schema.schema.as_ref(), &ipc_options);
        let schema_result = SchemaResult::try_from(schema_ipc)
            .map_err(|e| Status::internal(format!("Failed to convert Schema: {}", e)))
            .unwrap();
        let bytes = schema_result.schema.clone().to_vec();
        Ok(bytes)
    }
}
