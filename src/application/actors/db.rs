// Changed from SyncContext

use crate::config::database::DatabaseSettings;
use crate::core::db::factory::database_factory::RepositoryProvider;
use crate::core::db::repository::Schema;
use actix::{Actor, AsyncContext, Context, Handler, Message, spawn};
use std::sync::Arc;
use tracing::log::info;

pub struct DbActor {
    repos: Arc<dyn RepositoryProvider>,
}

impl DbActor {
    pub async fn new(
        _database_settings: DatabaseSettings,
        repos: Arc<dyn RepositoryProvider>,
    ) -> Self {
        Self { repos }
    }
}

impl Actor for DbActor {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        // let settings_for_spawn = self.database_settings.clone();
        let address = ctx.address();
        let repos = self.repos.clone(); // ✅ clone the field, not self
        spawn(async move {
            // let pool = settings_for_spawn.connection_pool().await;
            let _ = address.send(ReposReady { repos });
        });
    }
}

pub struct ReposReady {
    pub repos: Arc<dyn RepositoryProvider + Send + Sync>,
}

impl Message for ReposReady {
    type Result = ();
}

impl Handler<ReposReady> for DbActor {
    type Result = ();

    fn handle(&mut self, msg: ReposReady, _ctx: &mut Self::Context) -> Self::Result {
        self.repos = msg.repos;
    }
}

pub struct GetPatternsForTenant {
    _tenant: String,
}

impl Message for GetPatternsForTenant {
    type Result = Vec<String>;
}

impl Handler<GetPatternsForTenant> for DbActor {
    type Result = Vec<String>;

    fn handle(&mut self, _msg: GetPatternsForTenant, _ctx: &mut Self::Context) -> Self::Result {
        vec![]
    }
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct SaveSchema {
    pub flight_name: String,
    pub schema: Arc<arrow::datatypes::Schema>,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
}

impl Handler<SaveSchema> for DbActor {
    type Result = ();

    fn handle(&mut self, arrow_schema: SaveSchema, _ctx: &mut Self::Context) -> Self::Result {
        info!("Inside the database actor:");
        let flight_name = arrow_schema.flight_name;
        let schema = arrow_schema.schema;
        let repos = self.repos.clone(); // ✅ this works now
        tokio::spawn(async move {
            let schema = Schema {
                flight_name,
                schema,
                created_at: Default::default(),
                updated_at: Default::default(),
            };
            repos
                .schema_repository()
                .insert_schema(schema)
                .await
                .expect("TODO: panic message");
        });
    }
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct GetSchema;
