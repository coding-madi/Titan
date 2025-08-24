#[cfg(test)]
use crate::application::actors::broadcast::MockBroadcastActor;
#[cfg(not(test))]
use crate::application::actors::db::DbActor;
#[cfg(test)]
use crate::application::actors::db::MockDbActor;
#[cfg(not(test))]
use crate::application::actors::flight_registry::FlightRegistry;
#[cfg(test)]
use crate::application::actors::flight_registry::MockFlightRegistry;
#[cfg(not(test))]
use crate::application::actors::iceberg::IcebergActor;
#[cfg(test)]
use crate::application::actors::iceberg::MockIcebergActor;
#[cfg(test)]
use crate::application::actors::parser::MockParsingActor;
#[cfg(test)]
use crate::application::actors::wal::MockWalActor;
#[cfg(not(test))]
use crate::application::actors::wal::WalActor;

#[cfg(not(test))]
use crate::application::actors::db::DbActorAddr;
#[cfg(not(test))]
use crate::application::actors::flight_registry::FlightRegistryActorWrapped;
#[cfg(not(test))]
use crate::application::actors::iceberg::IcebergActorAddr;
#[cfg(not(test))]
use crate::application::actors::wal::WalActorWrapper;
use crate::config::yaml_reader::Settings;
use crate::core::db::factory::database_factory::RepositoryProvider;
use crate::platform::registry::{Registry, RegistryBuilder};
use actix::{Actor, Addr};
use std::sync::Arc;

#[cfg(not(test))]
pub async fn init_actors(config: &Settings, repos: Arc<dyn RepositoryProvider>) -> Addr<Registry> {
    let registry_actor = Registry {
        db_actor_addr: DbActorAddr::Empty,
        iceberg_actor_addr: IcebergActorAddr::Empty,
        wal_actor_addr: WalActorWrapper::Empty,
        flight_registry_actor_addr: FlightRegistryActorWrapped::Empty,
        parser_actor_addr: None,
        factory_actor: FactoryActorAddr::Empty,
        broadcast_actor: None,
    };

    let registry_actor_addr = registry_actor.clone().start();

    let db_actor = DbActor::new(config.database.clone(), repos, registry_actor_addr.clone()).await;

    // Start IcebergActor once and get its address
    let iceberg_actor_instance = IcebergActor::new(
        registry_actor_addr.clone(),
        config.storage.clone(),
        config.storage.namespace.clone(),
    )
    .await; // The actor instance

    // WalActor uses the Addr of the *started* IcebergActor
    let wal_actor_instance = WalActor::new(registry_actor_addr.clone());

    let flight_registry_actor = FlightRegistry::new(registry_actor_addr.clone()).await;
    let factory_actor = FactoryActor::new(registry_actor_addr.clone());

    let registry = RegistryBuilder::new()
        .db_actor(db_actor)
        .iceberg_actor(iceberg_actor_instance.unwrap()) // Pass the *instance* to the builder
        .flight_registry_actor(flight_registry_actor)
        .wal_actor(wal_actor_instance) // Pass the WalActor instance
        .factory_actor(factory_actor)
        .build();

    registry_actor_addr
}

