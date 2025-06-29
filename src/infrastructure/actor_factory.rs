use crate::application::actors::broadcast::Broadcaster;
use crate::application::actors::db::DbActor;
use crate::application::actors::iceberg::IcebergWriter;
use crate::application::actors::parser::{ParsingActor, Pattern};
use crate::application::actors::wal::WalEntry;
use crate::config::yaml_reader::Settings;
use actix::{Actor, Addr};
use std::collections::HashMap;

pub struct ActorFactory;

impl ActorFactory {
    pub fn broadcast_actor(parser_actor: Vec<Addr<ParsingActor>>) -> Addr<Broadcaster> {
        let broadcast_actor = Broadcaster::new(parser_actor);
        broadcast_actor.start()
    }

    pub fn parser_actor(shards: u8, wal_actor: Addr<WalEntry>) -> Vec<Addr<ParsingActor>> {
        let mut list_of_parsers = vec![];
        for _ in 0..shards {
            let mut map = HashMap::new();
            let _ = map.insert(
                String::from("/benchmark/batch_0"),
                Pattern::Regex(".*".to_string()),
            );
            let actor = ParsingActor::start(ParsingActor {
                patterns: map,          // Initialize with an empty regex,
                schema: HashMap::new(), // Initialize with an empty schema
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
    pub async fn db_actor(config: &Settings) -> Addr<DbActor> {
        let db_actor = DbActor::new(config.database.clone()).await;
        db_actor.start()
    }
}

pub struct InjestRegistry {
    pub db: Addr<DbActor>,
    pub broadcaster: Addr<Broadcaster>,
    pub parser: Vec<Addr<ParsingActor>>, // sharded for performance
    pub wal_actor: Addr<WalEntry>,
    pub iceberg_actor: Addr<IcebergWriter>,
}

pub trait InjestSystem: Sync + Send {
    fn get_db(&self) -> Addr<DbActor>;
    fn get_parser(&self) -> Vec<Addr<ParsingActor>>;
    fn get_wal_actor(&self) -> Addr<WalEntry>;
    fn get_iceberg_actor(&self) -> Addr<IcebergWriter>;
    fn get_broadcaster_actor(&self) -> Addr<Broadcaster>;
}

impl InjestSystem for InjestRegistry {
    fn get_db(&self) -> Addr<DbActor> {
        self.db.clone() // This clone is cheap and not an anti-pattern, in this context.
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
}
