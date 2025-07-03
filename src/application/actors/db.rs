// Changed from SyncContext

use crate::config::database::DatabaseSettings;
use crate::core::db::factory::database_factory::DatabasePool;
use actix::{Actor, AsyncContext, Context, Handler, Message, spawn};
use arrow::ipc::writer::IpcWriteOptions;
use arrow_flight::{SchemaAsIpc, SchemaResult};
use std::sync::Arc;
use tonic::Status;

pub struct DbActor {
    database_settings: DatabaseSettings,
    pool: Box<dyn DatabasePool>,
}

impl DbActor {
    pub async fn new(database_settings: DatabaseSettings, pool: Box<dyn DatabasePool>) -> Self {
        Self {
            database_settings,
            pool,
        }
    }
}

impl Actor for DbActor {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        // let settings_for_spawn = self.database_settings.clone();
        let address = ctx.address();
        let pool = self.pool.clone(); // ✅ clone the field, not self
        spawn(async move {
            // let pool = settings_for_spawn.connection_pool().await;
            let _ = address.send(PoolReady { pool });
        });
    }
}

pub struct PoolReady {
    pub pool: Box<dyn DatabasePool + Send + Sync>,
}

impl Message for PoolReady {
    type Result = ();
}

impl Handler<PoolReady> for DbActor {
    type Result = ();

    fn handle(&mut self, msg: PoolReady, _ctx: &mut Self::Context) -> Self::Result {
        self.pool = msg.pool;
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
        let flight_name = arrow_schema.flight_name;
        let schema = arrow_schema.schema;
        let ipc_options = IpcWriteOptions::default();
        let schema_ipc = SchemaAsIpc::new(schema.as_ref(), &ipc_options);
        let schema_result = SchemaResult::try_from(schema_ipc)
            .map_err(|e| Status::internal(format!("Failed to convert Schema: {}", e)))
            .unwrap();
        let bytes = schema_result.schema;

        let pool_clone = self.pool.clone(); // ✅ this works now
        tokio::spawn(async move {
            match pool_clone.execute_query("INSERT INTO SCHEMA (flight_name, schema, created_at, updated_at) VALUES ($1, $2, $3, $4)").await
            {
                Ok(_) => println!("Schema saved successfully"),
                Err(e) => eprintln!("Failed to save schema:"),
            }
        });
    }
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct GetSchema;
