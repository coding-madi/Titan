use crate::application::actors::database::db_actor::DbActor;
#[cfg(test)]
use crate::application::actors::database::db_actor::MockDbActor;
use crate::core::db::factory::database_factory::RepositoryProvider;
use actix::{Handler, Message};
use std::sync::Arc;

/// A handler sent to `DbActor` to signal that the `RepositoryProvider` is ready.
/// This is primarily used during the actor's startup phase (`started` method)
/// to ensure the `repos` field is properly established within the actor's context.
#[derive(Message)]
#[rtype(result = "()")]
pub struct ReposReady {
    pub repos: Arc<dyn RepositoryProvider + Send + Sync>,
}

impl Handler<ReposReady> for DbActor {
    type Result = ();

    /// Handles the `ReposReady` handler.
    ///
    /// This method updates the actor's internal `repos` field with the provided
    /// `RepositoryProvider`. While `repos` is already set during `new`, this handler
    /// could be extended to perform additional readiness checks or logging once
    /// the actor is fully operational.
    fn handle(&mut self, msg: ReposReady, _ctx: &mut Self::Context) -> Self::Result {
        self.repos = msg.repos;
    }
}

#[cfg(test)]
impl Handler<ReposReady> for MockDbActor {
    type Result = ();
    fn handle(&mut self, _msg: ReposReady, _: &mut Self::Context) -> Self::Result {}
}
