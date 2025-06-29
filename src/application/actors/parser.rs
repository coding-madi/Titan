use std::collections::HashMap;

use crate::application::actors::broadcast::{RecordBatchWrapper};
use crate::application::actors::wal::WalEntry;
use actix::{Actor, Addr, Context, Handler};
use arrow::compute::regexp_match;
use arrow::datatypes::Schema;
use arrow_array::{Array, Datum, ListArray, StringArray};
use crate::api::http::regex::RegexRequest;

pub struct ParsingActor {
    pub patterns: HashMap<String, Pattern>, // key, Pattern
    pub schema: HashMap<String, Schema>,
    pub log_writer: Addr<WalEntry>,
}

impl ParsingActor {
    #[allow(dead_code)]
    pub fn default(log_writer: Addr<WalEntry>) -> Self {
        Self {
            patterns: HashMap::new(),
            schema: HashMap::new(),
            log_writer,
        }
    }

    #[allow(dead_code)]
    pub fn new(team_id: String, log_writer: Addr<WalEntry>) -> Self {
        // Read patterns from database
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

    fn started(&mut self, _ctx: &mut Self::Context)
    where
        Self: Actor<Context = Context<Self>>,
    {
    }
}

impl Handler<RegexRequest> for ParsingActor {
    type Result = Result<(), String>;

    fn handle(&mut self, msg: RegexRequest, _ctx: &mut Self::Context) -> Self::Result {
        // Here we would typically compile the regex and store it in the actor's state
        // For now, we just print the message
        println!("Received RegexRule: {:?}", msg);
        // Simulate some processing with the regex

        // let value = self.patterns.get(&msg.key);
        // match value {
        //     Some(_pattern) => {
        //         self.patterns.insert(msg.key.clone(), msg.pattern);
        //     }
        //     None => {
        //         self.patterns.insert(msg.key.clone(), msg.pattern);
        //     }
        // }
        Ok(())
    }
}

impl<'a> Handler<RecordBatchWrapper> for ParsingActor {
    type Result = ();

    fn handle(&mut self, record: RecordBatchWrapper, _ctx: &mut Self::Context) -> Self::Result {
        let service_id = &record.metadata.service_id;
        // Find if matching regex exists
        let extract_pattern = self.patterns.get(service_id);
        match extract_pattern {
            Some(pattern) => match pattern {
                // Apply regex at service level
                Pattern::Regex(match_pattern) => {
                    let column_name = "event_type";
                    let column_index = record.data.schema().index_of(&column_name).unwrap();
                    let text_array = record
                        .data
                        .column(column_index)
                        .as_any()
                        .downcast_ref::<StringArray>()
                        .ok_or_else(|| {
                            arrow::error::ArrowError::ParseError(format!(
                                "Column '{}' is not a StringArray",
                                column_name
                            ))
                        });
                    let text_length = text_array.unwrap();
                    let extracted_scalar = vec![match_pattern.clone(); text_length.len()];
                    let pattern_array = StringArray::from(extracted_scalar);

                    let matches =
                        regexp_match(text_length, &pattern_array as &dyn Datum, None).unwrap();

                    println!("Matches for {}", match_pattern);

                    let list_array = matches.as_any().downcast_ref::<ListArray>().unwrap();

                    for i in 0..list_array.len() {
                        let group_values = list_array.value(i);
                        let str_array =
                            group_values.as_any().downcast_ref::<StringArray>().unwrap();
                        let _groups: Vec<_> =
                            (0..str_array.len()).map(|i| str_array.value(i)).collect();
                        // println!("Groups {:?}", groups);
                    }
                }
                Pattern::LogRegex(_log_level_pattern) => {
                    let column = "message";
                    let _column_index = record.data.schema().index_of(column).unwrap();
                    // Apply regex per log_name
                }
                Pattern::NoPattern => {
                    // do nothing
                }
            },
            None => {
                // No matching pattern, choose popular groks
            }
        }
        self.log_writer.do_send(record)
    }
}

#[derive(Debug, Clone)]
pub enum Pattern {
    Regex(String),
    LogRegex(HashMap<String, String>),
    NoPattern,
}

#[allow(dead_code)]
fn get_patterns_from_database(_team_id: &String) -> HashMap<String, Pattern> {
    unimplemented!()
}

#[allow(dead_code)]
fn get_flight_and_schemas(_team_id: &String) -> HashMap<String, Schema> {
    unimplemented!()
}
