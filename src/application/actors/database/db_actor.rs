use crate::application::actors::database::handler::pattern::GetPatternsForTenant;
pub use crate::application::actors::database::handler::repo_ready::ReposReady;
use crate::application::actors::database::handler::schema::SaveSchema;
use crate::config::database_conf::DatabaseConf;
use crate::core::db::factory::database_factory::RepositoryProvider;
use crate::platform::registry::Registry;
use actix::{Actor, Addr, AsyncContext, Context, Message, spawn};
use async_trait::async_trait;
use log::trace;
use std::sync::Arc;

#[derive(Clone, Message, Debug)]
#[rtype(result = "()")]
pub enum DbActorAddr {
    Real(Addr<DbActor>),
    #[cfg(test)]
    Mock(Addr<MockDbActor>),
    Empty,
}

impl DbActorAddr {
    pub async fn send_repo_ready(
        &self,
        repos_ready: ReposReady,
    ) -> Result<(), actix::MailboxError> {
        match self {
            DbActorAddr::Real(addr) => addr.send(repos_ready).await,
            #[cfg(test)]
            DbActorAddr::Mock(addr) => addr.send(repos_ready).await,
            _ => Ok(()),
        }
    }

    pub async fn send_get_patterns_for_tenant(
        &self,
        get_patterns_for_tenant: GetPatternsForTenant,
    ) -> Result<Vec<String>, actix::MailboxError> {
        match self {
            DbActorAddr::Real(addr) => addr.send(get_patterns_for_tenant).await,
            #[cfg(test)]
            DbActorAddr::Mock(addr) => addr.send(get_patterns_for_tenant).await,
            _ => Ok(vec![]),
        }
    }

    pub async fn send_save_schema(
        &self,
        save_schema: SaveSchema,
    ) -> Result<(), actix::MailboxError> {
        match self {
            DbActorAddr::Real(addr) => addr.send(save_schema).await,
            #[cfg(test)]
            DbActorAddr::Mock(addr) => addr.send(save_schema).await,
            _ => Ok(()),
        }
    }
}

/// `DbActor` is an Actix actor responsible for handling all database operations.
/// It holds an `Arc` to a `RepositoryProvider` trait object, allowing it to interact
/// with various database repositories in a decoupled manner.
#[derive(Clone)]
pub struct DbActor {
    pub(crate) repos: Arc<dyn RepositoryProvider>,
    pub registry: Addr<Registry>,
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
impl DbActor {
    pub async fn new(
        _database_settings: DatabaseConf,
        repos: Arc<dyn RepositoryProvider>,
        registry: Addr<Registry>,
    ) -> Self {
        Self { repos, registry }
    }
}

impl Actor for DbActor {
    type Context = Context<Self>;

    /// Called when the actor starts.
    ///
    /// In this method, the `DbActor` sends a `ReposReady` handler to itself. This
    /// asynchronous initialization ensures that the `repos` field is properly
    /// set up within the actor's context, even though it's already available
    /// from the `new` constructor. This pattern can be useful for more complex
    /// asynchronous setup operations that need to complete *after* the actor
    /// has officially started and its address is available.
    fn started(&mut self, ctx: &mut Self::Context) {
        // let settings_for_spawn = self.database_settings.clone();
        let address = ctx.address();
        let registry_address = self.registry.clone();
        let repos = self.repos.clone(); // ✅ clone the field, not self
        spawn(async move {
            // let pool = settings_for_spawn.connection_pool().await;
            let _ = address.send(ReposReady { repos }).await;
            registry_address.do_send(DbActorAddr::Real(address));
        });
        trace!("DbActor started.");
    }
}

/// A handler to request a schema from the database.
///
/// This handler needs to be extended to include parameters (e.g., `flight_name`)
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

#[cfg(test)]
pub struct MockDbActor {
    pub registry_address: Addr<Registry>,
}

#[cfg(test)]
impl Actor for MockDbActor {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        self.registry_address
            .do_send(DbActorAddr::Mock(ctx.address()));
    }
}
