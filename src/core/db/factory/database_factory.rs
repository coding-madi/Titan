use crate::config::database::DatabaseType;
use crate::config::yaml_reader::Settings;
use crate::core::db::factory::pg_database_factory::PGDatabaseFactory;
use crate::core::db::factory::sqlite_database_factory::SqliteDatabaseFactory;
use sqlx::{PgPool, SqlitePool};
use std::error::Error;
use std::fmt::Debug;
use tonic::async_trait;

#[derive(Clone)]
pub enum AnyPool {
    Postgres(PgPool),
    Sqlite(SqlitePool),
}

#[async_trait]
pub trait DatabaseFactory {
    async fn create_pool(&self, config: &Settings) -> AnyPool;
}

pub async fn create(config: &Settings) -> AnyPool {
    match config.database.database_type {
        DatabaseType::Postgres => {
            let pg_factory = PGDatabaseFactory {};
            pg_factory.create_pool(config).await
        }
        DatabaseType::Sqlite => {
            let sqlite_factory = SqliteDatabaseFactory {};
            sqlite_factory.create_pool(config).await
        }
    }
}

/// Trial

#[async_trait]
pub trait DatabasePool: Send + Sync + Debug {
    async fn execute_query(&self, query: &str) -> Result<(), Box<dyn Error>>;
    fn clone_box(&self) -> Box<dyn DatabasePool>;
}

impl Clone for Box<dyn DatabasePool> {
    fn clone(&self) -> Self {
        self.clone_box()
    }
}

#[derive(Debug, Clone)]
pub struct PostgresPool {
    pub pool: PgPool,
}

#[async_trait]
impl DatabasePool for PostgresPool {
    async fn execute_query(&self, query: &str) -> Result<(), Box<dyn Error>> {
        todo!()
    }

    fn clone_box(&self) -> Box<dyn DatabasePool> {
        Box::new(self.clone())
    }
}

#[derive(Debug, Clone)]
pub struct SQLitePool {
    pub pool: SqlitePool,
}

#[async_trait]
impl DatabasePool for SQLitePool {
    async fn execute_query(&self, query: &str) -> Result<(), Box<dyn Error>> {
        println!("{}", query);
        Ok(())
    }

    fn clone_box(&self) -> Box<dyn DatabasePool> {
        Box::new(self.clone())
    }
}
