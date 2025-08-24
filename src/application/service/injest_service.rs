use std::sync::Arc;
use std::time::Instant;
use actix::Addr;
use actix_web::web::Bytes;
use arrow_flight::{FlightData, PutResult};
use futures_core::Stream;
use futures_util::StreamExt;
use tonic::Status;
use tracing::{error, info};
use crate::application::actors::broadcast::BroadcastActor;
use crate::application::actors::factory_actor::{CreateBroadcastActor, CreateParserActor, FactoryActor, FactoryActorAddr};
use crate::application::actors::factory_actor::FactoryActorAddr::Real;
use crate::config::yaml_reader::Settings;
use crate::core::parser::parser_contract::ParserType;
use crate::core::utils::flight::{handle_record_batch_put_message, initialize_stream};
use crate::platform::registry::Registry;

pub struct InjestService {
    pub actor_registry: Addr<Registry>,
    pub config: Arc<Settings>
}

impl InjestService {
    pub fn new(actor_registry: Addr<Registry>, config: Arc<Settings>) -> Self {
        Self {
            actor_registry,
            config
        }
    }

    pub async fn do_put_from_stream<S>(&self, mut stream: S) -> Result<Vec<PutResult>, Status>
    where
        S: Stream<Item = Result<FlightData, Status>> + Unpin + Send + 'static,
    {
        let stream_start_time = Instant::now();

        // 1. Process initial metadata and schema.
        let (name, schema) = initialize_stream(&mut stream, self.actor_registry.clone()).await?;

        // 2. Initialize the necessary actors.
        let broadcast = initialize_actors(
            self.actor_registry.clone(),
            self.config.clone(),
            &name,
        ).await?;

        // 3. Process the data batches and get the total bytes received.
        let total_data_bytes_received = process_flight_stream(
            &mut stream,
            &name,
            &schema,
            broadcast.clone(),
        ).await?;

        // 4. Create and return the final results.
        let total_server_duration = stream_start_time.elapsed().as_secs_f64();
        let metadata_string = format!(
            "Duration:{:.4},Bytes:{}",
            total_server_duration, total_data_bytes_received
        );
        info!("Server finished processing stream. Metrics: {}", metadata_string);

        let put_result = PutResult {
            app_metadata: Bytes::from(metadata_string.into_bytes()),
            ..Default::default()
        };
        Ok(vec![put_result])
    }

    // pub async fn do_put_from_stream<S>(&self, mut stream: S) -> Result<Vec<PutResult>, Status>
    // where
    //     S: Stream<Item = Result<FlightData, Status>> + Unpin + Send + 'static,
    // {
    //     let stream_start_time = Instant::now();
    //     let mut total_data_bytes_received: usize = 0; // To track only data body bytes
    //
    //     // 1. Process the initial metadata and schema messages.
    //     let (name, schema) = initialize_stream(&mut stream, self.actor_registry.clone()).await?;
    //
    //     let name_copy = name.clone();
    //     let actor_registry_addr = self.actor_registry.clone();
    //
    //     // Get the factor actor that can create actors in the actix context.
    //     let factory = actor_registry_addr
    //         .send(crate::platform::registry::FetchFactoryActor {})
    //         .await
    //         .unwrap()
    //         .unwrap();
    //
    //     let broadcast = match factory {
    //         Real(factory_actor_addr) => {
    //             let parser_type: ParserType = self.config.parser.clone().into();
    //             let parser_actors = factory_actor_addr
    //                 .send(CreateParserActor {
    //                     flight_name: name_copy.clone(),
    //                     count: 2,
    //                     parser_type
    //
    //                 })
    //                 .await
    //                 .unwrap();
    //
    //             let broadcast_actor = factory_actor_addr
    //                 .send(CreateBroadcastActor {
    //                     flight_name: name_copy.clone(),
    //                     parser_actors,
    //                 })
    //                 .await
    //                 .unwrap();
    //             broadcast_actor
    //         }
    //         _ => {
    //             error!("Failed to get factory actor");
    //             return Err(Status::internal("Failed to get factory actor"));
    //         }
    //     };
    //
    //     // 2. Process the remaining data batches.
    //     let mut put_results: Vec<PutResult> = vec![];
    //     while let Some(flight_data_res) = stream.next().await {
    //         let flight_data = flight_data_res?;
    //
    //         // Skip empty data messages that are not the schema message
    //         if flight_data.data_body.is_empty() {
    //             continue;
    //         }
    //
    //         total_data_bytes_received += flight_data.data_body.len();
    //
    //         handle_record_batch_put_message(
    //             &flight_data,
    //             &schema,
    //             &Some(name.clone()),
    //             broadcast.clone(),
    //         )
    //             .await?;
    //     }
    //     let total_server_duration = stream_start_time.elapsed().as_secs_f64();
    //     let metadata_string = format!(
    //         "Duration:{:.4},Bytes:{}",
    //         total_server_duration, total_data_bytes_received
    //     );
    //     info!(
    //         "Server finished processing stream. Metrics: {}",
    //         metadata_string
    //     );
    //     let put_result = PutResult {
    //         app_metadata: Bytes::from(metadata_string.into_bytes()), // Convert String to Bytes
    //         // Other fields in PutResult can be left default or populated as needed.
    //         // For example, if you want to include some summary stats for the data received:
    //         // row_count: -1, // Or actual count if tracked
    //         // schema_uri: "".to_string(),
    //     };
    //     put_results.insert(0, put_result);
    //     Ok(put_results)
    // }
}

async fn process_flight_stream(
    stream: &mut (impl Stream<Item = Result<FlightData, Status>> + Unpin + Send + 'static),
    name: &str,
    schema: &arrow_schema::SchemaRef,
    broadcast: Addr<BroadcastActor>,
) -> Result<usize, Status> {
    let mut total_data_bytes_received: usize = 0;
    while let Some(flight_data_res) = stream.next().await {
        let flight_data = flight_data_res?;

        if flight_data.data_body.is_empty() {
            continue;
        }

        total_data_bytes_received += flight_data.data_body.len();

        handle_record_batch_put_message(
            &flight_data,
            schema,
            &Some(name.to_string()),
            broadcast.clone(),
        ).await?;
    }
    Ok(total_data_bytes_received)
}


/// We create a broadcast actor for each flight stream.
/// Each broadcast actor has 2 parser actors for parallelism
async fn initialize_actors(
    actor_registry: Addr<Registry>,
    config: Arc<Settings>,
    flight_name: &str,
) -> Result<Addr<BroadcastActor>, Status> {
    let factory_actor_addr = actor_registry
        .send(crate::platform::registry::FetchFactoryActor {})
        .await
        .map_err(|e| Status::internal(format!("Failed to get factory actor: {}", e)))?
        .map_err(|e | Status::internal("Failed to get factory actor: Not found"))?;

    let factory = match factory_actor_addr {
        Real(addr) => addr,
        _ => {
            return Err(Status::internal("Failed to get real factory actor"));
        }
    };

    let parser_type: ParserType = config.parser.clone().into();
    let parser_actors = factory
        .send(CreateParserActor {
            flight_name: flight_name.to_string(),
            count: 2,
            parser_type,
        })
        .await
        .map_err(|e| Status::internal(format!("Failed to create parser actors: {}", e)))?;

    let broadcast_actor = factory
        .send(CreateBroadcastActor {
            flight_name: flight_name.to_string(),
            parser_actors,
        })
        .await
        .map_err(|e| Status::internal(format!("Failed to create broadcast actor: {}", e)))?;

    Ok(broadcast_actor)
}