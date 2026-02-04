use actix::Handler;
use actix::{Actor, Addr, Context, Message};
use arrow_array::RecordBatch;
use arrow_schema::Schema;
use std::fmt::Display;
use tracing::trace;

pub struct Broadcaster {
    pub parser_registry: Vec<Addr<ParsingActor>>,
    pub next_shard_idx: usize, // Index to keep track of the next shard to send messages to
}

impl Broadcaster {
    pub fn new(parser_actor: Vec<Addr<ParsingActor>>) -> Broadcaster {
        Self {
            next_shard_idx: 0,
            parser_registry: parser_actor,
        }
    }
}

impl Actor for Broadcaster {
    type Context = Context<Self>;
}

// We need to initialize the actor and pass the handles to the Query server.
// The query server will then use these handles to send messages to the regex actors.
// The regex actors will then process the messages and return results to the query server.
impl Handler<RegexRequest> for Broadcaster {
    type Result = Result<(), ValidationErrors>;

    fn handle(&mut self, regex_request: RegexRequest, _ctx: &mut Self::Context) -> Self::Result {
        for i in 0..self.parser_registry.len() {
            self.parser_registry[i].do_send(regex_request.clone())
        }
        Ok(())
    }
}

use crate::api::http::regex::RegexRequest;
use crate::application::actors::parser::ParsingActor;
use std::sync::Arc;
use validator::ValidationErrors;

impl<'a> Handler<RecordBatchWrapper> for Broadcaster {
    type Result = ();

    fn handle(&mut self, msg: RecordBatchWrapper, _ctx: &mut Self::Context) -> Self::Result {
        if self.parser_registry.is_empty() {
            eprintln!("No regex handlers available to distribute RecordBatchWrapper.");
            return;
        }

        // Get the address of the next shard in a round-robin fashion
        let current_shard_idx = self.next_shard_idx;
        let parser_handle = &self.parser_registry[current_shard_idx];

        // Update the index for the next message
        self.next_shard_idx = (self.next_shard_idx + 1) % self.parser_registry.len();

        // Send the RecordBatch to the selected RegexActor
        // Use `do_send` for fire-and-forget, or `send().await` if you need to wait for a response
        trace!(
            "Dispatched RecordBatchWrapper for key '{}' to shard index {}",
            &msg.metadata, current_shard_idx
        );
        parser_handle.do_send(msg);
    }
}

pub struct RecordBatchWrapper {
    pub metadata: Metadata,
    pub data: Arc<RecordBatch>,
}

#[derive(Debug, Clone)]
pub struct Metadata {
    pub flight: String,
    pub buffer_id: u64,
    pub schema: Arc<Schema>,
    pub service_id: String,
}

impl Display for Metadata {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Metadata(key: {}, Schema: {:?})",
            self.flight, self.schema
        )
    }
}

impl<'a> Message for RecordBatchWrapper {
    type Result = ();
}
