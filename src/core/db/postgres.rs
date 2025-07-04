use crate::core::db::repository::{Schema, SchemaRepository};
use arrow_flight::{SchemaAsIpc, SchemaResult};
use arrow_ipc::writer::IpcWriteOptions;
use async_trait::async_trait;
use sqlx::PgPool;
use std::error::Error;
use tonic::Status;
use tracing::info;

#[derive(Clone)]
pub struct PostgresSchemaRepository {
    pub pool: PgPool,
}

#[async_trait]
impl SchemaRepository for PostgresSchemaRepository {
    async fn insert_schema(&self, schema: Schema) -> Result<(), Box<dyn Error>> {
        let ipc_options = IpcWriteOptions::default();
        let schema_ipc = SchemaAsIpc::new(schema.schema.as_ref(), &ipc_options);
        let schema_result = SchemaResult::try_from(schema_ipc)
            .map_err(|e| Status::internal(format!("Failed to convert Schema: {}", e)))
            .unwrap();
        let bytes = schema_result.schema.clone().to_vec(); // Vec<u8>

        info!("Received schema: {:?}", schema_result);

        sqlx::query("INSERT INTO schema (flight_name, schema) VALUES ($1, $2)")
            .bind(&schema.flight_name)
            .bind(bytes)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn get_schema(&self, _flight_name: &str) -> Result<Option<Schema>, Box<dyn Error>> {
        todo!()
    }

    async fn list_schemas(&self) -> Result<Vec<Schema>, Box<dyn Error>> {
        todo!()
    }

    async fn delete_schema(&self, _flight_name: &str) -> Result<(), Box<dyn Error>> {
        todo!()
    }
}
