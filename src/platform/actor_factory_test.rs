// use crate::platform::actor_factory_test::mocks::{MockDbActor, MockParsingActor, MockWalActor};

// --- Test Infrastructure --- //
#[cfg(test)]
pub mod mocks {
    use crate::api::http::regex::RegexRequest;
    use crate::application::actors::broadcast::RecordBatchWrapper;
    use crate::application::actors::db::{GetPatternsForTenant, ReposReady, SaveSchema};
    use crate::application::actors::flight_registry::{CheckFlight, ListFlights, RegisterFlight};
    use crate::application::actors::iceberg::FlushInstruction;
    use crate::platform::actor_factory::Registry;
    use actix::{Actor, Addr, Context, Handler};
    use std::collections::HashSet;
    use std::io::Error;
    use validator::ValidationErrors;

    pub struct MockDbActor;
    #[cfg(test)]
    pub struct MockWalActor;
    #[cfg(test)]
    pub struct MockParsingActor;

    #[cfg(test)]
    impl Actor for MockDbActor {
        type Context = Context<Self>;
    }
    #[cfg(test)]
    impl Actor for MockWalActor {
        type Context = Context<Self>;
    }
    #[cfg(test)]
    impl Actor for MockParsingActor {
        type Context = Context<Self>;
    }

    #[cfg(test)]
    impl Handler<ReposReady> for MockDbActor {
        type Result = Result<(), ()>;
        fn handle(&mut self, _msg: ReposReady, _: &mut Self::Context) -> Self::Result {
            Ok(())
        }
    }

    #[cfg(test)]
    impl Handler<SaveSchema> for MockDbActor {
        type Result = ();
        fn handle(&mut self, _: SaveSchema, _: &mut Self::Context) -> Self::Result {
            todo!()
        }
    }

    #[cfg(test)]
    impl Handler<GetPatternsForTenant> for MockDbActor {
        type Result = Vec<String>;
        fn handle(&mut self, _: GetPatternsForTenant, _: &mut Self::Context) -> Self::Result {
            todo!()
        }
    }

    #[cfg(test)]
    impl Handler<RecordBatchWrapper> for MockWalActor {
        type Result = ();
        fn handle(&mut self, _: RecordBatchWrapper, _: &mut Self::Context) -> Self::Result {
            todo!()
        }
    }

    #[cfg(test)]
    impl Handler<RecordBatchWrapper> for MockParsingActor {
        type Result = ();
        fn handle(&mut self, _: RecordBatchWrapper, _: &mut Self::Context) -> Self::Result {
            todo!()
        }
    }

    #[cfg(test)]
    impl Handler<RegexRequest> for MockParsingActor {
        type Result = Result<(), ValidationErrors>;
        fn handle(&mut self, _: RegexRequest, _: &mut Self::Context) -> Self::Result {
            todo!()
        }
    }

    #[cfg(test)]
    pub struct MockIcebergActor;

    #[cfg(test)]
    impl Actor for MockIcebergActor {
        type Context = Context<Self>;
    }

    #[cfg(test)]
    impl Handler<FlushInstruction> for MockIcebergActor {
        type Result = Result<(), String>;

        fn handle(&mut self, _msg: FlushInstruction, _ctx: &mut Self::Context) -> Self::Result {
            todo!()
        }
    }

    // --- Test Registry --- //
    #[cfg(test)]
    pub struct TestInjestRegistry {
        pub db_actor: Addr<MockDbActor>,
        pub parser_actor: Vec<Addr<MockParsingActor>>,
        pub wal_actor: Addr<MockWalActor>,
        pub iceberg_actor: Addr<MockIcebergActor>,
        pub broadcaster: Addr<MockBroadcastActor>,
        pub flight_registry: Addr<MockFlightRegistry>,
    }

    #[cfg(test)]
    pub struct MockBroadcastActor;

    #[cfg(test)]
    impl Actor for MockBroadcastActor {
        type Context = Context<Self>;
    }

    #[cfg(test)]
    impl Handler<RegexRequest> for MockBroadcastActor {
        type Result = Result<(), ValidationErrors>;

        fn handle(&mut self, _msg: RegexRequest, _ctx: &mut Self::Context) -> Self::Result {
            todo!()
        }
    }

    #[cfg(test)]
    impl Handler<RecordBatchWrapper> for MockBroadcastActor {
        type Result = ();

        fn handle(&mut self, _msg: RecordBatchWrapper, _ctx: &mut Self::Context) -> Self::Result {
            todo!()
        }
    }

    #[cfg(test)]
    pub struct MockFlightRegistry;

    #[cfg(test)]
    impl Actor for MockFlightRegistry {
        type Context = Context<Self>;
    }

    #[cfg(test)]
    impl Handler<CheckFlight> for MockFlightRegistry {
        type Result = Result<bool, Error>;

