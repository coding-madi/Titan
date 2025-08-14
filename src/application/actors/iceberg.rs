// This actor reads Arrow IPC Wal files
// It then constructs a new Arrow buffer, that is partitioned based on the configuration

// TODO: Vectorized read of the Wal file. Also, read some fields from metadata for grouping.
// This should be done on a mmap file and the metadata should be stored in Flatbuf.

use crate::application::actors::broadcast::RecordBatchWrapper;
use crate::config::yaml_reader::{GCSProperties, ObjectStorage, S3Properties, Storage};
use crate::core::utils::iceberg::convert_arrow_to_iceberg_schema;
use crate::platform::registry::Registry;
#[cfg(test)]
use actix::Context;
use actix::{Actor, Addr, AsyncContext, Handler, Message, ResponseFuture};
use arrow::compute::concat_batches;
use arrow_array::RecordBatch;
use iceberg::spec::DataFile;
use iceberg::table::Table;
use iceberg::transaction::Transaction;
use iceberg::writer::base_writer::data_file_writer::DataFileWriterBuilder;
use iceberg::writer::file_writer::ParquetWriterBuilder;
use iceberg::writer::file_writer::location_generator::{
    DefaultFileNameGenerator, DefaultLocationGenerator,
};
use iceberg::writer::{IcebergWriter, IcebergWriterBuilder};
use iceberg::{Catalog, NamespaceIdent, TableCreation, TableIdent};
use iceberg_catalog_rest::{RestCatalog, RestCatalogConfig};
use log::{error, warn};
use parquet::file::properties::WriterProperties;
use std::collections::HashMap;
use std::fmt::Error;
use std::ops::Deref;
use std::pin::Pin;
use std::sync::{Arc, mpsc};
use std::thread::spawn;
use tokio::sync::Mutex;
use tokio::sync::mpsc::{Receiver, Sender};
use tokio::task::JoinHandle;
use tracing::info;
use uuid::Uuid;

#[derive(Clone, Message)]
#[rtype(result = "()")]
pub enum IcebergActorAddr {
    Real(Addr<IcebergActor>),
    #[cfg(test)]
    Mock(Addr<MockIcebergActor>),
    Empty,
}

// type BufferMap = Mutex<HashMap<String, Arc<Mutex<Vec<RecordBatchWrapper>>>>>;
use dashmap::DashMap;
type BufferMap = DashMap<String, Vec<RecordBatchWrapper>>;

fn s3_props(properties: S3Properties) -> HashMap<String, String> {
    HashMap::from([
        ("aws.region", properties.aws_region),
        ("aws.endpoint", properties.aws_endpoint),
        ("aws.access_key_id", properties.aws_access_key_id),
        ("aws.secret_access_key", properties.aws_secret_access_key),
        (
            "path-style-access",
            properties.path_style_access.to_string(),
        ),
    ])
    .into_iter()
    .map(|(k, v)| (k.to_string(), v.to_string()))
    .collect()
}

fn gcs_props(_properties: GCSProperties) -> HashMap<String, String> {
    HashMap::new()
}

fn catalog_config(
    properties: HashMap<String, String>,
    warehouse_name: String,
) -> RestCatalogConfig {
    let endpoint = properties
        .get("aws.endpoint")
        .expect("Missing endpoint property")
        .clone();
    RestCatalogConfig::builder()
        .uri(endpoint)
        .warehouse(warehouse_name)
        .props(properties)
        .build()
}

pub async fn fetch_catalog(storage: Storage) -> RestCatalog {
    let object_storage = storage.object_storage;
    let properties: HashMap<String, String> = match object_storage {
        ObjectStorage::S3(s3_properties) => s3_props(s3_properties),
        ObjectStorage::GCS(gcs_properties) => gcs_props(gcs_properties),
    };

    let catalog = catalog_config(properties, storage.warehouse);
    RestCatalog::new(catalog)
}

type BatchSender = tokio::sync::mpsc::UnboundedSender<RecordBatchWrapper>;
type BatchReceiver = tokio::sync::mpsc::UnboundedReceiver<RecordBatchWrapper>;
#[derive(Clone)]
pub struct IcebergActor {
    catalog: Arc<Mutex<RestCatalog>>,
    namespace: String,
    registry_address: Addr<Registry>,
    channels: Arc<DashMap<String, Sender<RecordBatchWrapper>>>,
    buffer: Arc<DashMap<String, Vec<RecordBatchWrapper>>>,
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

