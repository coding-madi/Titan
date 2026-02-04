use crate::core::db::repository::{Schema, SchemaRepository};
use async_trait::async_trait;
use sqlx::SqlitePool;
use std::error::Error;

#[derive(Clone)]
pub struct SqliteSchemaRepository {
    pub sqlite_pool: SqlitePool,
}

#[async_trait]
impl SchemaRepository for SqliteSchemaRepository {
    async fn insert_schema(&self, schema: Schema) -> Result<(), Box<dyn Error>> {
        let bytes = self.convert_schema_to_bytes(&schema).await?;

        sqlx::query("INSERT INTO schema (flight_name, schema) VALUES (?, ?)")
            .bind(&schema.flight_name)
            .bind(bytes)
            .execute(&self.sqlite_pool)
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