        fn handle(&mut self, _msg: CheckFlight, _ctx: &mut Self::Context) -> Self::Result {
            todo!()
        }
    }

    #[cfg(test)]
    impl Handler<RegisterFlight> for MockFlightRegistry {
        type Result = ();

        fn handle(&mut self, _msg: RegisterFlight, _ctx: &mut Self::Context) -> Self::Result {
            todo!()
        }
    }

    #[cfg(test)]
    impl Handler<ListFlights> for MockFlightRegistry {
        type Result = Result<HashSet<String>, Error>;

        fn handle(&mut self, _msg: ListFlights, _ctx: &mut Self::Context) -> Self::Result {
            todo!()
        }
    }

    #[cfg(test)]
    impl Registry for TestInjestRegistry {
        type Db = MockDbActor;
        type Parser = MockParsingActor;
        type Wal = MockWalActor;
        type IcebergActor = MockIcebergActor;
        type Broadcaster = MockBroadcastActor;
        type FlightRegistry = MockFlightRegistry;

        fn get_db(&self) -> Addr<Self::Db> {
            self.db_actor.clone()
        }
        fn get_parser(&self) -> Vec<Addr<Self::Parser>> {
            self.parser_actor.clone()
        }
        fn get_wal_actor(&self) -> Addr<Self::Wal> {
            self.wal_actor.clone()
        }

        fn get_iceberg_actor(&self) -> Addr<Self::IcebergActor> {
            self.iceberg_actor.clone()
        }

        fn get_broadcaster_actor(&self) -> Addr<Self::Broadcaster> {
            self.broadcaster.clone()
        }

        fn get_flight_registry_actor(&self) -> Addr<Self::FlightRegistry> {
            self.flight_registry.clone()
        }
    }
}

#[cfg(test)]
pub mod tests {
    use std::sync::Arc;
    use actix::{Actor, Addr, Context, Handler};

    use actix_rt::test as actix_test;
    use sqlx::SqlitePool;
    use crate::api::http::regex::RegexRequest;
    use crate::application::actors::db::{GetPatternsForTenant, ReposReady, SaveSchema};
    use crate::core::db::factory::database_factory::SqliteRepositoryProvider;
    use crate::core::db::sqlite::SqliteSchemaRepository;
    use crate::platform::actor_factory_test::mocks::*;

    pub struct MockFactory;

    impl MockFactory {
        pub fn create_db() -> Addr<MockDbActor> {
            MockDbActor {}.start()
        }
        pub fn create_parser() -> Vec<Addr<MockParsingActor>> {
            vec![MockParsingActor {}.start()]
        }
        pub fn create_wal() -> Addr<MockWalActor> {
            MockWalActor {}.start()
        }
    }

    // --- OnTestActor --- //
    struct OnTestActor<DB>
    where
        DB: Actor
        + Sync
        + Send
        + Handler<ReposReady>
        + Handler<GetPatternsForTenant>
        + Handler<SaveSchema>,
    {
        db_actor: Addr<DB>,
    }

    impl<DB> Actor for OnTestActor<DB>
    where
        DB: Actor
        + Sync
        + Send
        + Handler<ReposReady>
        + Handler<GetPatternsForTenant>
        + Handler<SaveSchema>,
    {
        type Context = Context<Self>;
    }

    impl OnTestActor<MockDbActor> {
        fn new(db_actor: Addr<MockDbActor>) -> Self {
            Self { db_actor }
        }
    }

    impl Handler<ReposReady> for OnTestActor<MockDbActor> {
        type Result = Result<(), ()>;
        fn handle(&mut self, msg: ReposReady, _: &mut Self::Context) -> Self::Result {
            self.db_actor.do_send(msg);
            Ok(())
        }
    }

    #[actix_test]
    async fn test_create_mock_db() {
        let _db = MockFactory::create_db();
        // No connected() method on Addr; just ensure no panic on creation
    }

    #[actix_test]
    async fn test_parser_receives_regex_request() {
        let parser = MockFactory::create_parser().pop().unwrap();
        let req = RegexRequest {
            name: "test".into(),
            tenant: "t1".into(),
            flight_id: "f1".into(),
            log_group: "lg".into(),
            pattern: vec![],
        };

        let _ = parser.send(req).await;
    }

    #[actix_test]
    async fn test_integration_with_on_test_actor() {
        let db = MockFactory::create_db();

        let repo = ReposReady {
            repos: Arc::new(SqliteRepositoryProvider {
                schema_repo: Arc::new(SqliteSchemaRepository {
                    sqlite_pool: SqlitePool::connect("sqlite::memory:").await.unwrap(),
                }),
            }),
        };

        let actor = OnTestActor::new(db.clone()).start();
        let result = actor.send(repo).await.unwrap();
        assert!(result.is_ok());
    }
}
