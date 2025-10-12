use crate::application::actors::broadcaster::broadcast_actor::RecordBatchWrapper;
use crate::application::actors::iceberg::iceberg_actor::{IcebergActor, IcebergActorAddr};
use crate::application::actors::parser::handlers::try_parsing_regex::TryParsingRegex;
use crate::application::actors::rhai::rhai_actor::RhaiActor;
use crate::application::actors::wal::log::log_wal_actor::WalActorWrapper;
use crate::core::error::exception::buffer_error::BufferError;
use crate::core::error::exception::regex::RegexError;
use crate::core::parser::messages::parser::Pattern;
use crate::core::parser::parser_contract::ParserContract;
use crate::core::utils::arrow::arrow_buffer_to_json;
use crate::platform::registry::{FetchIcebergActor, FetchWalActor, Registry};
use actix::Addr;
use futures_util::future;
use log::error;
use serde_json::Value;
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;

#[derive(Clone)]
pub struct ParserService {
    #[allow(dead_code, unused_imports)]
    flight_name: String,
    patterns: HashMap<String, Vec<Pattern>>, // log_group → patterns
    registry_address: Addr<Registry>,
    rhai_meter_actor: Option<Addr<RhaiActor>>, // optional for now
    parser_engine: Arc<dyn ParserContract + Send + Sync>,
}

impl ParserService {
    pub fn default(
        flight_name: String,
        registry_address: Addr<Registry>,
        parser_engine: Arc<dyn ParserContract + Send + Sync + 'static>,
        rhai_meter_actor: Option<Addr<RhaiActor>>,
    ) -> Self {
        Self {
            flight_name,
            patterns: HashMap::new(),
            registry_address,
            rhai_meter_actor,
            parser_engine,
        }
    }

    pub fn new(
        flight_name: String,
        registry_address: Addr<Registry>,
        parser_engine: Arc<dyn ParserContract + Send + Sync + 'static>,
        rhai_meter_actor: Option<Addr<RhaiActor>>,
    ) -> Self {
        // let patterns = get_patterns_from_database(&flight_name);
        let patterns = HashMap::new();
        Self {
            flight_name,
            patterns,
            registry_address,
            rhai_meter_actor,
            parser_engine,
        }
    }

    pub fn get_registry_address(&self) -> Addr<Registry> {
        self.registry_address.clone()
    }

    pub fn save_pattern_in_state(&mut self, log_group_name: String, patterns: Vec<Pattern>) {
        self.patterns.insert(log_group_name, patterns);
    }

    pub fn handle_message(&self, message: RecordBatchWrapper) -> Pin<Box<dyn Future<Output = ()>>> {
        let registry_address = self.registry_address.clone();
        let parser_engine = self.parser_engine.clone();
        let flight_id = message.get_flight_name().to_string();
        let patterns = self.patterns.clone();
        let rhai_actor = self.rhai_meter_actor.clone();

        Box::pin(async move {
            // 1. Fetch actors
            let wal_actor = match Self::fetch_wal_actor(&registry_address, &flight_id).await {
                Some(addr) => addr,
                None => return,
            };

            let iceberg_actor = match Self::fetch_iceberg_actor(&registry_address, &flight_id).await
            {
                Some(addr) => addr,
                None => return,
            };

            if let Err(e) = iceberg_actor.send(message.clone()).await {
                error!("Failed to send record to IcebergActor: {:?}", e);
            }
            if let Ok(result) =
                Self::spawn_parse_task(parser_engine, message.clone(), patterns).await
            {
                for record in result {
                    Self::send_to_wal(wal_actor.clone(), record.clone());
                    if let Some(rhai_address) = rhai_actor.clone() {
                        rhai_address.do_send(record);
                    }
                }
            } else {
                error!("Failed to parse record: {:?}", message);
            }
        })
    }

    async fn fetch_wal_actor(
        registry: &Addr<Registry>,
        flight_id: &str,
    ) -> Option<WalActorWrapper> {
        match registry.send(FetchWalActor).await {
            Ok(Ok(addr)) => Some(addr),
            _ => {
                error!("Failed to fetch WalActorAddr for flight {flight_id}");
                None
            }
        }
    }

    async fn fetch_iceberg_actor(
        registry: &Addr<Registry>,
        flight_id: &str,
    ) -> Option<Addr<IcebergActor>> {
        match registry.send(FetchIcebergActor).await {
            Ok(Ok(IcebergActorAddr::Real(actor))) => Some(actor),
            _ => {
                error!("Failed to fetch IcebergActor for flight {flight_id}");
                None
            }
        }
    }

    async fn spawn_parse_task(
        parser_engine: Arc<dyn ParserContract + Send + Sync>,
        message: RecordBatchWrapper,
        patterns: HashMap<String, Vec<Pattern>>,
    ) -> Result<Vec<RecordBatchWrapper>, RegexError> {
        let result = parser_engine.parse(vec![message.clone()], patterns, false);
        result
    }

    fn send_to_wal(wal: WalActorWrapper, message: RecordBatchWrapper) {
        match wal {
            WalActorWrapper::Real(actor) => actor.do_send(message),
            #[cfg(test)]
            WalActorWrapper::Mock(actor) => actor.do_send(message),
            _ => {}
        }
    }

    pub fn try_parse(
        &self,
        msg: TryParsingRegex,
    ) -> Pin<Box<dyn Future<Output = Result<Value, RegexError>>>> {
        let registry = self.registry_address.clone();
        let flight_name = msg.flight_name.clone();
        let parser_engine = self.parser_engine.clone();

        let try_parsing_regex = msg.try_parsing.clone();

        let futures = async move {
            let iceberg_actor = registry
                .send(FetchIcebergActor)
                .await
                .map_err(|_| {
                    std::io::Error::new(std::io::ErrorKind::Other, "Registry mailbox closed")
                })
                .unwrap()
                .unwrap();

            let mut futures = vec![];

            if try_parsing_regex {
                let future = iceberg_actor.get_buffer(flight_name);
                futures.push(future);
            }

            let results: Vec<Result<Vec<RecordBatchWrapper>, BufferError>> =
                future::join_all(futures).await;

            let flattened: Result<Vec<RecordBatchWrapper>, BufferError> = results
                .into_iter()
                .collect::<Result<Vec<_>, _>>() // Result<Vec<Vec<_>>, fmt::Error>
                .map(|nested_vectors| nested_vectors.into_iter().flatten().collect());

            if let Ok(records) = flattened {
                let mut map = HashMap::new();
                map.insert(msg.log_group.clone(), msg.pattern.clone());
                let results = parser_engine.parse(records, map, false);
                let record = results.unwrap();
                let record_batch_wrapper = match record.first() {
                    Some(record_batch_wrapper) => record_batch_wrapper,
                    None => return Ok(Value::Null),
                };
                let json = arrow_buffer_to_json(record_batch_wrapper.get_data().as_ref());

                return Ok(Value::Array(json));
            } else {
                error!("Failed to get records from IcebergActor");
            }

            Ok(Value::Null)
        };
        Box::pin(futures)
    }
}
