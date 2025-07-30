// This actor reads Arrow IPC Wal files
// It then constructs a new Arrow buffer, that is partitioned based on the configuration

// TODO: Vectorized read of the Wal file. Also, read some fields from metadata for grouping.
// This should be done on a mmap file and the metadata should be stored in Flatbuf.

use crate::application::actors::broadcast::RecordBatchWrapper;
use crate::core::utils::iceberg::convert_arrow_to_iceberg_schema;
use crate::platform::registry::Registry;
#[cfg(test)]
use actix::Context;
use actix::{Actor, Addr, AsyncContext, Handler, Message};
use arrow::compute::concat_batches;
use arrow_array::RecordBatch;
use iceberg::spec::Schema;
use iceberg::transaction::Transaction;
use iceberg::writer::base_writer::data_file_writer::DataFileWriterBuilder;
use iceberg::writer::file_writer::ParquetWriterBuilder;
use iceberg::writer::file_writer::location_generator::{
    DefaultFileNameGenerator, DefaultLocationGenerator,
};
use iceberg::writer::{IcebergWriter, IcebergWriterBuilder};
use iceberg::{Catalog, NamespaceIdent, TableCreation, TableIdent};
use iceberg_catalog_rest::{RestCatalog, RestCatalogConfig};
use parquet::file::properties::WriterProperties;
use std::collections::HashMap;
use std::sync::Arc;
use log::error;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use tracing::{info, warn};
use uuid::Uuid;

#[derive(Clone, Message)]
#[rtype(result = "()")]
pub enum IcebergActorAddr {
    Real(Addr<IcebergActor>),
    #[cfg(test)]
    Mock(Addr<MockIcebergActor>),
    Empty,
}

#[derive(Clone)]
pub struct IcebergActor {
    catalog: Arc<Mutex<RestCatalog>>,
    registry_address: Addr<Registry>,
    buffer: Arc<Mutex<Vec<RecordBatchWrapper>>>,
}

pub async fn fetch_catalog() -> RestCatalog {
    let mut props = HashMap::new();
    props.insert("aws.region".to_string(), "us-east-1".to_string());
    props.insert("aws.endpoint".to_string(), "http://minio:9000".to_string());
    props.insert(
        "aws.access_key_id".to_string(),
        "minio-root-user".to_string(),
    );
    props.insert(
        "aws.secret_access_key".to_string(),
        "minio-root-password".to_string(),
    );
    props.insert("path-style-access".to_string(), "false".to_string());

    let config = RestCatalogConfig::builder()
        .uri("http://127.0.0.1:8181/catalog".to_string())
        .warehouse("log".to_string())
        .props(props)
        .build();
    let catalog = RestCatalog::new(config);
    catalog
}

impl IcebergActor {
    pub fn new(&mut self, registry_address: Addr<Registry>) -> JoinHandle<IcebergActor> {
        actix_rt::spawn(async move {
            let catalog = Arc::new(Mutex::new(fetch_catalog().await));
            IcebergActor {
                catalog,
                registry_address,
                buffer: Arc::new(Mutex::new(vec![])),
            }
        })
    }

    pub fn default(registry_address: Addr<Registry>) -> JoinHandle<IcebergActor> {
        actix_rt::spawn(async move {
            let catalog = Arc::new(Mutex::new(fetch_catalog().await));
            IcebergActor {
                catalog,
                registry_address,
                buffer: Arc::new(Mutex::new(vec![])),
            }
        })
    }

    pub fn write(&self, _data: &[u8]) {
        // Here we would write the data to the WAL file
        // For now, we just print the data
    }
}

impl Actor for IcebergActor {
    type Context = actix::Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        let address = ctx.address();
        let registry_address = self.registry_address.clone();
        actix_rt::spawn(async move {
            registry_address.do_send(IcebergActorAddr::Real(address));
        });
        println!("Started IcebergActor");
    }
}

