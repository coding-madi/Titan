use actix::dev::ToEnvelope;
use actix::{Actor, Addr, Context, Handler};
use arrow::datatypes::Schema;
use arrow_array::{Array, BooleanArray, StringArray};
use std::collections::HashMap;
use tracing::log::info;
use validator::ValidationErrors;

pub(crate) use crate::api::http::regex::{Pattern, RegexRequest};
use crate::application::actors::broadcast::RecordBatchWrapper;

pub struct ParsingActor<WL: Actor + Sync + Send + Handler<RecordBatchWrapper>> {
    pub patterns: HashMap<String, Vec<Pattern>>, // flight_id → patterns
    pub schema: HashMap<String, Schema>,         // service_id → schema
    pub log_writer: Addr<WL>,
}

impl<WL> ParsingActor<WL>
where
    WL: Actor + Sync + Send + Handler<RecordBatchWrapper>,
    WL::Context: ToEnvelope<WL, RecordBatchWrapper>,
{
    pub fn default(log_writer: Addr<WL>) -> Self {
        Self {
            patterns: HashMap::new(),
            schema: HashMap::new(),
            log_writer,
        }
    }

    pub fn new(team_id: String, log_writer: Addr<WL>) -> Self {
        let patterns = get_patterns_from_database(&team_id);
        let schema = get_flight_and_schemas(&team_id);

        Self {
            patterns,
            schema,
            log_writer,
        }
    }
}

impl<WL: Actor + Sync + Send + Handler<RecordBatchWrapper>> Actor for ParsingActor<WL> {
    type Context = Context<Self>;

    fn started(&mut self, _ctx: &mut Self::Context) {
        info!("ParsingActor started")
    }
}

// Handle regex rule registration
impl<WL: Actor + Sync + Send + Handler<RecordBatchWrapper>> Handler<RegexRequest>
    for ParsingActor<WL>
{
    type Result = Result<(), ValidationErrors>;

    fn handle(&mut self, msg: RegexRequest, _ctx: &mut Self::Context) -> Self::Result {
        println!("Received RegexRule in parser: {:?}", msg);
        self.patterns.insert(msg.flight_id, msg.pattern);
        Ok(())
    }
}

use arrow_array::builder::BooleanBuilder;

// Handle incoming data for parsing
impl<WL> Handler<RecordBatchWrapper> for ParsingActor<WL>
where
    WL: Actor + Sync + Send + Handler<RecordBatchWrapper>,
    WL::Context: ToEnvelope<WL, RecordBatchWrapper>,
{
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
                    let column_name = "event_type";
                    let column_index = record
                        .data
                        .schema()
                        .index_of(column_name)
                        .map_err(|e| format!("Column not found '{}': {:?}", column_name, e))
                        .unwrap();

                    let text_array = record
                        .data
                        .column(column_index)
                        .as_any()
                        .downcast_ref::<StringArray>()
                        .ok_or_else(|| format!("Column '{}' is not a StringArray", column_name))
                        .unwrap();
                    let matches = fast_regex_match(text_array, ".*").unwrap();
                    info!("Regex application succeeded");
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
use regex::Regex;

fn fast_regex_match(text_array: &StringArray, pattern: &str) -> Result<BooleanArray, String> {
    let regex = Regex::new(pattern).map_err(|e| format!("Invalid regex: {e}"))?;
    let mut builder = BooleanBuilder::new();

    for i in 0..text_array.len() {
        if text_array.is_null(i) {
            builder.append_null();
        } else {
            builder.append_value(regex.is_match(text_array.value(i)));
        }
    }

    Ok(builder.finish())
}

#[allow(dead_code)]
fn get_patterns_from_database(_team_id: &String) -> HashMap<String, Vec<Pattern>> {
    unimplemented!()
}

#[allow(dead_code)]
fn get_flight_and_schemas(_team_id: &String) -> HashMap<String, Schema> {
    unimplemented!()
}
