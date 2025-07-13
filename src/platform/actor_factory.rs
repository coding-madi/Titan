use crate::application::actors::broadcast::Broadcaster;
use crate::application::actors::db::{DbActor, GetPatternsForTenant, ReposReady, SaveSchema};
use crate::application::actors::flight_registry::FlightRegistry;
use crate::application::actors::iceberg::IcebergWriter;
use crate::application::actors::parser::ParsingActor;
use crate::application::actors::wal::WalEntry;
use crate::config::yaml_reader::Settings;
use crate::core::db::factory::database_factory::RepositoryProvider;
use actix::{Actor, Addr, Handler};
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

pub trait Registry {
    type Db: Actor
        + Sync
        + Send
        + Handler<ReposReady>
        + Handler<GetPatternsForTenant>
        + Handler<SaveSchema>;

    fn get_db(&self) -> Addr<Self::Db>;
}

#[derive(Clone)]
struct ProdInjestRegistry {
    pub db: Addr<DbActor>,
}

impl Registry for ProdInjestRegistry {
    type Db = DbActor;

    fn get_db(&self) -> Addr<DbActor> {
        self.db.clone()
    }
}

struct MockDbActor;

impl Actor for MockDbActor {
    type Context = actix::Context<Self>;
}

struct TestInjestRegistry {
    pub db_actor: Addr<MockDbActor>,
}

impl Handler<ReposReady> for MockDbActor {
    type Result = Result<(), ()>;

    fn handle(&mut self, msg: ReposReady, ctx: &mut Self::Context) -> Self::Result {
        println!("Repo ready called with:");
        Ok(())
    }
}

impl Handler<SaveSchema> for MockDbActor {
    type Result = ();

    fn handle(&mut self, msg: SaveSchema, ctx: &mut Self::Context) -> Self::Result {
        todo!()
    }
}

impl Handler<GetPatternsForTenant> for MockDbActor {
    type Result = Vec<String>;

    fn handle(&mut self, msg: GetPatternsForTenant, ctx: &mut Self::Context) -> Self::Result {
        todo!()
    }
}

impl Registry for TestInjestRegistry {
    type Db = MockDbActor;

    fn get_db(&self) -> Addr<MockDbActor> {
        self.db_actor.clone()
    }
}

struct MockFactory;

impl MockFactory {
    pub fn create_db() -> Addr<MockDbActor> {
        MockDbActor {}.start()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::db::factory::database_factory::SqliteRepositoryProvider;
    use crate::core::db::sqlite::SqliteSchemaRepository;
    use sqlx::SqlitePool;

    #[actix_rt::test]
    async fn create_mock() {
        let mock = MockFactory::create_db();
        let registry = TestInjestRegistry {
            db_actor: mock.clone(),
        };

        let on_test_actor = OnTestActor::new(registry.db_actor.clone()).start();
        let repo = ReposReady {
            repos: Arc::new(SqliteRepositoryProvider {
                schema_repo: Arc::new(SqliteSchemaRepository {
                    sqlite_pool: SqlitePool::connect("").await.unwrap(),
                }),
            }),
        };
        let x = on_test_actor.send(repo).await.unwrap();
        assert!(x.is_ok());
        // thread::sleep(std::time::Duration::from_secs(1));
    }
}

struct OnTestActor<
    T: Actor + Sync + Send + Handler<ReposReady> + Handler<GetPatternsForTenant> + Handler<SaveSchema>,
> {
    db_actor: Addr<T>,
}

impl<
    T: Actor + Sync + Send + Handler<ReposReady> + Handler<GetPatternsForTenant> + Handler<SaveSchema>,
> Actor for OnTestActor<T>
{
    type Context = actix::Context<Self>;
}

impl OnTestActor<MockDbActor> {
    fn new(db_actor: Addr<MockDbActor>) -> Self {
        Self { db_actor }
    }
}

impl Handler<ReposReady> for OnTestActor<MockDbActor> {
    type Result = Result<(), ()>;

    fn handle(&mut self, msg: ReposReady, ctx: &mut Self::Context) -> Self::Result {
        self.db_actor.do_send(msg);
        Ok(())
    }
}
