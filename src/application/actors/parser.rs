use std::collections::HashMap;

use actix::{Actor, Addr, Context, Handler};
use arrow::compute::regexp_match;
use arrow::datatypes::Schema;
use arrow_array::{Array, Datum, ListArray, StringArray};
use tracing::log::info;
use validator::ValidationErrors;

pub(crate) use crate::api::http::regex::{Pattern, RegexRequest};
use crate::application::actors::broadcast::RecordBatchWrapper;
use crate::application::actors::wal::WalEntry;

pub struct ParsingActor {
    pub patterns: HashMap<String, Vec<Pattern>>, // service_id → patterns
    pub schema: HashMap<String, Schema>,         // service_id → schema
    pub log_writer: Addr<WalEntry>,
}

impl ParsingActor {
    pub fn default(log_writer: Addr<WalEntry>) -> Self {
        Self {
            patterns: HashMap::new(),
            schema: HashMap::new(),
            log_writer,
        }
    }

    pub fn new(team_id: String, log_writer: Addr<WalEntry>) -> Self {
        let patterns = get_patterns_from_database(&team_id);
        let schema = get_flight_and_schemas(&team_id);

        Self {
            patterns,
            schema,
            log_writer,
        }
    }
}

impl Actor for ParsingActor {
    type Context = Context<Self>;

    fn started(&mut self, _ctx: &mut Self::Context) {
        info!("ParsingActor started")
    }
}

// Handle regex rule registration
impl Handler<RegexRequest> for ParsingActor {
    type Result = Result<(), ValidationErrors>;

    fn handle(&mut self, msg: RegexRequest, _ctx: &mut Self::Context) -> Self::Result {
        println!("Received RegexRule in parser: {:?}", msg);
        self.patterns.insert(msg.flight_id, msg.pattern);
        Ok(())
    }
}

// Handle incoming data for parsing
impl Handler<RecordBatchWrapper> for ParsingActor {
    type Result = ();

    fn handle(&mut self, record: RecordBatchWrapper, _ctx: &mut Self::Context) -> Self::Result {
        let service_id = &record.metadata.service_id;

        let Some(patterns) = self.patterns.get(service_id) else {
            // No patterns found, forward as-is
            self.log_writer.do_send(record);
            return;
        };

        for pattern in patterns {
            match pattern {
                Pattern::RegexPattern(regex_pattern) => {
                    if let Err(e) = apply_regex_pattern(&record, regex_pattern) {
                        eprintln!("Regex application failed: {:?}", e);
                    }
                }
                Pattern::GrokPattern(_grok) => {
                    // TODO: Implement Grok parsing if needed
                }
            }
        }

        self.log_writer.do_send(record);
    }
}

// Apply a single RegexPattern to the "event_type" column
fn apply_regex_pattern(
    record: &RecordBatchWrapper,
    pattern: &crate::api::http::regex::RegexPattern,
) -> Result<(), String> {
    let column_name = "event_type";

    let column_index = record
        .data
        .schema()
        .index_of(column_name)
        .map_err(|e| format!("Column not found '{}': {:?}", column_name, e))?;

    let text_array = record
        .data
        .column(column_index)
        .as_any()
        .downcast_ref::<StringArray>()
        .ok_or_else(|| format!("Column '{}' is not a StringArray", column_name))?;

    // Create a pattern array of the same length
    let pattern_array = StringArray::from(vec![pattern.pattern_string.clone(); text_array.len()]);

    let matches = regexp_match(text_array, &pattern_array as &dyn Datum, None)
        .map_err(|e| format!("regexp_match error: {:?}", e))?;

    let list_array = matches
        .as_any()
        .downcast_ref::<ListArray>()
        .ok_or("Match result is not a ListArray")?;

    for i in 0..list_array.len() {
        let group_values = list_array.value(i);
        let str_array = group_values
            .as_any()
            .downcast_ref::<StringArray>()
            .ok_or("Group value is not a StringArray")?;

        let groups: Vec<&str> = (0..str_array.len()).map(|i| str_array.value(i)).collect();
        println!("Regex groups: {groups:?}");
    }

    Ok(())
}

#[allow(dead_code)]
fn get_patterns_from_database(_team_id: &String) -> HashMap<String, Vec<Pattern>> {
    unimplemented!()
}

#[allow(dead_code)]
fn get_flight_and_schemas(_team_id: &String) -> HashMap<String, Schema> {
    unimplemented!()
}
