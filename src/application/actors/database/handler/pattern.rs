use crate::application::actors::database::db_actor::DbActor;
#[cfg(test)]
use crate::application::actors::database::db_actor::MockDbActor;
use actix::{Handler, Message};

/// A handler to request patterns associated with a specific tenant.
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

    /// Handles the `GetPatternsForTenant` handler.
    ///
    /// Currently, this handler returns an empty `Vec`. In a real-world scenario,
    /// it would interact with a repository (e.g., `tenant_repository()`) to fetch
    /// and return the actual patterns for the given tenant.
    ///
    /// # Arguments
    ///
    /// * `_msg` - The `GetPatternsForTenant` handler containing the tenant identifier.
    /// * `_ctx` - The actor's context (unused in this handler).
    fn handle(&mut self, _msg: GetPatternsForTenant, _ctx: &mut Self::Context) -> Self::Result {
        vec![]
    }
}

#[cfg(test)]
impl Handler<GetPatternsForTenant> for MockDbActor {
    type Result = Vec<String>;
    fn handle(&mut self, _: GetPatternsForTenant, _: &mut Self::Context) -> Self::Result {
        todo!()
    }
}
