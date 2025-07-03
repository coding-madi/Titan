use crate::config::yaml_reader::Settings;
use crate::core::db::factory::database_factory::{AnyPool, DatabaseFactory};
use sqlx::PgPool;

pub struct PGDatabaseFactory;

#[async_trait::async_trait]
impl DatabaseFactory for PGDatabaseFactory {
    async fn create_pool(&self, config: &Settings) -> AnyPool {
        // get connection string
        let db_url = format!(
            "{}{}:{}/{}",
            "postgres://",
            config.database.host,
            config.database.port,
            config.database.database_name
        );
        let pool = PgPool::connect(&db_url)
            .await
            .expect("Error connecting to Postgres");
        AnyPool::Postgres(pool)
    }
}
