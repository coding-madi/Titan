use crate::config::yaml_reader::Settings;
use crate::core::db::factory::database_factory::RepositoryProvider;
use crate::platform::actor_factory::{ActorFactory, InjestRegistry, InjestSystem};
use std::sync::Arc;

pub async fn init_actors(
    config: &Settings,
    repos: Arc<dyn RepositoryProvider>,
) -> Arc<dyn InjestSystem> {
    let iceberg_actor = ActorFactory::iceberg_actor();
    let wal_actor = ActorFactory::wal_actor(iceberg_actor.clone());
    let parser = ActorFactory::parser_actor(20, wal_actor.clone());
    let broadcaster = ActorFactory::broadcast_actor(parser.clone());
    let db = ActorFactory::db_actor(config, repos).await;
    let flight_registry = ActorFactory::flight_registry_actor(config).await;

    let _actor_registry = InjestRegistry {
        db,
        broadcaster: broadcaster.clone(),
        parser,
        wal_actor,
        iceberg_actor,
        flight_registry,
    };
    Arc::new(_actor_registry)
}
