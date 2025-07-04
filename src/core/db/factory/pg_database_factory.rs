use crate::config::database::DatabaseType;
use crate::config::yaml_reader::Settings;
use crate::core::db::factory::database_factory::{AnyPool, DatabaseFactory, PostgresRepositories};
use crate::core::db::postgres::PostgresSchemaRepository;
use sqlx::PgPool;
use sqlx::migrate::Migrator;
use std::path::Path;
use tracing::info;

pub struct PGDatabaseFactory;

#[async_trait::async_trait]
impl DatabaseFactory for PGDatabaseFactory {
    async fn create_pool(&self, config: &Settings) -> AnyPool {
        // get connection string
        let db_url = config.database.connection_string();
        let pool = PgPool::connect(&db_url)
            .await
            .expect("Error connecting to Postgres");

        let migrator_path = match config.database.database_type {
            DatabaseType::Sqlite => "./migrations/sqlite",
            DatabaseType::Postgres => "./migrations/postgres",
        };

        let migrator = Migrator::new(Path::new(migrator_path)).await.unwrap();
        migrator.run(&pool).await.unwrap();

        info!("PG Migrations applied");

        let schema_repository = PostgresRepositories {
            schema_repository: PostgresSchemaRepository { pool: pool.clone() },
        };
        AnyPool::Postgres(schema_repository)
    }
}
