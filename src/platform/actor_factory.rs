use crate::application::actors::broadcast::Broadcaster;
use crate::application::actors::db::DbActor;
use crate::application::actors::flight_registry::FlightRegistry;
use crate::application::actors::iceberg::IcebergWriter;
use crate::application::actors::parser::ParsingActor;
use crate::application::actors::wal::WalEntry;
use crate::config::yaml_reader::Settings;
use crate::core::db::factory::database_factory::RepositoryProvider;
use actix::{Actor, Addr};
use std::collections::HashMap;
use std::sync::Arc;

pub struct ActorFactory;

impl ActorFactory {
    pub fn broadcast_actor(parser_actor: Vec<Addr<ParsingActor>>) -> Addr<Broadcaster> {
        let broadcast_actor = Broadcaster::new(parser_actor);
        broadcast_actor.start()
    }

    pub fn parser_actor(shards: u8, wal_actor: Addr<WalEntry>) -> Vec<Addr<ParsingActor>> {
        let mut list_of_parsers = vec![];
        for _ in 0..shards {
            let map = HashMap::new();

            let actor = ParsingActor::start(ParsingActor {
                patterns: map,          // Initialize with an empty regex,
                schema: HashMap::new(), // Initialize with an empty Schema
                log_writer: wal_actor.clone(),
            });
            list_of_parsers.push(actor);
        }
        list_of_parsers
    }

    pub fn wal_actor(iceberg: Addr<IcebergWriter>) -> Addr<WalEntry> {
        let wal_actor = WalEntry::new(iceberg);
        wal_actor.start()
    }

    pub fn iceberg_actor() -> Addr<IcebergWriter> {
        let iceberg_actor = IcebergWriter::default();
        iceberg_actor.start()
    }

    // This actor is lazy initialized
    pub async fn db_actor(config: &Settings, repos: Arc<dyn RepositoryProvider>) -> Addr<DbActor> {
        let db_actor = DbActor::new(config.database.clone(), repos).await;
        db_actor.start()
    }

    pub async fn flight_registry_actor(_config: &Settings) -> Addr<FlightRegistry> {
        let flight_registry_actor = FlightRegistry::new().await;
        flight_registry_actor.start()
    }
}

pub struct InjestRegistry {
    pub db: Addr<DbActor>,
    pub broadcaster: Addr<Broadcaster>,
    pub parser: Vec<Addr<ParsingActor>>, // sharded for performance
    pub wal_actor: Addr<WalEntry>,
    pub iceberg_actor: Addr<IcebergWriter>,
    pub flight_registry: Addr<FlightRegistry>,
}

pub trait InjestSystem: Sync + Send {
    fn get_db(&self) -> Addr<DbActor>;
    fn get_parser(&self) -> Vec<Addr<ParsingActor>>;
    fn get_wal_actor(&self) -> Addr<WalEntry>;
    fn get_iceberg_actor(&self) -> Addr<IcebergWriter>;
    fn get_broadcaster_actor(&self) -> Addr<Broadcaster>;
    fn get_flight_registry_actor(&self) -> Addr<FlightRegistry>;
}

impl InjestSystem for InjestRegistry {
    fn get_db(&self) -> Addr<DbActor> {
        self.db.clone() // This clone is cheap and not an antipattern, in this context.
    }

    fn get_parser(&self) -> Vec<Addr<ParsingActor>> {
        self.parser.clone()
    }

    fn get_wal_actor(&self) -> Addr<WalEntry> {
        self.wal_actor.clone()
    }

    fn get_iceberg_actor(&self) -> Addr<IcebergWriter> {
        self.iceberg_actor.clone()
    }

    fn get_broadcaster_actor(&self) -> Addr<Broadcaster> {
        self.broadcaster.clone()
    }

    fn get_flight_registry_actor(&self) -> Addr<FlightRegistry> {
        self.flight_registry.clone()
    }
}
