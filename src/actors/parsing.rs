use std::{collections::HashMap, sync::Arc};

use actix::{Actor, Addr, Context, Handler};
use arrow::datatypes::Schema;

use crate::actors::{
    broadcast::{RecordBatchWrapper, RegexRule},
    wal::WalEntry,
};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Key {
    Service(String),
    ServiceWithLog { service: String, log_name: String },
}

pub struct ParsingActor {
    pub regex: HashMap<Key, String>, // key, Pattern
    pub schema: HashMap<String, Schema>,
    pub log_writer: Arc<Addr<WalEntry>>,
}

impl ParsingActor {
    fn default(log_writer: Arc<Addr<WalEntry>>) -> Self {
        Self {
            regex: HashMap::new(),
            schema: HashMap::new(),
            log_writer: log_writer,
        }
    }

    fn new(team_id: String, log_writer: Arc<Addr<WalEntry>>) -> Self {
        // Read patterns from database
        let patterns = get_patterns_from_database(&team_id);
        let schemas = get_flight_and_schemas(&team_id);

        Self {
            regex: patterns,
            schema: schemas,
            log_writer: log_writer,
        }
    }
}

impl Actor for ParsingActor {
    type Context = Context<Self>;
}

impl Handler<RegexRule> for ParsingActor {
    type Result = Result<(), String>;

    fn handle(&mut self, msg: RegexRule, _ctx: &mut Self::Context) -> Self::Result {
        // Here we would typically compile the regex and store it in the actor's state
        // For now, we just print the message
        println!("Received RegexRule: {:?}", msg);
        // Simulate some processing with the regex
        self.regex
            .entry(msg.key)
            .or_default()
            .push_str(msg.pattern.as_str());
        Ok(())
    }
}

impl Handler<RecordBatchWrapper> for ParsingActor {
    type Result = ();

    fn handle(&mut self, _msg: RecordBatchWrapper, _ctx: &mut Self::Context) -> Self::Result {
        // Here we would typically apply the regex to the data in the RecordBatchWrapper
        // For now, we just print the message
        println!("Received RecordBatchWrapper");

        // Simulate processing the record batch with the regex
        // This is where you would apply the regex to the data

        // Send the parsed data to the wal actor
        self.log_writer.do_send(_msg);
    }
}

fn get_patterns_from_database(_team_id: &String) -> HashMap<Key, String> {
    unimplemented!()
}

fn get_flight_and_schemas(_team_id: &String) -> HashMap<String, Schema> {
    unimplemented!()
}
