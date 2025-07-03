use crate::core::db::repository::{Schema, SchemaRepository};
use async_trait::async_trait;
use sqlx::SqlitePool;
use std::error::Error;

pub struct Sqlite {
    sqlite_pool: SqlitePool,
}

#[async_trait]
impl SchemaRepository for Sqlite {
    async fn insert_schema(&self, schema: Schema) -> Result<(), Box<dyn Error>> {
        todo!()
    }

    async fn get_schema(&self, flight_name: &str) -> Result<Option<Schema>, Box<dyn Error>> {
        todo!()
    }

    async fn list_schemas(&self) -> Result<Vec<Schema>, Box<dyn Error>> {
        todo!()
    }

    async fn delete_schema(&self, flight_name: &str) -> Result<(), Box<dyn Error>> {
        todo!()
    }
}
