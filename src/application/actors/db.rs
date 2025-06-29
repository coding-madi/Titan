use crate::config::database::DatabaseSettings;
use actix::{Actor, AsyncContext, Context, Handler, Message, spawn};
use sqlx::{Pool, Postgres};
// Changed from SyncContext

pub struct DbActor {
    database_settings: DatabaseSettings,
    pool: Option<Pool<Postgres>>,
}

impl DbActor {
    pub async fn new(database_settings: DatabaseSettings) -> Self {
        Self {
            database_settings,
            pool: None,
        }
    }

    // A helper method to get the pool, assuming it's already initialized
    pub fn get_pool(&self) -> &Pool<Postgres> {
        self.pool
            .as_ref()
            .expect("Database actor pool not initialized!")
    }
}

impl Actor for DbActor {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        let address = ctx.address();
        let settings_for_spawn = self.database_settings.clone();
        spawn(async move {
            let pool = settings_for_spawn.connection_pool().await;
            let _ = address.send(PoolReady(pool.clone()));
        });
    }
}

struct PoolReady(Pool<Postgres>);

impl Message for PoolReady {
    type Result = ();
}

impl Handler<PoolReady> for DbActor {
    type Result = ();

    fn handle(&mut self, msg: PoolReady, _ctx: &mut Self::Context) -> Self::Result {
        self.pool = Some(msg.0)
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
