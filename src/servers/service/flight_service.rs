use crate::actors::broadcast::Broadcaster;
use actix::{Addr, dev::Stream};
use arrow::datatypes::Schema;
use arrow::record_batch::RecordBatch;
use arrow_flight::utils::flight_data_to_arrow_batch;
use arrow_flight::{
    Action, ActionType, Criteria, Empty, FlightData, FlightDescriptor, FlightInfo,
    HandshakeRequest, HandshakeResponse, PollInfo, PutResult, SchemaResult, Ticket,
    flight_service_server::FlightService,
};
use futures_util::StreamExt;
use std::{collections::HashMap, pin::Pin, sync::Arc};
use tokio::sync::Mutex;
use tonic::{Request, Response, Status, Streaming};
use tracing::info;
pub struct LogFlightServer {
    pub data: Arc<Mutex<HashMap<String, RecordBatch>>>,
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

            let combined_batch = if let Some(old_batch) = data.get(name.as_ref().unwrap()) {
                let all_columns = old_batch
                    .columns()
                    .iter()
                    .zip(received_batches[0].columns()) // assuming the schema is not changing mid-stream
                    .map(|(old, new_batch)| {
                        arrow::compute::concat(&[old.as_ref(), new_batch.as_ref()]).unwrap()
                    })
                    .collect::<Vec<_>>();
                RecordBatch::try_new(old_batch.schema().clone(), all_columns).unwrap()
            } else {
                received_batches[0].clone()
            };
            data.insert(name.unwrap().clone(), combined_batch);
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
        unimplemented!()
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
        unimplemented!()
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
