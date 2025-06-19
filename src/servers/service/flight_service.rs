use crate::actors::broadcast::Broadcaster;
use actix::{Addr, dev::Stream};
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
    pub broadcast_actor: Arc<Addr<Broadcaster>>,
}

impl LogFlightServer {
    pub fn new(broadcast_actor: Arc<Addr<Broadcaster>>) -> Self {
        LogFlightServer {
            data: Arc::new(Mutex::new(HashMap::new())),
            broadcast_actor: broadcast_actor,
        }
    }
}

#[tonic::async_trait]
impl FlightService for LogFlightServer {
    type DoPutStream = Pin<Box<dyn Stream<Item = Result<PutResult, Status>> + Send>>;
    type HandshakeStream = Pin<Box<dyn Stream<Item = Result<HandshakeResponse, Status>> + Send>>;
    type ListFlightsStream = Pin<Box<dyn Stream<Item = Result<FlightInfo, Status>> + Send>>;
    type DoExchangeStream = Pin<Box<dyn Stream<Item = Result<FlightData, Status>> + Send>>;
    type DoActionStream = Pin<Box<dyn Stream<Item = Result<arrow_flight::Result, Status>> + Send>>;
    type DoGetStream = Pin<Box<dyn Stream<Item = Result<FlightData, Status>> + Send>>;
    type ListActionsStream = Pin<Box<dyn Stream<Item = Result<ActionType, Status>> + Send>>;

    // Send data to broacast actor
    async fn do_put(
        &self,
        request: Request<Streaming<FlightData>>,
    ) -> Result<Response<Self::DoPutStream>, Status> {
        let mut flight_data_stream = request.into_inner();
        let mut descriptor_opt: Option<arrow_flight::FlightDescriptor> = None;
        let mut schema_opt: Option<Arc<Schema>> = None;
        let mut received_batches: Vec<RecordBatch> = Vec::new();
        let mut name: Option<String> = Option::None;

        while let Some(flight_data_res) = flight_data_stream.next().await {
            let flight_data = flight_data_res?; // Automatically handles errors

            // First message may contain descriptor
            if descriptor_opt.is_none() {
                if let Some(descriptor) = flight_data.flight_descriptor.clone() {
                    descriptor_opt = Some(descriptor.clone());
                    info!("Received flight descriptor: {:?}", descriptor_opt);
                    name = Option::Some(
                        descriptor
                            .path
                            .get(0)
                            .ok_or_else(|| {
                                Status::invalid_argument("Flight descriptor path is empty")
                            })?
                            .to_string(),
                    );
                }
            }

            // Try to parse schema if not already parsed
            if schema_opt.is_none() && !flight_data.data_header.is_empty() {
                let schema = Schema::try_from(&flight_data).map_err(|e| {
                    Status::invalid_argument(format!("Failed to parse schema: {}", e))
                })?;
                schema_opt = Some(Arc::new(schema));
                continue; // Schema messages do not contain data
            }

            // Parse actual RecordBatch
            if let Some(schema) = &schema_opt {
                if !flight_data.data_body.is_empty() {
                    let batch = flight_data_to_arrow_batch(
                        &flight_data,
                        schema.clone(),
                        &Default::default(),
                    )
                    .map_err(|e| {
                        Status::internal(format!("Failed to convert to RecordBatch: {}", e))
                    })?;
                    received_batches.push(batch);
                }
            } else {
                return Err(Status::failed_precondition("Received data before schema"));
            }
        }

        // TODO
        // Combine all the received RecordBatches into a single one
        if !received_batches.is_empty() {
            let mut data = self.data.lock().await;

            let entry = data.entry(name.unwrap().clone()).or_insert_with(Vec::new);
            entry.extend(received_batches);
        }

        let result_stream = futures::stream::once(async { Ok(PutResult::default()) });
        Ok(Response::new(Box::pin(result_stream)))
    }

    async fn handshake(
        &self,
        _request: Request<Streaming<HandshakeRequest>>,
    ) -> Result<Response<Self::HandshakeStream>, Status> {
        unimplemented!()
    }

