use crate::config::database::DatabaseType;
use crate::config::yaml_reader::Settings;
use crate::core::db::factory::pg_database_factory::PGDatabaseFactory;
use crate::core::db::factory::sqlite_database_factory::SqliteDatabaseFactory;
use crate::core::db::postgres::PostgresSchemaRepository;
use crate::core::db::sqlite::SqliteSchemaRepository;
use sqlx::{PgPool, SqlitePool};
use std::error::Error;
use std::fmt::Debug;
use tonic::async_trait;

#[derive(Clone)]
pub enum AnyPool {
    Postgres(PostgresRepositories),
    Sqlite(SqliteRepositories),
}

#[derive(Clone)]
pub struct PostgresRepositories {
    pub schema_repository: PostgresSchemaRepository,
}

#[derive(Clone)]
pub struct SqliteRepositories {
    pub schema_repository: SqliteSchemaRepository,
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
    async fn execute_query(&self, _query: &str) -> Result<(), Box<dyn Error>> {
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

use crate::core::db::repository::SchemaRepository;
use std::sync::Arc;

pub trait RepositoryProvider: Send + Sync {
    fn schema_repository(&self) -> Arc<dyn SchemaRepository + Send + Sync>;
}

pub struct SqliteRepositoryProvider {
    schema_repo: Arc<dyn SchemaRepository + Send + Sync>,
}
impl SqliteRepositoryProvider {
    pub fn new(pool: SqlitePool) -> Self {
        let repo = SqliteSchemaRepository { sqlite_pool: pool };
        Self {
            schema_repo: Arc::new(repo),
        }
    }
}

impl RepositoryProvider for SqliteRepositoryProvider {
    fn schema_repository(&self) -> Arc<dyn SchemaRepository + Send + Sync> {
        self.schema_repo.clone()
    }
}

pub struct PostgresRepositoryProvider {
    schema_repo: Arc<dyn SchemaRepository + Send + Sync>,
}

impl PostgresRepositoryProvider {
    pub fn new(pool: PgPool) -> Self {
        let repo = PostgresSchemaRepository { pool };
        Self {
            schema_repo: Arc::new(repo),
        }
    }
}

impl RepositoryProvider for PostgresRepositoryProvider {
    fn schema_repository(&self) -> Arc<dyn SchemaRepository + Send + Sync> {
        self.schema_repo.clone()
    }
}
