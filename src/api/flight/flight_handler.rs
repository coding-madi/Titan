use crate::application::service::ingest_service::InjestService;
use crate::monitor::prometheus::registry::{
    ACTIVE_SESSIONS, COUNTER, JOB_LATENCY_HISTOGRAM, REGISTRY,
};
use actix::dev::Stream;
use arrow_flight::{
    Action, ActionType, Criteria, Empty, FlightData, FlightDescriptor, FlightInfo,
    HandshakeRequest, HandshakeResponse, PollInfo, PutResult, SchemaResult, Ticket,
    flight_service_server::FlightService,
};
use futures::stream;
use futures_util::StreamExt;
use std::pin::Pin;
use tonic::{Request, Response, Status, Streaming};
use tracing::error;

pub struct LogFlightServer {
    pub injest_service: InjestService,
}

impl LogFlightServer {
    pub fn new(injest_service: InjestService) -> Self {
        Self { injest_service }
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
        unimplemented!()
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
        // let ticket = _request.into_inner();
        // let data_lock = self.data.lock().await; // Acquire lock once
        // let table_name = String::from_utf8(ticket.ticket.to_vec())
        //     .map_err(|_| Status::invalid_argument("Invalid ticket encoding"))?;
        //
        // let batches = data_lock
        //     .get(&table_name)
        //     .cloned()
        //     .ok_or_else(|| Status::not_found(format!("No data found for table '{table_name}'")))?;
        //
        // if batches.is_empty() {
        //     return Err(Status::not_found(format!(
        //         "No record batches for '{}'",
        //         table_name
        //     )));
        // }
        //
        // let schema = batches[0].schema();
        // let ipc_write_options = IpcWriteOptions::default();
        // let generator = IpcDataGenerator::default();
        //
        // let mut all_flight_data: Vec<Result<FlightData, Status>> = vec![];
        //
        // // Add schema FlightData
        // all_flight_data.push(schema_to_flight_data(&schema, &ipc_write_options).await);
        //
        // // Add dictionary and record batch FlightData
        // for batch in batches {
        //     let (dicts, batch_data) =
        //         encode_record_batch_flight_data(&batch, &ipc_write_options, &generator).await?;
        //     for d in dicts {
        //         all_flight_data.push(Ok(d));
        //     }
        //     all_flight_data.push(Ok(batch_data));
        // }
        //
        // let output_stream = stream::iter(all_flight_data);
        // Ok(Response::new(Box::pin(output_stream) as Self::DoGetStream))
        unimplemented!()
    }

    type DoPutStream = Pin<Box<dyn Stream<Item = Result<PutResult, Status>> + Send>>;

    // Send data to broacast actor
    async fn do_put(
        &self,
        request: Request<Streaming<FlightData>>,
    ) -> Result<Response<Self::DoPutStream>, Status> {
        let results = match self
            .injest_service
            .do_put_from_stream(request.into_inner())
            .await
        {
            Ok(r) => r,
            Err(e) => {
                error!("do_put_from_stream failed: {e:?}");
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
