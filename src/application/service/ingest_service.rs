use crate::application::actors::broadcaster::broadcast_actor::BroadcastActor;
use crate::application::actors::factory::factory_actor::CreateBroadcastActor;
use crate::application::actors::factory::factory_actor::FactoryActorAddr::Real;
use crate::application::actors::messages::registeration::{CreateParserActor, CreateRhaiActor};
use crate::config::yaml_reader::Settings;
use crate::core::parser::parser_contract::ParserType;
use crate::core::utils::flight::{handle_record_batch_put_message, initialize_stream};
use crate::monitor::prometheus::registry::{ACTIVE_SESSIONS, COUNTER, REGISTRY};
use crate::platform::registry::Registry;
use actix::Addr;
use actix_web::web::Bytes;
use arrow_flight::{FlightData, PutResult};
use futures_core::Stream;
use futures_util::StreamExt;
use std::pin::Pin;
use std::sync::Arc;
use tonic::Status;
use tracing::info;

pub struct InjestService {
    actor_registry: Addr<Registry>,
    config: Arc<Settings>,
}

impl InjestService {
    pub fn new(actor_registry: Addr<Registry>, config: Arc<Settings>) -> Self {
        Self {
            actor_registry,
            config,
        }
    }

    pub async fn do_put_from_stream<S>(&self, mut stream: S) -> Result<Vec<PutResult>, Status>
    where
        S: Stream<Item = Result<FlightData, Status>> + Unpin + Send + 'static,
    {
        // Register the active flight streams count
        ACTIVE_SESSIONS.with_label_values(&["ingest"]).inc();
        // 1. Process initial metadata and schema.
        // Register the details in Flight registry
        let (flight_name, arrow_schema) =
            initialize_stream(&mut stream, self.actor_registry.clone()).await?;

        // 2. Initialize the necessary actors.
        let broadcast = initialize_actors(
            self.actor_registry.clone(),
            self.config.clone(),
            &flight_name,
        )
        .await?;

        let mut stream: FlightStream = Box::pin(stream);

        // 3. Process the data batches and get the total bytes received.
        let total_data_bytes_received =
            process_flight_stream(&mut stream, &flight_name, &arrow_schema, broadcast.clone())
                .await?;

        // 4. Create and return the final results.
        let metadata_string = format!("Bytes:{}", total_data_bytes_received);
        let app_metadata = Bytes::from(metadata_string.clone().into_bytes());
        info!("Server finished processing stream. Metrics: {metadata_string}");

        let put_result = PutResult {
            app_metadata,
            ..Default::default()
        };
        ACTIVE_SESSIONS.with_label_values(&["ingest"]).dec();
        Ok(vec![put_result])
    }
}

type FlightStream = Pin<Box<dyn Stream<Item = Result<FlightData, Status>> + Send>>;

async fn process_flight_stream(
    stream: &mut FlightStream,
    flight_name: &str,
    schema: &arrow_schema::SchemaRef,
    broadcast: Addr<BroadcastActor>,
) -> Result<usize, Status> {
    let mut total_data_bytes_received: usize = 0;
    while let Some(flight_data_res) = stream.next().await {
        let flight_data = flight_data_res?;

        if flight_data.data_body.is_empty() {
            continue;
        }

        let size_of_current_buffer = flight_data.data_body.len();
        total_data_bytes_received += size_of_current_buffer;

        println!(
            "total_data_bytes_received: {}",
            size_of_current_buffer as u64 / 1024
        );
        COUNTER
            .with_label_values(&[flight_name, "KB_received"])
            .inc_by((size_of_current_buffer / 1024) as u64);

        handle_record_batch_put_message(&flight_data, schema, flight_name, broadcast.clone())
            .await?;
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
    info!("Initializing actors for flight: {}", flight_name);
    let factory_actor_addr = actor_registry
        .send(crate::platform::registry::FetchFactoryActor {})
        .await
        .map_err(|e| Status::internal(format!("Failed to get factory actor: {}", e)))?
        .map_err(|_e| Status::internal("Failed to get factory actor: Not found"))?;

    let factory = match factory_actor_addr {
        Real(addr) => addr,
        _ => {
            return Err(Status::internal("Failed to get real factory actor"));
        }
    };

    let rhai_actor = factory
        .send(CreateRhaiActor {
            flight_name: flight_name.to_string(),
        })
        .await
        .map_err(|e| Status::internal(format!("Failed to create rhai actor: {}", e)))?;

    let parser_type: ParserType = config.parser.clone().into();
    let parser_actors = factory
        .send(CreateParserActor {
            flight_name: flight_name.to_string(),
            rhai_actor: rhai_actor.clone(),
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