    fn handle(&mut self, msg: RecordBatchWrapper, _ctx: &mut Self::Context) {
        let channel = self.channels.clone();
        let this = self.clone();
        actix_rt::spawn(async move {
            let flight = msg.metadata.flight.clone();

            let sender = channel
                .entry(flight.clone())
                .or_insert_with(|| {
                    // create channel + spawn consumer bound to this flight
                    let (tx, rx): (Sender<RecordBatchWrapper>, Receiver<RecordBatchWrapper>) =
                        tokio::sync::mpsc::channel(1024);
                    this.spawn_stream_consumer(flight.clone(), rx);
                    tx
                })
                .clone();

            let _ = sender.send(msg).await;
        });
    }
}

pub struct FlushInstruction;

impl Message for FlushInstruction {
    type Result = Result<(), String>;
}

impl Handler<FlushInstruction> for IcebergActor {
    type Result = Result<(), String>;

    fn handle(&mut self, _msg: FlushInstruction, _ctx: &mut Self::Context) -> Self::Result {
        let this = self.clone();
        println!("Received FlushInstruction. Attempting to flush buffered data.");
        actix_rt::spawn(async move {
            if let Err(e) = this.flush_buffer().await {
                eprintln!("Flush failed: {}", e);
            }
        });
        Ok(())
    }
}

impl IcebergActor {
    pub fn new(
        registry_address: Addr<Registry>,
        object_storage_properties: Storage,
        namespace: String,
    ) -> JoinHandle<IcebergActor> {
        actix_rt::spawn(async move {
            let catalog = Arc::new(Mutex::new(fetch_catalog(object_storage_properties).await));
            IcebergActor {
                catalog,
                namespace,
                registry_address,
                channels: Arc::new(DashMap::new()),
                buffer: Arc::new(DashMap::new()),
            }
        })
    }

    fn spawn_stream_consumer(&self, stream: String, mut rx: Receiver<RecordBatchWrapper>) {
        let mut batch_buf: Arc<DashMap<String, Vec<RecordBatchWrapper>>> = self.buffer.clone();
        tokio::spawn(async move {
            while let Some(batch) = rx.recv().await {
                batch_buf.entry(stream.clone()).or_default().push(batch);
            }
        });
    }

    async fn is_buffer_empty(&self) -> bool {
        self.buffer.is_empty()
    }

    pub async fn concat_batches_grouped(
        &self,
        batches: &Vec<RecordBatchWrapper>,
    ) -> Result<HashMap<String, Arc<RecordBatch>>, String> {
        if batches.is_empty() {
            return Err("No batches to concat".into());
        }

        // Step 1: Group by flight metadata
        let mut groups: HashMap<String, Vec<&RecordBatchWrapper>> = HashMap::new();
        for b in batches {
            let key = b.metadata.flight.clone();
            groups.entry(key).or_default().push(b);
        }

        // Step 2: Concatenate per group
        let mut results = HashMap::new();
        for (flight, group_batches) in groups {
            let schema = group_batches[0].data.schema();
            let refs = group_batches.iter().map(|b| &*b.data).collect::<Vec<_>>();

            let concatenated = concat_batches(&schema, refs)
                .map(Arc::new)
                .map_err(|e| format!("Failed to concatenate for flight {}: {:?}", flight, e))?;

            results.insert(flight, concatenated);
        }

        Ok(results)
    }

    fn print_schema_info(&self, batch: &RecordBatch) {
        println!("Merged RecordBatch Schema (Arrow): {:?}", batch.schema());
        for (i, field) in batch.schema().fields().iter().enumerate() {
            println!(
                "Arrow Field: Index={}, Name='{}', DataType={:?}",
                i,
                field.name(),
                field.data_type()
            );
        }
    }

    fn make_table_ident(&self, table: String) -> Result<TableIdent, String> {
        TableIdent::from_strs(vec!["log", &table.clone()])
            .map_err(|e| format!("Failed to create TableIdent: {:?}", e))
    }

    async fn load_table(&self, table_ident: &TableIdent) -> Result<Table, String> {
        let catalog = self.catalog.lock().await;
        match catalog.load_table(table_ident).await {
            Ok(table) => {
                println!("Table loaded successfully: {:?}", table_ident);
                Ok(table)
            }
            Err(e) => {
                println!("Failed to load table {:?}: {:?}", table_ident, e);
                Err(format!("Failed to load table for flush: {:?}", e))
            }
        }
    }

