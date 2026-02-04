use crate::config::database::DatabaseType;
use crate::config::yaml_reader::Settings;
use crate::core::db::factory::database_factory::{AnyPool, DatabaseFactory, SqliteRepositories};
use crate::core::db::sqlite::SqliteSchemaRepository;
use sqlx::SqlitePool;
use sqlx::migrate::Migrator;
use std::path::Path;
use tracing::info;

pub struct SqliteDatabaseFactory;

#[async_trait::async_trait]
impl DatabaseFactory for SqliteDatabaseFactory {
    async fn create_pool(&self, config: &Settings) -> AnyPool {
        let connection_string = config.database.connection_string();
        let pool = SqlitePool::connect(&connection_string)
            .await
            .expect("Failed to create pool");
        let repositories = SqliteRepositories {
            schema_repository: SqliteSchemaRepository {
                sqlite_pool: pool.clone(),
            },
        };
        let migrator_path = match config.database.database_type {
            DatabaseType::Sqlite => "./migrations/sqlite",
            DatabaseType::Postgres => "./migrations/postgres",
        };

        let migrator = Migrator::new(Path::new(migrator_path)).await.unwrap();
        migrator.run(&pool).await.unwrap();

        info!("Sqlite Migrations applied");

        AnyPool::Sqlite(repositories)
    }
}
