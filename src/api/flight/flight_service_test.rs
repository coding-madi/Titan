#[cfg(test)]
mod test {

    use crate::api::flight::flight_service::LogFlightServer;
    use crate::application::actors::broadcast::BroadcastActorWrapper;
    use crate::application::actors::db::DbActorAddr;
    use crate::application::actors::flight_registry::FlightRegistryActorWrapped;
    use crate::application::actors::iceberg::IcebergActorAddr;
    use crate::application::actors::wal::WalActorWrapper;
    use crate::platform::registry::{ParserActorAddr, Registry};
    use actix::Actor;
    use arrow_array::{Int32Array, RecordBatch, StringArray};
    use arrow_flight::FlightData;
    use arrow_ipc::writer::{DictionaryTracker, IpcDataGenerator, IpcWriteOptions};
    use arrow_schema::{DataType, Field, Schema};
    use futures_util::stream;
    use std::sync::Arc;

    fn create_flight_data_vec() -> Vec<Result<FlightData, tonic::Status>> {
        // Define schema
        let schema = Arc::new(Schema::new(vec![
            Field::new("id", DataType::Int32, false),
            Field::new("name", DataType::Utf8, false),
        ]));

        // Create a record batch with sample data
        let batch = RecordBatch::try_new(
            schema.clone(),
            vec![
                Arc::new(Int32Array::from(vec![1, 2, 3])),
                Arc::new(StringArray::from(vec!["a", "b", "c"])),
            ],
        )
        .unwrap();

        let ipc_options = IpcWriteOptions::default();
        let mut dict_tracker = DictionaryTracker::new(false);
        let generator = IpcDataGenerator::default();

        // Serialize schema to FlightData
        let encoded_schema = generator.schema_to_bytes_with_dictionary_tracker(
            &schema,
            &mut dict_tracker,
            &ipc_options,
        );

        // Then explicitly convert `EncodedData` into `FlightData` from arrow_flight:
        let schema_data: FlightData = encoded_schema.into();

        // Serialize record batch to FlightData plus dictionaries
        let (dicts, batch_data) = generator
            .encoded_batch(&batch, &mut dict_tracker, &ipc_options)
            .unwrap();

        // Build vector of FlightData wrapped in Result for compatibility with tonic Streaming
        let mut flight_data_vec = Vec::new();
        flight_data_vec.push(Ok(schema_data));
        flight_data_vec.extend(dicts.into_iter().map(|d| Ok(d.into())));
        flight_data_vec.push(Ok(batch_data.into()));
        let (_dicts, batch_data) = generator
            .encoded_batch(&batch, &mut dict_tracker, &ipc_options)
            .unwrap();
        flight_data_vec.push(Ok(batch_data.into()));
        let (_dicts, batch_data) = generator
            .encoded_batch(&batch, &mut dict_tracker, &ipc_options)
            .unwrap();
        flight_data_vec.push(Ok(batch_data.into()));

        flight_data_vec
    }

    #[actix_rt::test]
    async fn test_flight_service() {
        let registry = Registry {
            db_actor_addr: DbActorAddr::Empty,
            broadcast_actor_addr: BroadcastActorWrapper::Empty,
            flight_registry_actor_addr: FlightRegistryActorWrapped::Empty,
            iceberg_actor_addr: IcebergActorAddr::Empty,
            parser_actor_addr: ParserActorAddr::Empty,
            wal_actor_addr: WalActorWrapper::Empty,
        };

        let _registry_addr = registry.clone().start();

        let flight_data_stream = create_flight_data_vec();

        let server = LogFlightServer::new(Arc::new(registry));
        let results = server
            .do_put_from_stream(stream::iter(flight_data_stream))
            .await
            .unwrap();

        assert_eq!(results.len(), 3);
    }
}
