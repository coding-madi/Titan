use crate::application::actors::broadcast::{BroadcastActorAddr, Metadata, RecordBatchWrapper};
use crate::application::actors::db::{DbActorAddr, SaveSchema};
use crate::application::actors::flight_registry::{Fields, FlightRegistryActorAddr};
use crate::platform::registry::Registry;
use actix::WrapStream;
use actix::dev::Stream;
use actix_web::web::Bytes;
use arrow::datatypes::Schema;
use arrow::record_batch::RecordBatch;
use arrow_flight::SchemaAsIpc;
use arrow_flight::utils::flight_data_to_arrow_batch;
use arrow_flight::{
    Action, ActionType, Criteria, Empty, FlightData, FlightDescriptor, FlightInfo,
    HandshakeRequest, HandshakeResponse, PollInfo, PutResult, SchemaResult, Ticket,
    flight_service_server::FlightService,
};
use arrow_ipc::writer::{DictionaryTracker, IpcDataGenerator, IpcWriteOptions};
use futures::stream;
use futures_util::StreamExt;
use std::vec;
use std::{collections::HashMap, pin::Pin, sync::Arc};
use tokio::sync::Mutex;
use tonic::{Request, Response, Status, Streaming};
use tracing::info;

pub struct LogFlightServer {
    pub data: Arc<Mutex<HashMap<String, Vec<RecordBatch>>>>,
    pub actor_registry: Arc<Registry>,
}

impl LogFlightServer {
    pub fn new(actor_registry: Arc<Registry>) -> Self {
        Self {
            data: Arc::new(Mutex::new(HashMap::new())),
            actor_registry,
        }
    }

