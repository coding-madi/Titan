// src/platform/injest_system.rs

use actix::{Actor, Addr, Handler};
use std::collections::HashMap;
use std::sync::Arc;

use crate::api::http::regex::RegexRequest;
use crate::application::actors::broadcast::{Broadcaster, RecordBatchWrapper};
use crate::application::actors::db::{DbActor, GetPatternsForTenant, ReposReady, SaveSchema};
use crate::application::actors::flight_registry::{
    CheckFlight, FlightRegistry, ListFlights, RegisterFlight,
};
use crate::application::actors::iceberg::{FlushInstruction, IcebergWriter};
use crate::application::actors::parser::ParsingActor;
use crate::application::actors::wal::WalEntry;
use crate::config::yaml_reader::Settings;
use crate::core::db::factory::database_factory::RepositoryProvider;

// --- Actor Factory --- //
pub struct ActorFactory;

impl ActorFactory {
    pub fn broadcast_actor(parser_actors: Vec<Addr<ParsingActor<WalEntry>>>) -> Addr<Broadcaster> {
        Broadcaster::new(parser_actors).start()
    }

    pub fn parser_actor(
        shards: u8,
        wal_actor: Addr<WalEntry>,
    ) -> Vec<Addr<ParsingActor<WalEntry>>> {
        (0..shards)
            .map(|_| {
                ParsingActor {
                    patterns: HashMap::new(),
                    schema: HashMap::new(),
                    log_writer: wal_actor.clone(),
                }
                .start()
            })
            .collect()
    }

    pub fn wal_actor(iceberg: Addr<IcebergWriter>) -> Addr<WalEntry> {
        WalEntry::new(iceberg).start()
    }

    pub fn iceberg_actor() -> Addr<IcebergWriter> {
        IcebergWriter::default().start()
    }

    pub async fn db_actor(config: &Settings, repos: Arc<dyn RepositoryProvider>) -> Addr<DbActor> {
        DbActor::new(config.database.clone(), repos).await.start()
    }

    pub async fn flight_registry_actor(_config: &Settings) -> Addr<FlightRegistry> {
        FlightRegistry::new().await.start()
    }
}

// --- Generic Registry Trait --- //
pub trait Registry {
    type Db: Actor
        + Sync
        + Send
        + Handler<ReposReady>
        + Handler<GetPatternsForTenant>
        + Handler<SaveSchema>;
    type Parser: Actor + Sync + Send + Handler<RegexRequest> + Handler<RecordBatchWrapper>;
    type Wal: Actor + Sync + Send + Handler<RecordBatchWrapper>;
    type IcebergActor: Actor + Sync + Send + Handler<FlushInstruction>;
    type Broadcaster: Actor + Sync + Send + Handler<RecordBatchWrapper>;
    type FlightRegistry: Actor
        + Sync
        + Send
        + Handler<CheckFlight>
        + Handler<RegisterFlight>
        + Handler<ListFlights>;

    fn get_db(&self) -> Addr<Self::Db>;
    fn get_parser(&self) -> Vec<Addr<Self::Parser>>;
    fn get_wal_actor(&self) -> Addr<Self::Wal>;
    fn get_iceberg_actor(&self) -> Addr<Self::IcebergActor>;
    fn get_broadcaster_actor(&self) -> Addr<Self::Broadcaster>;
    fn get_flight_registry_actor(&self) -> Addr<Self::FlightRegistry>;
}

#[derive(Clone)]
pub struct ProdInjestRegistry {
    pub db: Addr<DbActor>,
    pub parser: Vec<Addr<ParsingActor<WalEntry>>>,
    pub wal_actor: Addr<WalEntry>,
    pub iceberg_actor: Addr<IcebergWriter>,
    pub broadcaster: Addr<Broadcaster>,
    pub flight_registry: Addr<FlightRegistry>,
}

impl Registry for ProdInjestRegistry {
    type Db = DbActor;
    type Parser = ParsingActor<WalEntry>;
    type Wal = WalEntry;
    type IcebergActor = IcebergWriter;
    type Broadcaster = Broadcaster;
    type FlightRegistry = FlightRegistry;

    fn get_db(&self) -> Addr<DbActor> {
        self.db.clone()
    }
    fn get_parser(&self) -> Vec<Addr<Self::Parser>> {
        self.parser.clone()
    }
    fn get_wal_actor(&self) -> Addr<Self::Wal> {
        self.wal_actor.clone()
    }
    fn get_iceberg_actor(&self) -> Addr<Self::IcebergActor> {
        self.iceberg_actor.clone()
    }
    fn get_broadcaster_actor(&self) -> Addr<Broadcaster> {
        self.broadcaster.clone()
    }
    fn get_flight_registry_actor(&self) -> Addr<Self::FlightRegistry> {
        self.flight_registry.clone()
    }
}
