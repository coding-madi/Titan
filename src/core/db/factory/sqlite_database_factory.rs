use crate::config::yaml_reader::Settings;
use crate::core::db::factory::database_factory::{AnyPool, DatabaseFactory};
use sqlx::SqlitePool;

pub struct SqliteDatabaseFactory;

#[async_trait::async_trait]
impl DatabaseFactory for SqliteDatabaseFactory {
    async fn create_pool(&self, config: &Settings) -> AnyPool {
        let config = config.database.host.clone();
        let connection_string = format!("sqlite://{}", &config);
        let pool = SqlitePool::connect("sqlite::memory:")
            .await
            .expect("Failed to create pool");
        AnyPool::Sqlite(pool)
    }
}