    /// Helper to create a single FlightInfo from a table's data.
    pub fn create_flight_info(
        &self,
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

        let ticket = Ticket {
            ticket: Bytes::from(table_name.to_string()),
        };

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

    /// Helper to encode a schema into FlightData.
    fn encode_schema_flight_data(
        &self,
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

    /// Helper to encode a RecordBatch into FlightData.
    fn encode_record_batch_flight_data(
        &self,
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

    /// Handles the initial FlightData message containing descriptor and schema.
    async fn handle_initial_put_message(
        &self,
        flight_data: &FlightData,
        name: &mut Option<String>,
        schema_opt: &mut Option<Arc<Schema>>,
    ) -> Result<(), Status> {
        // First message may contain descriptor
        if name.is_none() {
            if let Some(descriptor) = flight_data.flight_descriptor.clone() {
                info!("Received flight descriptor: {:?}", descriptor);
                *name = Some(
                    descriptor
                        .path
                        .get(0)
                        .ok_or_else(|| Status::invalid_argument("Flight descriptor path is empty"))?
                        .to_string(),
                );
            }
        }

        // Try to parse Schema if not already parsed and data_header is present
        if schema_opt.is_none() && !flight_data.data_header.is_empty() {
            let schema =
                Arc::new(Schema::try_from(flight_data).map_err(|e| {
                    Status::invalid_argument(format!("Failed to parse Schema: {}", e))
                })?);
            *schema_opt = Some(schema.clone());

            let save_schema = SaveSchema {
                flight_name: name.clone().unwrap_or_default(), // Use default if name is still none
                schema: schema.clone(),
                created_at: Default::default(),
                updated_at: Default::default(),
            };

            // This loop does nothing, can be removed if not intended for something else
            let _fields: Vec<Fields> = schema
                .fields()
                .iter()
                .map(|field| Fields {
                    column_name: field.name().to_string(),
                    data_type: field.data_type().to_string(),
                })
                .collect(); // If you intend to use `_fields`, ensure it's used or the loop is removed

            // Persist the schema in database
            let db = self.actor_registry.db_actor_addr.clone();
            let flight_registry = self.actor_registry.flight_registry_actor_addr.clone();

            match db {
                DbActorAddr::Real(db_actor) => {
                    db_actor.do_send(save_schema);
                    info!("Schema saved in database");
                    match flight_registry {
                        FlightRegistryActorAddr::Real(_flight_registry_actor) => { /* actor usage */
                        }
                        #[cfg(test)]
                        FlightRegistryActorAddr::Mock(_) => {}
                        _ => {}
                    }
                }
                #[cfg(test)]
                DbActorAddr::Mock(_) => {}
                _ => {}
            }
        }
        Ok(())
    }

    /// Handles a FlightData message containing actual RecordBatch data.
    async fn handle_record_batch_put_message(
        &self,
        flight_data: &FlightData,
        schema: &Arc<Schema>,
        name: &Option<String>,
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

        let batch_wrapped = RecordBatchWrapper {
            metadata: Metadata {
                flight: name.clone().unwrap_or_default(),
                schema: schema.clone(),
                buffer_id: 1, // Consider if this ID should be dynamic/unique
                service_id: name.clone().unwrap_or_default(),
            },
            data: Arc::new(batch.clone()),
        };

        match &self.actor_registry.broadcast_actor_addr {
            BroadcastActorAddr::Real(addr) => {
                addr.do_send(batch_wrapped);
            }
            #[cfg(test)]
            BroadcastActorAddr::Mock(_) => {}
            _ => {}
        }

        info!(
            "Batch received for flight: {} | rows: {}",
            name.clone().unwrap_or_default(),
            batch.num_rows()
        );
        Ok(())
    }

    pub async fn do_put_from_stream<S>(&self, mut stream: S) -> Result<Vec<PutResult>, Status>
    where
        S: futures_core::Stream<Item = Result<FlightData, Status>> + Unpin + Send + 'static,
    {
        // let mut flight_data_stream = request.into_inner();
        let mut name: Option<String> = None;
        let mut schema_opt: Option<Arc<Schema>> = None;
        let mut put_result: Vec<PutResult> = vec![];
        while let Some(flight_data_res) = stream.next().await {
            let flight_data = flight_data_res?;

            // Handle initial message containing descriptor and/or schema
            if name.is_none() || schema_opt.is_none() {
                self.handle_initial_put_message(&flight_data, &mut name, &mut schema_opt)
                    .await?;
                // If this was a schema message (no body), continue to next message
                if schema_opt.is_some() && flight_data.data_body.is_empty() {
                    continue;
                }
            }

            // Handle record batch data (if schema is available)
            if let Some(schema) = &schema_opt {
                self.handle_record_batch_put_message(&flight_data, schema, &name)
                    .await?;
                put_result.push(PutResult {
                    app_metadata: Bytes::from_static(b"batch committed"),
                });
            } else {
                put_result.push(PutResult {
                    app_metadata: Bytes::from_static(b"batch failed"),
                });
                return Err(Status::failed_precondition("Received data before Schema"));
            }
        }
        // let result_stream = futures::stream::once(async { Ok(PutResult::default()) });
        Ok(put_result)
    }
}

#[tonic::async_trait]
impl FlightService for LogFlightServer {
    type HandshakeStream = Pin<Box<dyn Stream<Item = Result<HandshakeResponse, Status>> + Send>>;
    async fn handshake(
        &self,
        _request: Request<Streaming<HandshakeRequest>>,
    ) -> Result<Response<Self::HandshakeStream>, Status> {
        unimplemented!()
    }
    type ListFlightsStream = Pin<Box<dyn Stream<Item = Result<FlightInfo, Status>> + Send>>;

    async fn list_flights(
        &self,
        _request: Request<Criteria>,
    ) -> Result<Response<Self::ListFlightsStream>, Status> {
        let data = self.data.lock().await;
        let ipc_options = IpcWriteOptions::default();

        let flight_infos: Vec<Result<FlightInfo, Status>> = data
            .iter()
            .map(|(table_name, batches)| self.create_flight_info(table_name, batches, &ipc_options))
            .collect();

        let output_stream = futures::stream::iter(flight_infos);
        Ok(Response::new(Box::pin(output_stream)))
    }

    async fn get_flight_info(
        &self,
        _request: Request<FlightDescriptor>,
    ) -> Result<Response<FlightInfo>, Status> {
        unimplemented!()
    }

    async fn poll_flight_info(
        &self,
        _request: Request<FlightDescriptor>,
    ) -> Result<Response<PollInfo>, Status> {
        unimplemented!()
    }

    async fn get_schema(
        &self,
        _request: Request<FlightDescriptor>,
    ) -> Result<Response<SchemaResult>, Status> {
        unimplemented!()
    }

    type DoGetStream = Pin<Box<dyn Stream<Item = Result<FlightData, Status>> + Send>>;

    async fn do_get(
        &self,
        _request: Request<Ticket>,
    ) -> Result<Response<Self::DoGetStream>, Status> {
        let ticket = _request.into_inner();
        let data_lock = self.data.lock().await; // Acquire lock once
        let table_name = String::from_utf8(ticket.ticket.to_vec())
            .map_err(|_| Status::invalid_argument("Invalid ticket encoding"))?;

        let batches = data_lock
            .get(&table_name)
            .cloned()
            .ok_or_else(|| Status::not_found(format!("No data found for table '{table_name}'")))?;

        if batches.is_empty() {
            return Err(Status::not_found(format!(
                "No record batches for '{}'",
                table_name
            )));
        }

        let schema = batches[0].schema();
        let ipc_write_options = IpcWriteOptions::default();
        let generator = IpcDataGenerator::default();

        let mut all_flight_data: Vec<Result<FlightData, Status>> = vec![];

        // Add schema FlightData
        all_flight_data.push(self.encode_schema_flight_data(&schema, &ipc_write_options));

        // Add dictionary and record batch FlightData
        for batch in batches {
            let (dicts, batch_data) =
                self.encode_record_batch_flight_data(&batch, &ipc_write_options, &generator)?;
            for d in dicts {
                all_flight_data.push(Ok(d));
            }
            all_flight_data.push(Ok(batch_data));
        }

        let output_stream = stream::iter(all_flight_data);
        Ok(Response::new(Box::pin(output_stream) as Self::DoGetStream))
    }

    type DoPutStream = Pin<Box<dyn Stream<Item = Result<PutResult, Status>> + Send>>;

    // Send data to broacast actor
    async fn do_put(
        &self,
        request: Request<Streaming<FlightData>>,
    ) -> Result<Response<Self::DoPutStream>, Status> {
        let result = self.do_put_from_stream(request.into_inner()).await;
        let Ok(results) = result else {
            return Err(Status::internal("Failed to process data"));
        };
        let stream = stream::iter(results.into_iter().map(Ok)); // Wrap each PutResult as Ok(PutResult)
        Ok(Response::new(Box::pin(stream)))
    }
    type DoExchangeStream = Pin<Box<dyn Stream<Item = Result<FlightData, Status>> + Send>>;
    async fn do_exchange(
        &self,
        _request: Request<Streaming<FlightData>>,
    ) -> Result<Response<Self::DoExchangeStream>, Status> {
        unimplemented!()
    }
    type DoActionStream = Pin<Box<dyn Stream<Item = Result<arrow_flight::Result, Status>> + Send>>;

    async fn do_action(
        &self,
        _request: Request<Action>,
    ) -> Result<Response<Self::DoActionStream>, Status> {
        unimplemented!()
    }
    type ListActionsStream = Pin<Box<dyn Stream<Item = Result<ActionType, Status>> + Send>>;
    async fn list_actions(
        &self,
        _request: Request<Empty>,
    ) -> Result<Response<Self::ListActionsStream>, Status> {
        unimplemented!()
    }
}
