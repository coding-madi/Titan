// Changed from SyncContext

use crate::config::database::DatabaseSettings;
use actix::{Actor, AsyncContext, Context, Handler, Message, spawn};
use actix_web::body::MessageBody;
use arrow::ipc::writer::IpcWriteOptions;
use arrow_flight::{SchemaAsIpc, SchemaResult};
use sqlx::{Pool, Postgres};
use std::sync::Arc;
use tonic::Status;

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
            let _ = address.send(PoolReady { pool: pool.clone() });
        });
    }
}

pub struct PoolReady {
    pub pool: Pool<Postgres>,
}

impl Message for PoolReady {
    type Result = ();
}

impl Handler<PoolReady> for DbActor {
    type Result = ();

    fn handle(&mut self, msg: PoolReady, _ctx: &mut Self::Context) -> Self::Result {
        self.pool = Some(msg.pool)
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

    fn handle(&mut self, arrow_schema: SaveSchema, ctx: &mut Self::Context) -> Self::Result {
        let flight_name = arrow_schema.flight_name;
        let schema = arrow_schema.schema;
        let ipc_options = IpcWriteOptions::default();
        let schema_ipc = SchemaAsIpc::new(schema.as_ref(), &ipc_options);
        let schema_result = SchemaResult::try_from(schema_ipc)
            .map_err(|e| Status::internal(format!("Failed to convert Schema: {}", e)))
            .unwrap();
        let bytes = schema_result.schema;
        let pool = match self.pool {
            Some(ref pool) => pool.clone(),
            None => panic!("Database actor pool not initialized!"),
        };
        tokio::spawn(async move {
            match sqlx::query("INSERT INTO SCHEMA (flight_name, schema, created_at, updated_at) VALUES ($1, $2, $3, $4)")
                .bind(&flight_name)
                .bind(&bytes.to_vec())
                .bind(chrono::Utc::now())
                .bind(chrono::Utc::now())
                .execute(&pool)
                .await
            {
                Ok(_) => println!("Schema saved successfully"),
                Err(e) => eprintln!("Failed to save schema: {}", e),
            }
        });
    }
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct GetSchema;
