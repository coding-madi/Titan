use crate::application::actors::broadcast_actor::{BroadcastActor, Metadata, RecordBatchWrapper};
use crate::application::actors::db_actor::{DbActorAddr, SaveSchema};
use crate::application::actors::flight_registry_actor::{
    CheckFlight, Fields, FlightRegistryActorWrapped, RegisterFlight,
};
use crate::application::actors::iceberg_actor::{CreateTable, IcebergActorAddr};
use crate::core::error::exception::registry::RegistryError;
use crate::platform::registry::{
    FetchDbActor, FetchFlightRegistryActor, FetchIcebergActor, Registry,
};
use actix::Addr;
use actix_web::web::Bytes;
use arrow_array::RecordBatch;
use arrow_flight::utils::flight_data_to_arrow_batch;
use arrow_flight::{FlightData, FlightDescriptor, FlightInfo, SchemaAsIpc, SchemaResult};
use arrow_ipc::writer::{DictionaryTracker, IpcDataGenerator, IpcWriteOptions};
use arrow_schema::Schema;
use futures_core::Stream;
use futures_util::StreamExt;
use std::sync::Arc;
use tonic::Status;
use tracing::info;

/// Util to create flight info
pub fn create_flight_info(
    table_name: &str,
    batches: &[RecordBatch],
    ipc_options: &IpcWriteOptions,
) -> Result<FlightInfo, Status> {
    if batches.is_empty() {
        return Err(Status::not_found(format!(
            "No data found for {}",
            table_name
        )));
    }

    let schema = batches[0].schema();
    let schema_ipc = SchemaAsIpc::new(schema.as_ref(), ipc_options);

    let schema_result = SchemaResult::try_from(schema_ipc)
        .map_err(|e| Status::internal(format!("Failed to convert Schema: {e}")))?;

    let descriptor = FlightDescriptor::new_path(vec![table_name.to_string()]);

    let total_records: i64 = batches.iter().map(|batch| batch.num_rows() as i64).sum();

    let total_bytes: i64 = -1; // Placeholder, as before

    Ok(FlightInfo {
        flight_descriptor: Some(descriptor),
        schema: schema_result.schema,
        total_records,
        total_bytes,
        endpoint: vec![], // No endpoints specified, as before
        app_metadata: Bytes::new(),
        ordered: false,
    })
}

pub async fn get_flight_name(descriptor: &FlightDescriptor) -> Option<String> {
    info!("Received flight descriptor: {:?}", descriptor);
    match descriptor.path.get(0) {
        Some(path) => Some(path.to_string()),
        None => None,
    }
}

pub async fn create_arrow_schema(flight_data: &FlightData) -> Option<Arc<Schema>> {
    let schema = Arc::new(
        Schema::try_from(flight_data)
            .map_err(|e| Status::invalid_argument(format!("Failed to parse Schema: {}", e)))
            .unwrap(),
    );
    Some(schema.clone())
}

pub async fn register_flight_schema(
    registry: Addr<Registry>,
    schema: Arc<Schema>,
    flight_name: String,
) {
    save_schema(schema.clone(), &flight_name, registry.clone()).await;
    create_iceberg_table(schema.clone(), &flight_name, registry.clone()).await;
}

async fn save_schema(schema: Arc<Schema>, flight_name: &str, registry: Addr<Registry>) {
    let db_actor = registry.send(FetchDbActor).await;
    if let Ok(db_actor) = db_actor {
        if let Ok(db_actor) = db_actor {
            match db_actor {
                DbActorAddr::Real(db_actor) => {
                    let save_schema = SaveSchema {
                        flight_name: flight_name.parse().unwrap(),
                        schema: schema.clone(),
                        created_at: Default::default(),
                        updated_at: Default::default(),
                    };
                    let _ = db_actor.send(save_schema).await;
                }
                #[cfg(test)]
                DbActorAddr::Mock(_) => {}
                DbActorAddr::Empty => {
                    return;
                }
            }
        }
    }
}

async fn create_iceberg_table(schema: Arc<Schema>, flight_name: &str, registry: Addr<Registry>) {
    let iceberg_actor = registry.send(FetchIcebergActor).await;
    if let Ok(iceberg_actor) = iceberg_actor {
        if let Ok(iceberg_actor) = iceberg_actor {
            match iceberg_actor {
                IcebergActorAddr::Real(iceberg_actor) => {
                    let create_table = CreateTable {
                        table: flight_name.parse().unwrap(),
                        schema,
                        _partition_fields: vec![],
                    };
                    let _ = iceberg_actor.send(create_table).await;
                }
                _ => {}
            }
        }
    }
}

pub async fn schema_to_flight_data(
    schema: &Arc<Schema>,
    ipc_write_options: &IpcWriteOptions,
) -> Result<FlightData, Status> {
    let generator = IpcDataGenerator::default();
    let schema_flight_data: FlightData = generator
        .schema_to_bytes_with_dictionary_tracker(
            schema.as_ref(),
            &mut DictionaryTracker::new(false),
            ipc_write_options,
        )
        .into();
    Ok(schema_flight_data)
}

pub async fn encode_record_batch_flight_data(
    batch: &RecordBatch,
    ipc_write_options: &IpcWriteOptions,
    generator: &IpcDataGenerator,
) -> Result<(Vec<FlightData>, FlightData), Status> {
    let (dicts, batch_data) = generator
        .encoded_batch(batch, &mut DictionaryTracker::new(false), ipc_write_options)
        .map_err(|e| Status::internal(format!("Failed to encode batch: {e}")))?;

    let dict_flight_data: Vec<FlightData> = dicts.into_iter().map(Into::into).collect();
    Ok((dict_flight_data, batch_data.into()))
}

