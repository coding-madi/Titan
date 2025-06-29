use crate::config::yaml_reader::Settings;
use crate::platform::actor_factory::{ActorFactory, InjestRegistry, InjestSystem};
use std::sync::Arc;

pub async fn init_actors(config: &Settings) -> Arc<dyn InjestSystem> {
    // Cloning Addr is shallow, it is Arc protected pointer clone
    let iceberg_actor = ActorFactory::iceberg_actor();
    let wal_actor = ActorFactory::wal_actor(iceberg_actor.clone());
    let parser = ActorFactory::parser_actor(2, wal_actor.clone());
    let broadcaster = ActorFactory::broadcast_actor(parser.clone());
    let db = ActorFactory::db_actor(config).await;

    let _actor_registry = InjestRegistry {
        db,
        broadcaster: broadcaster.clone(),
        parser,
        wal_actor,
        iceberg_actor,
    };
    Arc::new(_actor_registry)
}
