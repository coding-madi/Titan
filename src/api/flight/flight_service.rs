use crate::application::actors::broadcast::{
    BroadcastActor, BroadcastActorWrapper, Metadata, RecordBatchWrapper,
};
use crate::application::actors::db::{DbActorAddr, SaveSchema};
use crate::application::actors::factory_actor::FactoryActorAddr::Real;
use crate::application::actors::factory_actor::{CreateBroadcastActor, CreateParserActor};
use crate::application::actors::flight_registry::{Fields, FlightRegistryActorWrapped};
use crate::application::actors::iceberg::{CreateTable, IcebergActorAddr};
use crate::core::utils::flight::{
    create_flight_info, encode_record_batch_flight_data, handle_record_batch_put_message,
    initialize_stream, register_flight_schema, schema_to_flight_data, set_flight_name, set_schema,
};
use crate::platform::registry::{
    FetchBroadcastActor, FetchDbActor, FetchFactoryActor, FetchFlightRegistryActor,
    FetchIcebergActor, ParserActor, Registry,
};
use actix::dev::Stream;
use actix::{Actor, Addr};
use actix_rt::Arbiter;
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
use futures_util::{SinkExt, StreamExt};
use std::time::Instant;
use std::vec;
use std::{collections::HashMap, pin::Pin, sync::Arc};
use tokio::sync::Mutex;
use tonic::{Request, Response, Status, Streaming};
use tracing::{error, info};

pub struct LogFlightServer {
    pub data: Arc<Mutex<HashMap<String, Vec<RecordBatch>>>>,
    pub actor_registry: Addr<Registry>,
}

impl LogFlightServer {
    pub fn new(actor_registry: Addr<Registry>) -> Self {
        Self {
            data: Arc::new(Mutex::new(HashMap::new())),
            actor_registry,
        }
    }

    // DOING -
    async fn do_put_from_stream<S>(&self, mut stream: S) -> Result<Vec<PutResult>, Status>
    where
        S: Stream<Item = Result<FlightData, Status>> + Unpin + Send + 'static,
    {
        let stream_start_time = Instant::now();
        let mut total_data_bytes_received: usize = 0; // To track only data body bytes
        // 1. Process the initial metadata and schema messages.
        let (name, schema) = initialize_stream(&mut stream, self.actor_registry.clone()).await?;

        let name_copy = name.clone();
        let actor_registry_addr = self.actor_registry.clone();

        // Get the factor actor that can create actors in the actix context.
        let factory = actor_registry_addr
            .send(FetchFactoryActor {})
            .await
            .unwrap()
            .unwrap();

        let broadcast = match factory {
            Real(factory_actor_addr) => {
                let parser_actors = factory_actor_addr
                    .send(CreateParserActor {
                        flight_name: name_copy.clone(),
                        count: 2,
                    })
                    .await
                    .unwrap();

                let broadcast_actor = factory_actor_addr
                    .send(CreateBroadcastActor {
                        flight_name: name_copy.clone(),
                        parser_actors,
                    })
                    .await
                    .unwrap();
                broadcast_actor
            }
            _ => {
                error!("Failed to get factory actor");
                return Err(Status::internal("Failed to get factory actor"));
            }
        };

        // 2. Process the remaining data batches.
        let mut put_results: Vec<PutResult> = vec![];
        while let Some(flight_data_res) = stream.next().await {
            let flight_data = flight_data_res?;

            // Skip empty data messages that are not the schema message
            if flight_data.data_body.is_empty() {
                continue;
            }

            total_data_bytes_received += flight_data.data_body.len();

            handle_record_batch_put_message(
                &flight_data,
                &schema,
                &Some(name.clone()),
                broadcast.clone(),
            )
            .await?;
        }
        let total_server_duration = stream_start_time.elapsed().as_secs_f64();
        let metadata_string = format!(
            "Duration:{:.4},Bytes:{}",
            total_server_duration, total_data_bytes_received
        );
        info!(
            "Server finished processing stream. Metrics: {}",
            metadata_string
        );
        let put_result = PutResult {
            app_metadata: Bytes::from(metadata_string.into_bytes()), // Convert String to Bytes
                                                                     // Other fields in PutResult can be left default or populated as needed.
                                                                     // For example, if you want to include some summary stats for the data received:
                                                                     // row_count: -1, // Or actual count if tracked
                                                                     // schema_uri: "".to_string(),
        };
        put_results.insert(0, put_result);
        Ok(put_results)
    }
}

#[tonic::async_trait]
impl FlightService for LogFlightServer {
    type HandshakeStream = Pin<Box<dyn Stream<Item = Result<HandshakeResponse, Status>> + Send>>;

    /// TODO: Implement handshake and auth
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
            .map(|(table_name, batches)| create_flight_info(table_name, batches, &ipc_options))
            .collect();

        let output_stream = futures::stream::iter(flight_infos);
        Ok(Response::new(Box::pin(output_stream)))
    }

    /// TODO: Implement handshake and auth
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
        all_flight_data.push(schema_to_flight_data(&schema, &ipc_write_options).await);

        // Add dictionary and record batch FlightData
        for batch in batches {
            let (dicts, batch_data) =
                encode_record_batch_flight_data(&batch, &ipc_write_options, &generator).await?;
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
        let results = match self.do_put_from_stream(request.into_inner()).await {
            Ok(r) => r,
            Err(e) => {
                tracing::error!("do_put_from_stream failed: {e:?}");
                return Err(e);
            }
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
