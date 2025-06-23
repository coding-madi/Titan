use arrow_ipc::writer::{DictionaryTracker, IpcDataGenerator, IpcWriteOptions, StreamWriter};
use chrono::Utc;
use flatbuffers::FlatBufferBuilder;

// Using fully qualified paths for generated Flatbuffers types for clarity
// This assumes 'wal_schema_generated' is generated into a 'wal' module.
use crate::actors::broadcast::{Metadata, RecordBatchWrapper};
use crate::schema::wal_schema_generated::wal::{
    FlatbufMeta,
    FlatbufMetaArgs,
    LogMeta,
    LogMetaArgs,
    Metadata as WalMetadataUnion, // Renamed to avoid clash with crate::actors::broadcast::Metadata
    Serialization,
};

/// Serializes an Arrow RecordBatch into a byte buffer without including the schema.
///
/// This function generates the IPC message and data for dictionaries and the record batch,
/// then concatenates them into a single `Vec<u8>`.
///
/// # Arguments
/// * `record_batch_wrapper` - A wrapper containing the Arrow RecordBatch to serialize.
///
/// # Returns
/// A `Vec<u8>` containing the serialized Arrow data.
///
/// # Panics
/// Panics if the RecordBatch fails to encode.
pub fn serialize_record_batch_without_schema(record_batch_wrapper: &RecordBatchWrapper) -> Vec<u8> {
    let options = IpcWriteOptions::default();
    let generator = IpcDataGenerator::default();
    let mut dictionary_tracer = DictionaryTracker::new(false);

    // Encode the Arrow RecordBatch, including any dictionaries
    let (dictionaries, batch_data) = generator
        .encoded_batch(&record_batch_wrapper.data, &mut dictionary_tracer, &options)
        .expect("Failed to encode RecordBatch");

    let mut buffer: Vec<u8> = Vec::new();

    // Append dictionary messages and data
    for dict in dictionaries {
        buffer.extend_from_slice(&dict.ipc_message);
        buffer.extend_from_slice(&dict.arrow_data);
    }

    // Append batch message and data
    buffer.extend_from_slice(&batch_data.ipc_message);
    buffer.extend_from_slice(&batch_data.arrow_data);

    buffer
}

pub fn serialize_record_batch_full_ipc(record_batch: &RecordBatchWrapper) -> Vec<u8> {
    let mut buffer: Vec<u8> = Vec::new();
    let options = IpcWriteOptions::default();

    // Create a StreamWriter which automatically handles writing the schema header
    // and subsequent data/dictionary messages.
    let mut writer = StreamWriter::try_new(&mut buffer, &record_batch.data.schema())
        .expect("Failed to create IPC stream writer");

    // Write the record batch to the stream
    writer
        .write(&record_batch.data)
        .expect("Failed to write RecordBatch to IPC stream");

    // Finish the stream, ensuring all buffered data is written
    writer.finish().expect("Failed to finish IPC stream writer");

    buffer
}

/// Returns the current Unix timestamp in microseconds, Coordinated Universal Time (UTC).
fn current_timestamp_micros_utc() -> u64 {
    // `timestamp_micros()` returns i64, cast to u64 as per Flatbuffers schema.
    Utc::now().timestamp_micros() as u64
}

/// Builds a FlatbufMeta buffer with LogMeta as its primary metadata content.
///
/// This function constructs a Flatbuffers `FlatbufMeta` object, embedding
/// `LogMeta` details within its union field.
///
/// # Arguments
/// * `metadata` - A reference to the application-specific `Metadata` struct
///                containing flight and service IDs.
///
/// # Returns
/// A `Vec<u8>` containing the serialized FlatbufMeta.
pub fn build_flatbufmeta_with_logmeta<'a>(metadata: &Metadata) -> Vec<u8> {
    let mut builder = FlatBufferBuilder::new();

    // Create Flatbuffers strings for flight_id and service_id
    let flight_id_fb_str = Some(builder.create_string(&metadata.flight));
    let service_id_fb_str = Some(builder.create_string(&metadata.service_id));

    // Build the nested LogMeta table for the union
    let log_meta_offset = LogMeta::create(
        &mut builder,
        &LogMetaArgs {
            flight_id: flight_id_fb_str,
            schema_id: 1,       // Example: Fixed schema ID
            schema_hash: 123,   // Example: Fixed schema hash
            arrow_buffer_id: 1, // Example: Fixed Arrow buffer ID
            service_id: service_id_fb_str,
            partition_fields: None, // No partition fields for this example
        },
    );

    // Create the Flatbuffers vector for the magic number
    let magic_number_vector_offset = Some(builder.create_vector(b"WALMAGIC"));

    // Get the current timestamp in microseconds UTC
    let timestamp = current_timestamp_micros_utc();

    // Build the top-level FlatbufMeta table
    let flatbuf_meta_offset = FlatbufMeta::create(
        &mut builder,
        &FlatbufMetaArgs {
            version: 1,
            flags: 0,
            timestamp_unix_micros: timestamp,
            serialization: Serialization::Arrow, // Specify serialization format
            magic_number: magic_number_vector_offset,
            meta: Some(log_meta_offset.as_union_value()), // Set the actual LogMeta data
            meta_type: WalMetadataUnion::LogMeta,         // Set the type of metadata in the union
        },
    );

    // Finish the Flatbuffers buffer
    builder.finish(flatbuf_meta_offset, None);

    // Get the finished data as a Vec<u8>
    builder.finished_data().to_vec()
}
