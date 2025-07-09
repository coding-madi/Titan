use crate::config::database::DatabaseType;
use crate::config::yaml_reader::Settings;
use crate::core::db::factory::database_factory::{
    AnyPool, DatabaseFactory, PostgresRepositoryProvider, RepositoryProvider,
    SqliteRepositoryProvider,
};
use std::sync::Arc;

pub mod factory;
pub mod postgres;
pub mod repository;
pub mod sqlite;

pub async fn init_repositories(config: &Settings) -> Arc<dyn RepositoryProvider> {
    let repositories: Arc<dyn RepositoryProvider> = match config.database.database_type {
        DatabaseType::Postgres => {
            let pg_factory = crate::core::db::factory::pg_database_factory::PGDatabaseFactory {};
            let pool = pg_factory.create_pool(&config).await;
            match pool {
                AnyPool::Postgres(pg) => Arc::new(PostgresRepositoryProvider::new(
                    pg.schema_repository.pool.clone(),
                )),
                AnyPool::Sqlite(_) => panic!("Database type mismatch"),
            }
        }
        DatabaseType::Sqlite => {
            let sqlite_factory =
                crate::core::db::factory::sqlite_database_factory::SqliteDatabaseFactory {};
            let pool = sqlite_factory.create_pool(config).await;
            match pool {
                AnyPool::Postgres(_pg) => panic!("Database type mismatch"),
                AnyPool::Sqlite(sql) => Arc::new(SqliteRepositoryProvider::new(
                    sql.schema_repository.sqlite_pool.clone(),
                )),
            }
        }
    };

    repositories
}
