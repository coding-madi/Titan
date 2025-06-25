use crate::actors::parser_actor::{ParsingActor, Pattern};
use crate::actors::wal_actor::WalEntry;
use actix::Handler;
use actix::{Actor, Addr, Context, Message};
use arrow_array::RecordBatch;
use arrow_schema::Schema;
use std::fmt::Display;
use tracing::trace;

pub struct Broadcaster {
    pub shards: i8,
    pub parser_registry: Vec<Addr<ParsingActor>>,
    next_shard_idx: usize, // Index to keep track of the next shard to send messages to
}

impl Broadcaster {
    pub fn new(shard_count: i8) -> Self {
        let mut parser_registry = Vec::with_capacity(shard_count as usize);
        let wal = WalEntry::new(); // Placeholder for the log writer, should be set to a real actor
        let wal_address = Arc::new(wal.start()); // Start the WAL actor and get its address
        for _ in 0..shard_count {
            let mut map = HashMap::new();
            let _ = map.insert(
                String::from("/benchmark/batch_0"),
                Pattern::Regex(".*".to_string()),
            );
            let actor = ParsingActor::start(ParsingActor {
                patterns: map,                   // Initialize with an empty regex,
                schema: HashMap::new(),          // Initialize with an empty schema
                log_writer: wal_address.clone(), // Placeholder for the log writer, should be set to
            });
            parser_registry.push(actor);
        }

        Broadcaster {
            shards: shard_count, // Default number of shards
            parser_registry,     // Initialize with an empty vector
            next_shard_idx: 0,   // Start with the first shard
        }
    }

    pub fn default() -> Self {
        Broadcaster::new(2) // Default to 1 shard
    }
}

impl Actor for Broadcaster {
    type Context = Context<Self>;
}

#[derive(Debug, Clone)]
pub struct RegexRule {
    pub pattern: Pattern,
    pub key: String,
    pub field: String,
}

impl Message for RegexRule {
    type Result = Result<(), String>;
}

// We need to initialize the actor and pass the handles to the Query server.
// The query server will then use these handles to send messages to the regex actors.
// The regex actors will then process the messages and return results to the query server.
impl Handler<RegexRule> for Broadcaster {
    type Result = Result<(), String>;

    fn handle(&mut self, msg: RegexRule, _ctx: &mut Self::Context) -> Self::Result {
        println!("Received RegexRule: {:?}", msg);
        Ok(())
    }
}

use std::collections::HashMap;
use std::sync::Arc;
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
            "Metadata(key: {}, schema: {:?})",
            self.flight, self.schema
        )
    }
}

impl<'a> Message for RecordBatchWrapper {
    type Result = ();
}

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