impl Handler<RecordBatchWrapper> for IcebergActor {
    type Result = ();
    fn handle(&mut self, msg: RecordBatchWrapper, _ctx: &mut Self::Context) -> Self::Result {
        let buffer = self.buffer.clone();
        actix_rt::spawn(async move {
            buffer.lock().await.push(msg);
        });
    }
}

pub struct FlushInstruction;

impl Message for FlushInstruction {
    type Result = Result<(), String>;
}

// Trigger a flush to the iceberg table
impl Handler<FlushInstruction> for IcebergActor {
    type Result = Result<(), String>;

    fn handle(&mut self, _msg: FlushInstruction, _ctx: &mut Self::Context) -> Self::Result {
        println!("Received FlushInstruction. Attempting to flush buffered data.");
        let catalog_arc = self.catalog.clone();
        let buffer_arc = self.buffer.clone();

        actix_rt::spawn(async move {
            let mut buffer_guard = buffer_arc.lock().await;
            if buffer_guard.is_empty() {
                println!("Buffer is empty, nothing to flush.");
                return;
            }

            // Take all RecordBatches from the buffer
            let batches_to_flush: Vec<Arc<RecordBatch>> = buffer_guard
                .drain(..) // Drains the buffer, leaving it empty
                .map(|wrapper| wrapper.data)
                .collect();

            // If the buffer is empty after draining, handle it
            if batches_to_flush.is_empty() {
                println!("Buffer was emptied by drain, nothing to concatenate.");
                return;
            }

            // Assuming all batches have the same schema for concatenation
            let first_schema = batches_to_flush[0].schema();
            let raw_batches_refs: Vec<&RecordBatch> = batches_to_flush
                .iter()
                .map(|arc_batch| &**arc_batch)
                .collect();

            let merged_batch = match concat_batches(&first_schema, raw_batches_refs) {
                Ok(batch) => Arc::new(batch),
                Err(e) => {
                    eprintln!("Failed to concatenate RecordBatches: {:?}", e);
                    return;
                }
            };

            println!("Merged RecordBatch Schema (Arrow): {:?}", merged_batch.schema());
            // For more detailed inspection:
            for (i, field) in merged_batch.schema().fields().iter().enumerate() {
                println!("Arrow Field: Index={}, Name='{}', DataType={:?}", i, field.name(), field.data_type());
            }

            // Assuming a fixed table name for flushing, or you could derive it from metadata
            let table_name = "test_table".to_string(); // You might want to get this dynamically
            let table_ident =
                TableIdent::from_strs(vec!["log".to_string(), table_name.clone()]).unwrap();

            // Acquire catalog lock only for load_table, then drop
            println!("Waiting for catalog lock...");
            let table_result = {
                let catalog_guard = catalog_arc.lock().await;
                println!("Acquired catalog lock.");
                catalog_guard.load_table(&table_ident).await
            };

            let table = match table_result {
                Ok(t) => t,
                Err(e) => {
                    eprintln!("Failed to load table {} for flush: {:?}", table_name, e);
                    return;
                }
            };

            // Prepare writer and write the batch
            let file_io = table.file_io().clone();
            let current_schema = table.metadata().current_schema().clone();

            let location_generator =
                DefaultLocationGenerator::new(table.metadata().clone()).unwrap();
            let file_name_generator = DefaultFileNameGenerator::new(
                "data".to_string() + Uuid::now_v7().to_string().as_str(),
                None,
                iceberg::spec::DataFileFormat::Parquet,
            );

            let parquet_writer_builder = ParquetWriterBuilder::new(
                WriterProperties::default(),
                current_schema,
                file_io,
                location_generator,
                file_name_generator,
            );
            let iceberg_schema: &Schema = table.metadata().current_schema();
            println!("Iceberg Table Schema: {:?}", iceberg_schema);
            let data_file_writer_builder =
                DataFileWriterBuilder::new(parquet_writer_builder, None, 0);

            let mut data_file_writer = match data_file_writer_builder.build().await {
                Ok(writer) => writer,
                Err(e) => {
                    eprintln!("Failed to build DataFileWriter: {:?}", e);
                    return;
                }
            };

            if let Err(e) = data_file_writer.write((*merged_batch).clone()).await {
                eprintln!("Failed to write merged data to file: {:?}", e);
                return;
            }
            println!(
                "Data written to Parquet file(s) in S3. Now closing writer and getting file metadata..."
            );

            let data_files = match data_file_writer.close().await {
                Ok(files) => files,
                Err(e) => {
                    eprintln!("Failed to close DataFileWriter and get data files: {:?}", e);
                    return;
                }
            };

            println!("DataFile metadata obtained. Starting table transaction...");

            // Create a new transaction on the table
            let tx = Transaction::new(&table);

            let commit_id = Some(Uuid::now_v7());
            let key_metadata = vec![];

            let mut fast_append_action = tx.fast_append(commit_id, key_metadata).unwrap();

            // Append data files
            fast_append_action.add_data_files(data_files).unwrap();

            let updated_tx = fast_append_action.apply().await.unwrap();

            let catalog_guard = catalog_arc.lock().await;
            let committed_table = updated_tx.commit(&*catalog_guard).await.unwrap();
            drop(catalog_guard);
        });
        Ok(())
    }
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct CreateTable {
    pub(crate) table: String,
    pub(crate) schema: Arc<arrow_schema::Schema>,
    pub(crate) partition_fields: Vec<String>,
}

impl Handler<CreateTable> for IcebergActor {
    type Result = ();