    async fn list_flights(
        &self,
        _request: Request<Criteria>,
    ) -> Result<Response<Self::ListFlightsStream>, Status> {
        let data = self.data.lock().await;

        let ipc_options = IpcWriteOptions::default();

        let flight_infos: Vec<Option<FlightInfo>> = data
            .iter()
            .map(|(table_name, batches)| {
                let schema = batches
                    .first()
                    .map_or_else(
                        || {
                            Err(Status::not_found(format!(
                                "No data found for {}",
                                &table_name
                            )))
                        },
                        |batch| Ok(batch.schema().clone()),
                    )
                    .unwrap();

                let schema_ipc = SchemaAsIpc::new(schema.as_ref(), &ipc_options);

                let schema_result = SchemaResult::try_from(schema_ipc)
                    .map_err(|e| Status::internal(format!("Failed to convert schema: {}", e)))
                    .unwrap();

                let descriptor = FlightDescriptor::new_path(vec![table_name.clone()]);

                let ticket = Ticket {
                    ticket: Bytes::from(table_name.clone()),
                };

                let endpoint = arrow_flight::FlightEndpoint {
                    ticket: Some(ticket),
                    location: vec![],
                    expiration_time: None,
                    app_metadata: Bytes::new(),
                };

                let total_records: i64 = batches
                    .iter()
                    .map(|batch| batch.num_rows() as i64) // Cast each usize to i64
                    .sum(); // Now sums an iterator of i64

                let total_bytes: i64 = -1;

                let flight_info = FlightInfo {
                    flight_descriptor: Some(descriptor),
                    schema: schema_result.schema,
                    total_records: total_records,
                    total_bytes: total_bytes,
                    endpoint: vec![],
                    app_metadata: Bytes::new(),
                    ordered: false,
                };

                Some(flight_info)
            })
            .collect();

        let output_stream = futures::stream::iter(
            flight_infos
                .into_iter()
                .map(|info| info.ok_or_else(|| Status::not_found("No flight info found"))),
        );
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
    async fn do_get(
        &self,
        _request: Request<Ticket>,
    ) -> Result<Response<Self::DoGetStream>, Status> {
        let ticket = _request.into_inner();
        let data = self.data.lock().await;
        let table_name = String::from_utf8(ticket.ticket.to_vec())
            .map_err(|_| Status::invalid_argument("Invalid ticket encoding"))?;
        let batches = match data.get(&table_name) {
            Some(b) => b.clone(),
            None => {
                return Err(Status::not_found(format!(
                    "No data found for table '{}'",
                    table_name
                )));
            }
        };

        if batches.is_empty() {
            return Err(Status::not_found(format!(
                "No record batches for '{}'",
                table_name
            )));
        }

        let schema = batches[0].schema();
        let ipc_write_options = IpcWriteOptions::default();
        let generator = IpcDataGenerator::default();

        let schema_flight_data: FlightData = generator
            .schema_to_bytes_with_dictionary_tracker(
                schema.as_ref(),
                &mut DictionaryTracker::new(false),
                &ipc_write_options,
            )
            .into();
        let mut all_flight_data: Vec<Result<FlightData, Status>> = vec![Ok(schema_flight_data)];

        for batch in batches {
            let (dicts, batch_data) = generator
                .encoded_batch(
                    &batch,
                    &mut DictionaryTracker::new(false),
                    &ipc_write_options,
                )
                .map_err(|e| Status::internal(format!("Failed to encode batch: {e}")))?;

            for d in dicts {
                all_flight_data.push(Ok(d.into()));
            }

            all_flight_data.push(Ok(batch_data.into()));
        }

        let output_stream = stream::iter(all_flight_data);
        Ok(Response::new(Box::pin(output_stream) as Self::DoGetStream))
    }

    async fn do_exchange(
        &self,
        _request: Request<Streaming<FlightData>>,
    ) -> Result<Response<Self::DoExchangeStream>, Status> {
        unimplemented!()
    }
    async fn do_action(
        &self,
        _request: Request<Action>,
    ) -> Result<Response<Self::DoActionStream>, Status> {
        unimplemented!()
    }
    async fn list_actions(
        &self,
        _request: Request<Empty>,
    ) -> Result<Response<Self::ListActionsStream>, Status> {
        unimplemented!()
    }
}