    fn create_parquet_writer(
        &self,
        table: &Table,
    ) -> Result<
        DataFileWriterBuilder<
            ParquetWriterBuilder<DefaultLocationGenerator, DefaultFileNameGenerator>,
        >,
        String,
    > {
        let file_io = table.file_io().clone();
        let schema = table.metadata().current_schema().clone();

        let location_gen = DefaultLocationGenerator::new(table.metadata().clone())
            .map_err(|e| format!("Failed to create location generator: {:?}", e))?;

        let file_name_gen = DefaultFileNameGenerator::new(
            format!("data{}", Uuid::now_v7()),
            None,
            iceberg::spec::DataFileFormat::Parquet,
        );

        let parquet_writer_builder = ParquetWriterBuilder::new(
            WriterProperties::default(),
            schema,
            file_io,
            location_gen,
            file_name_gen,
        );

        Ok(DataFileWriterBuilder::new(parquet_writer_builder, None, 0))
    }

    async fn write_and_close(
        &self,
        data_file_writer_builder: DataFileWriterBuilder<
            ParquetWriterBuilder<DefaultLocationGenerator, DefaultFileNameGenerator>,
        >,
        batch: Arc<RecordBatch>,
    ) -> Result<Vec<DataFile>, String> {
        let mut writer = data_file_writer_builder
            .build()
            .await
            .map_err(|e| format!("Failed to build DataFileWriter: {:?}", e))?;

        writer
            .write((*batch).clone())
            .await
            .map_err(|e| format!("Failed to write merged data to file: {:?}", e))?;

        writer
            .close()
            .await
            .map_err(|e| format!("Failed to close DataFileWriter: {:?}", e))
    }

    async fn commit_transaction(
        &self,
        table: &Table,
        data_files: Vec<DataFile>,
    ) -> Result<Table, String> {
        let tx = Transaction::new(table);

        let commit_id = Some(Uuid::now_v7());
        let key_metadata = vec![];

        let mut fast_append = tx
            .fast_append(commit_id, key_metadata)
            .map_err(|e| format!("Failed to create fast append: {:?}", e))?;

        fast_append
            .add_data_files(data_files)
            .map_err(|e| format!("Failed to add data files: {:?}", e))?;

        let updated_tx = fast_append
            .apply()
            .await
            .map_err(|e| format!("Failed to apply transaction: {:?}", e))?;

        let catalog = self.catalog.lock().await;
        updated_tx
            .commit(&*catalog)
            .await
            .map_err(|e| format!("Failed to commit transaction: {:?}", e))
    }

    pub async fn flush_buffer(&self) -> Result<(), String> {
        let mut all_batches: Vec<RecordBatchWrapper> = Vec::new();

        // Drain all batches from the buffer, not the channels
        let keys: Vec<String> = self.buffer.iter().map(|e| e.key().clone()).collect();
        for key in keys {
            if let Some((_, mut batches)) = self.buffer.remove(&key) {
                all_batches.append(&mut batches);
            }
        }

        if all_batches.is_empty() {
            warn!("Flush called but buffer empty");
            return Ok(());
        }

        let grouped = self.concat_batches_grouped(&all_batches).await?;
        // Continue with writing grouped batches to Iceberg

        // Ensure namespace exists
        {
            let catalog = self.catalog.lock().await;
            let namespace_ident = NamespaceIdent::from_vec(vec![self.namespace.clone()]).unwrap();
            if !catalog
                .namespace_exists(&namespace_ident)
                .await
                .unwrap_or(false)
            {
                catalog
                    .create_namespace(&namespace_ident, HashMap::new())
                    .await
                    .map_err(|e| format!("Failed to create namespace: {:?}", e))?;
                info!("Namespace {} created", self.namespace);
            }
        }

        let tasks: Vec<_> = grouped
            .into_iter()
            .map(|(flight, batch)| {
                let this = self.clone();
                let this2 = self.clone();
                let this3 = self.clone();
                let namespace = self.namespace.clone();
                let catalog = self.catalog.clone();
                tokio::spawn(async move {
                    let table_ident = this.make_table_ident(flight.clone())?;
                    let table = match this2.load_table(&table_ident).await {
                        Ok(t) => t,
                        Err(_) => {
                            // Table missing → create it
                            let arrow_schema = batch.schema().deref().clone();
                            let iceberg_schema =
                                convert_arrow_to_iceberg_schema(&Arc::new(arrow_schema));
                            let table_build = TableCreation::builder()
                                .name(flight.clone())
                                .schema(iceberg_schema)
                                .build();
                            let namespace_ident =
                                NamespaceIdent::from_vec(vec![namespace]).unwrap();
                            catalog
                                .lock()
                                .await
                                .create_table(&namespace_ident, table_build)
                                .await
                                .map_err(|e| format!("Failed to create table: {:?}", e))?;
                            this3.load_table(&table_ident).await?
                        }
                    };
                    let writer_builder = this.create_parquet_writer(&table)?;
                    let data_files = tokio::task::spawn_blocking(move || {
                        futures::executor::block_on(this.write_and_close(writer_builder, batch))
                    })
                    .await
                    .unwrap();
                    // let data_files = this.write_and_close(writer_builder, batch).await?;
                    this2
                        .commit_transaction(&table, data_files.unwrap())
                        .await?;
                    Ok::<(), String>(())
                })
            })
            .collect();

        futures::future::try_join_all(tasks).await.unwrap();
        Ok(())
    }
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct CreateTable {
    pub(crate) table: String,
    pub(crate) schema: Arc<arrow_schema::Schema>,
    pub(crate) _partition_fields: Vec<String>,
}

impl Handler<CreateTable> for IcebergActor {
    type Result = ();