    fn handle(&mut self, msg: CreateTable, _ctx: &mut Self::Context) -> Self::Result {
        let catalog = self.catalog.clone();
        actix_rt::spawn(async move {
            let namespace_ident = NamespaceIdent::from_vec(vec!["log".to_string()]).unwrap();
            let namespace_exists = {
                let guard = catalog.lock().await;
                guard
                    .namespace_exists(&namespace_ident)
                    .await
                    .unwrap_or(false)
            };

            if namespace_exists {
                info!("Namespace already exists. Skipping creation.");

                let tables = {
                    let guard = catalog.lock().await;
                    guard.list_tables(&namespace_ident).await
                };
                match tables {
                    Ok(table_idents) => {
                        info!("Tables in namespace: {:?}", table_idents);
                    },
                    Err(e) => {
                        eprintln!("Failed to list tables in namespace {:?}: {:?}", namespace_ident, e);
                    }
                }
            } else {
                let guard = catalog.lock().await;
                if let Err(e) = guard
                    .create_namespace(
                        &namespace_ident,
                        HashMap::from([("key1".to_string(), "value1".to_string())]), // TODO: Add valid and useful properties for namespace
                    ).await
                {
                    eprintln!("Failed to create namespace {:?}: {:?}", namespace_ident, e);
                    return;
                }
                println!("Namespace {:?} created!", namespace_ident);
            }

            let table_schema = convert_arrow_to_iceberg_schema(&msg.schema);

            let table_build = TableCreation::builder()
                .name(msg.table.clone())
                .schema(table_schema)
                .build();

            // Create table holding lock for just the create call
            {
                let guard = catalog.lock().await;
                match guard.create_table(&namespace_ident, table_build).await {
                    Ok(_) => println!("Table created"),
                    Err(err) => error!("Failed to create table: {:?}", err),
                }
            }
        });
    }
}

#[cfg(test)]
#[derive(Clone)]
pub struct MockIcebergActor {
    pub(crate) registry_address: Addr<Registry>,
}

#[cfg(test)]
impl Actor for MockIcebergActor {
    type Context = Context<Self>;
}

#[cfg(test)]
impl Handler<FlushInstruction> for MockIcebergActor {
    type Result = Result<(), String>;

    fn handle(&mut self, _msg: FlushInstruction, _ctx: &mut Self::Context) -> Self::Result {
        todo!()
    }
}