// TODO - parse columns and store in database.
// This will help us during plan evolutions and agent contract negotiations
pub async fn parse_columns_from_schema(schema: &Schema) -> Vec<Fields> {
    schema
        .fields()
        .iter()
        .map(|field| Fields {
            column_name: field.name().to_string(),
            data_type: field.data_type().to_string(),
        })
        .collect()
}

/// creates iceberg tables from schema
/// Stores the schema information in a database
pub async fn initialize_stream<S>(
    stream: &mut S,
    registry: Addr<Registry>,
) -> Result<(String, Arc<Schema>), Status>
where
    S: Stream<Item = Result<FlightData, Status>> + Unpin + Send + 'static,
{
    let mut flight_name: Option<String> = None;
    let mut schema: Option<Arc<Schema>> = None;

    // Process the first few messages to get the descriptor and schema.
    while let Some(flight_data_res) = stream.next().await {
        let flight_data = flight_data_res?;

        if flight_name.is_none() {
            if let Some(descriptor) = flight_data.flight_descriptor.clone() {
                flight_name = Some(
                    get_flight_name(&descriptor)
                        .await
                        .ok_or_else(|| Status::failed_precondition("Failed to set flight name"))?,
                );
                info!("Received flight descriptor: {:?}", descriptor);
            }
        }

        if schema.is_none() {
            if let Some(schema_arc) = create_arrow_schema(&flight_data).await {
                register_flight_schema(
                    registry.clone(),
                    schema_arc.clone(),
                    flight_name.clone().unwrap(),
                )
                .await;
                schema = Some(schema_arc.clone());
                // Register the flight also
                let fields = extract_columns_from_schema(&schema_arc).await;
                register_flight_columns_in_registry(registry.clone(), flight_name.clone(), fields)
                    .await;
                break; // Found schema, exit initialization loop
            }
        }

        // We must have a name before a schema.
        if flight_name.is_none() {
            return Err(Status::failed_precondition(
                "Received data before flight descriptor",
            ));
        }
    }

    match (flight_name, schema) {
        (Some(n), Some(s)) => Ok((n, s)),
        _ => Err(Status::failed_precondition(
            "Incomplete stream initialization: missing name or schema",
        )),
    }
}

pub async fn extract_columns_from_schema(schema: &Schema) -> Vec<Fields> {
    let fields: Vec<Fields> = schema
        .fields()
        .iter()
        .map(|f| Fields {
            column_name: "".to_string(),
            data_type: format!("{:?}", f.data_type()),
        })
        .collect();
    fields
}

async fn register_flight_columns_in_registry(
    registry: Addr<Registry>,
    flight_name: Option<String>,
    fields: Vec<Fields>,
) {
    let flight_actor_addr = registry
        .send(FetchFlightRegistryActor)
        .await
        .unwrap()
        .unwrap();
    let FlightRegistryActorWrapped::Real(flight_actor) = flight_actor_addr else {
        panic!("Failed to fetch flight registry actor")
    };
    flight_actor.do_send(RegisterFlight {
        flight: flight_name.clone().unwrap().to_string(),
        fields,
    });
}

/// Converts the data to RecordBuffers and adds metadata and sends
/// to broadcast actor where is is further processed
pub async fn handle_record_batch_put_message(
    flight_data: &FlightData,
    schema: &Arc<Schema>,
    flight_name: &str,
    broadcast_actor: Addr<BroadcastActor>,
) -> Result<(), Status> {
    if flight_data.data_body.is_empty() {
        return Ok(()); // No data in this message
    }

    let batch = flight_data_to_arrow_batch(
        flight_data,
        schema.clone(),
        &Default::default(), // DictionaryTracker
    )
    .map_err(|e| Status::internal(format!("Failed to convert to RecordBatch: {}", e)))?;
    let metadata = Metadata::new(flight_name, 1, schema.clone());
    let batch_wrapped = RecordBatchWrapper::new(metadata, &batch);

    broadcast_actor.do_send(batch_wrapped);
    let num_records_per_batch = batch.num_rows();
    info!("Batch received for flight: {flight_name} | rows: {num_records_per_batch}");
    Ok(())
}

pub async fn validate_if_flight_exists(
    name: &str,
    registry_actor: Addr<Registry>,
) -> Result<bool, RegistryError> {
    if let Ok(flight_registry_actor) = registry_actor.send(FetchFlightRegistryActor).await {
        // Validate if flight exists
        match flight_registry_actor.unwrap() {
            FlightRegistryActorWrapped::Real(flight_registry_actor) => {
                match flight_registry_actor
                    .send(CheckFlight {
                        flight: name.to_string(),
                    })
                    .await
                {
                    Ok(res) => Ok(res),
                    Err(e) => {
                        return Err(RegistryError::ActorNotInitialized(
                            "Some error in actor system".to_string(),
                        ));
                    }
                }
            }
            #[cfg(test)]
            FlightRegistryActorWrapped::Mock(_) => Ok(true),
            FlightRegistryActorWrapped::Empty => {
                Err(RegistryError::ActorNotInitialized("Actor not initialized".to_string()).into())
            }
        }
    } else {
        Err(RegistryError::ActorNotInitialized("Actor not initialized".to_string()).into())
    }
}
