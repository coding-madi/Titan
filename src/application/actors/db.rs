// Changed from SyncContext

use crate::config::database::DatabaseSettings;
use crate::core::db::factory::database_factory::RepositoryProvider;
use crate::core::db::repository::Schema;
use actix::{Actor, AsyncContext, Context, Handler, Message, spawn};
use std::sync::Arc;
use tracing::log::info;

/// `DbActor` is an Actix actor responsible for handling all database operations.
/// It holds an `Arc` to a `RepositoryProvider` trait object, allowing it to interact
/// with various database repositories in a decoupled manner.
#[derive(Clone)]
pub struct DbActor {
    repos: Arc<dyn RepositoryProvider>,
}

/// Creates a new `DbActor` instance.
///
/// This constructor initializes the actor with a given `RepositoryProvider`.
/// The `_database_settings` parameter is currently unused but can be leveraged
/// if actor-specific database configuration is needed during instantiation.
///
/// # Arguments
///
/// * `_database_settings` - Database configuration settings (currently unused).
/// * `repos` - An `Arc` to a `RepositoryProvider` trait object, providing access to
///             concrete repository implementations.
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

    /// Called when the actor starts.
    ///
    /// In this method, the `DbActor` sends a `ReposReady` message to itself. This
    /// asynchronous initialization ensures that the `repos` field is properly
    /// set up within the actor's context, even though it's already available
    /// from the `new` constructor. This pattern can be useful for more complex
    /// asynchronous setup operations that need to complete *after* the actor
    /// has officially started and its address is available.
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

/// A message sent to `DbActor` to signal that the `RepositoryProvider` is ready.
/// This is primarily used during the actor's startup phase (`started` method)
/// to ensure the `repos` field is properly established within the actor's context.
#[derive(Message)]
#[rtype(result = "Result<(), ()>")]
pub struct ReposReady {
    pub repos: Arc<dyn RepositoryProvider + Send + Sync>,
}

impl Handler<ReposReady> for DbActor {
    type Result = Result<(), ()>;

    /// Handles the `ReposReady` message.
    ///
    /// This method updates the actor's internal `repos` field with the provided
    /// `RepositoryProvider`. While `repos` is already set during `new`, this handler
    /// could be extended to perform additional readiness checks or logging once
    /// the actor is fully operational.
    fn handle(&mut self, msg: ReposReady, _ctx: &mut Self::Context) -> Self::Result {
        self.repos = msg.repos;
        Ok(())
    }
}

/// A message to request patterns associated with a specific tenant.
///
/// The `_tenant` field holds the identifier for the tenant whose patterns are being requested.
pub struct GetPatternsForTenant {
    _tenant: String,
}


impl Message for GetPatternsForTenant {
    type Result = Vec<String>;
}

impl Handler<GetPatternsForTenant> for DbActor {
    type Result = Vec<String>;

    /// Handles the `GetPatternsForTenant` message.
    ///
    /// Currently, this handler returns an empty `Vec`. In a real-world scenario,
    /// it would interact with a repository (e.g., `tenant_repository()`) to fetch
    /// and return the actual patterns for the given tenant.
    ///
    /// # Arguments
    ///
    /// * `_msg` - The `GetPatternsForTenant` message containing the tenant identifier.
    /// * `_ctx` - The actor's context (unused in this handler).
    fn handle(&mut self, _msg: GetPatternsForTenant, _ctx: &mut Self::Context) -> Self::Result {
        vec![]
    }
}

/// A message to save a new schema definition to the database.
///
/// This message carries all necessary information to persist an `Arrow` schema
/// along with associated metadata.
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

    /// Handles the `SaveSchema` message.
    ///
    /// This method clones the `RepositoryProvider` and spawns a `tokio` task to
    /// asynchronously insert the schema into the database. This prevents the
    /// actor's main thread from blocking while waiting for database I/O.
    ///
    /// # Arguments
    ///
    /// * `arrow_schema` - The `SaveSchema` message containing the schema details.
    /// * `_ctx` - The actor's context (unused in this handler).
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

/// A message to request a schema from the database.
///
/// This message needs to be extended to include parameters (e.g., `flight_name`)
/// to specify which schema to retrieve.
#[derive(Message)]
#[rtype(result = "()")]
pub struct GetSchema;

// TODO: Implement Handler<GetSchema> for DbActor
// This handler would typically query the database for a specific schema
// and return it as its result.
//
/*
impl Handler<GetSchema> for DbActor {
    type Result = Result<Option<Schema>, String>; // Example: return Option<Schema> or an error

    async fn handle(&mut self, _msg: GetSchema, _ctx: &mut Self::Context) -> Self::Result {
        info!("DbActor: Received request to get schema.");
        // let repos = self.repos.clone();
        // let schema = repos.schema_repository().get_schema(...).await?;
        // Ok(Some(schema))
        Err("Not yet implemented".to_string())
    }
}
*/