    fn handle(&mut self, msg: CreateTable, _ctx: &mut Self::Context) -> Self::Result {
        let catalog = self.catalog.clone();
        let namespace = self.namespace.clone();
        actix_rt::spawn(async move {
            let namespace_ident = NamespaceIdent::from_vec(vec![namespace]).unwrap();
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
                    }
                    Err(e) => {
                        eprintln!(
                            "Failed to list tables in namespace {:?}: {:?}",
                            namespace_ident, e
                        );
                    }
                }
            } else {
                let guard = catalog.lock().await;
                if let Err(e) = guard
                    .create_namespace(
                        &namespace_ident,
                        HashMap::from([("key1".to_string(), "value1".to_string())]), // TODO: Add valid and useful properties for namespace
                    )
                    .await
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

#[derive(Message)]
#[rtype(result = "Result<Vec<RecordBatchWrapper>, std::fmt::Error>")]
pub struct GetBuffer {
    pub stream: String,
}

impl Handler<GetBuffer> for IcebergActor {
    type Result = ResponseFuture<Result<Vec<RecordBatchWrapper>, std::fmt::Error>>;
    fn handle(&mut self, msg: GetBuffer, _ctx: &mut Self::Context) -> Self::Result {
        let buffer = self.buffer.clone(); // Arc<DashMap<String, Arc<Mutex<Vec<RecordBatchWrapper>>>>>

        Box::pin(async move {
            buffer
                .get(&msg.stream)
                .map(|entry| entry.clone()) // clone the Arc<Mutex<...>>
                .ok_or_else(|| Error::default())
        })
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

#[cfg(test)]
impl Handler<CreateTable> for MockIcebergActor {
    type Result = ();

    fn handle(&mut self, _msg: CreateTable, _ctx: &mut Self::Context) -> Self::Result {
        todo!()
    }
}

#[cfg(test)]
pub mod test {
    use crate::application::actors::iceberg::{fetch_catalog, s3_props};
    use crate::config::yaml_reader::{ObjectStorage, S3Properties, Storage};

    #[test]
    fn test_s3_props_mapping() {
        let props = S3Properties {
            aws_region: "us-east-1".to_string(),
            aws_endpoint: "http://localhost:9000".to_string(),
            aws_access_key_id: "minio".to_string(),
            aws_secret_access_key: "secret".to_string(),
            path_style_access: true,
        };

        let map = s3_props(props);

        assert_eq!(map.get("aws.region").unwrap(), "us-east-1");
        assert_eq!(map.get("aws.endpoint").unwrap(), "http://localhost:9000");
        assert_eq!(map.get("aws.access_key_id").unwrap(), "minio");
        assert_eq!(map.get("aws.secret_access_key").unwrap(), "secret");
        assert_eq!(map.get("path-style-access").unwrap(), "true");
    }

    #[tokio::test]
    async fn test_fetch_catalog_s3() {
        let storage = Storage {
            warehouse: "log".to_string(),
            namespace: "test".to_string(),
            object_storage: ObjectStorage::S3(S3Properties {
                aws_region: "us-east-1".to_string(),
                aws_endpoint: "http://localhost:9000".to_string(),
                aws_access_key_id: "minio".to_string(),
                aws_secret_access_key: "secret".to_string(),
                path_style_access: true,
            }),
        };

        let _catalog = fetch_catalog(storage).await;
        // You can add further assertions if needed.
    }
}
