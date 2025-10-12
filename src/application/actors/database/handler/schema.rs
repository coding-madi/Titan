use crate::application::actors::database::db_actor::DbActor;
#[cfg(test)]
use crate::application::actors::database::db_actor::MockDbActor;
use crate::core::db::repository::Schema;
use actix::{Handler, Message};
use async_trait::async_trait;
use log::info;
use std::sync::Arc;

/// A handler to save a new schema definition to the database.
///
/// This handler carries all necessary information to persist an `Arrow` schema
/// along with associated metadata.
#[derive(Message)]
#[rtype(result = "()")]
pub struct SaveSchema {
    pub flight_name: String,
    pub schema: Arc<arrow::datatypes::Schema>,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
}

#[async_trait]
impl Handler<SaveSchema> for DbActor {
    type Result = ();

    /// Handles the `SaveSchema` handler.
    ///
    /// This method clones the `RepositoryProvider` and spawns a `tokio` task to
    /// asynchronously insert the schema into the database. This prevents the
    /// actor's main thread from blocking while waiting for database I/O.
    ///
    /// # Arguments
    ///
    /// * `arrow_schema` - The `SaveSchema` handler containing the schema details.
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
                .expect("TODO: panic handler");
        });
    }
}

#[cfg(test)]
impl Handler<SaveSchema> for MockDbActor {
    type Result = ();
    fn handle(&mut self, _: SaveSchema, _: &mut Self::Context) -> Self::Result {
        todo!()
    }
}