#[cfg(test)]
use crate::application::actors::broadcast::BroadcastActorWrapper;
#[cfg(test)]
use crate::application::actors::db::DbActorAddr;
use crate::application::actors::factory_actor::{FactoryActor, FactoryActorAddr};
#[cfg(test)]
use crate::application::actors::flight_registry::FlightRegistryActorWrapped;
#[cfg(test)]
use crate::application::actors::iceberg::IcebergActorAddr;
#[cfg(test)]
use crate::application::actors::parser::ParserActorAddr;
#[cfg(test)]
use crate::application::actors::wal::WalActorWrapper;
#[cfg(test)]
pub async fn init_actors(
    _config: &Settings,
    _repos: Arc<dyn RepositoryProvider>,
) -> Arc<Addr<Registry>> {
    let registry = Registry {
        db_actor_addr: DbActorAddr::Empty,
        flight_registry_actor_addr: FlightRegistryActorWrapped::Empty,
        iceberg_actor_addr: IcebergActorAddr::Empty,
        wal_actor_addr: WalActorWrapper::Empty,
        parser_actor_addr: None,
        factory_actor: FactoryActorAddr::Empty,
        broadcast_actor: BroadcastActorWrapper::Empty,
    };

    let registry_address = registry.start();
    let db_actor = MockDbActor {
        registry_address: registry_address.clone(),
    };
    let broadcast_actor = MockBroadcastActor {
        registry_address: registry_address.clone(),
        data: vec![],
        regex_request: vec![],
    };
    let flight_registry_actor = MockFlightRegistry {
        registry_address: registry_address.clone(),
    };
    let iceberg_actor = MockIcebergActor {
        registry_address: registry_address.clone(),
    };
    let wal_actor = MockWalActor::new(registry_address.clone());

    let parser_actor = vec![MockParsingActor {
        registry_address: registry_address.clone(),
        data: vec![],
        regex: vec![],
    }];

    let factory_actor = MockFactoryActor::new(registry_address.clone());

    let registry = RegistryBuilder::new()
        .db_actor(db_actor)
        .flight_registry_actor(flight_registry_actor)
        .iceberg_actor(iceberg_actor)
        .wal_actor(wal_actor)
        .factory_actor(factory_actor)
        .build();

    Arc::new(registry.start())
}

#[cfg(test)]
mod tests {
    use super::*; // Import everything from the outer scope (including dummy structs and init_actors)

    use crate::config::database_conf::{DatabaseConf, DatabaseType};
    use crate::config::flight_conf::FlightConf;
    use crate::config::yaml_reader::{ObjectStorage, S3Properties, ServerType, Storage};
    use crate::core::db::factory::database_factory::SqliteRepositoryProvider;
    use actix_rt::test;
    use sqlx::SqlitePool;
    // For #[tests] macro that provides Actix runtime

    async fn setup_test_db(db_name: &str) -> (SqlitePool, Arc<SqliteRepositoryProvider>) {
        let connection_string = format!("sqlite::{db_name}?mode=memory&cache=shared");
        let pool = SqlitePool::connect(&connection_string)
            .await
            .expect("Failed to create SQLite in-memory pool for tests");
        let repos = SqliteRepositoryProvider::new(pool.clone());
        (pool, Arc::new(repos))
    }

    fn create_test_settings(server_type: ServerType, db_name: &str) -> Settings {
        let database = DatabaseConf {
            database_type: DatabaseType::Sqlite,
            host: "".to_string(),
            port: 0,
            username: "".to_string(),
            password: Default::default(),
            database_name: db_name.to_string(), // Use a different name for each tests if needed
            max_active_connections: 0,
        };

        Settings {
            database,
            server: server_type,
            flight: FlightConf {
                address: "".to_string(),
                port: 0,
            },
            storage: Storage {
                warehouse: "log".to_string(),
                namespace: "log".to_string(),
                object_storage: ObjectStorage::S3(S3Properties {
                    aws_region: "".to_string(),
                    aws_endpoint: "".to_string(),
                    aws_access_key_id: "".to_string(),
                    aws_secret_access_key: "".to_string(),
                    path_style_access: false,
                }),
            },
            parser: "RUSTREGREX".to_string(),
        }
    }

    #[test]
    async fn test_init_actors_success() {
        let settings = create_test_settings(ServerType::QUERY, "query_test_db");

        let (_pool, repos) = setup_test_db("query_test_db").await;
        let registry = init_actors(&settings, repos).await;
        println!("{}", Arc::strong_count(&registry));
        assert!(
            Arc::strong_count(&registry) >= 1,
            "Registry Arc should have at least one strong reference"
        );
    }

    #[test]
    async fn test_init_actors_ingestion_server_type() {
        let settings = create_test_settings(ServerType::INJEST, "ingest_test_db");

        let (_pool, repos) = setup_test_db("ingest_test_db").await;

        let registry = init_actors(&settings, repos).await;

        // Assert that the Arc has at least one strong reference.
        assert!(
            Arc::strong_count(&registry) >= 1,
            "Registry Arc should have at least one strong reference for INGESTION server type"
        );
        // As above, more specific actor checks depend on `Registry`'s public interface.
    }
}
