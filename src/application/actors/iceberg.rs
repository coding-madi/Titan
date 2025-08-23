// This actor reads Arrow IPC Wal files
// It then constructs a new Arrow buffer, that is partitioned based on the configuration

// TODO: Vectorized read of the Wal file. Also, read some fields from metadata for grouping.
// This should be done on a mmap file and the metadata should be stored in Flatbuf.

use crate::application::actors::broadcast::RecordBatchWrapper;
use crate::config::yaml_reader::{ObjectStorage, S3Properties, Storage};
use crate::core::buffer::full_drain::FullDrain;
use crate::core::buffer::manager::BufferManager;
use crate::core::catalog::iceberg_operations::{
    create_parquet_writer, flush_buffer, load_table, write_and_close,
};
use crate::core::catalog::rest_catalog_factory::create_rest_catalog;
use crate::core::error::exception::iceberg_error::IcebergError;
use crate::core::utils::iceberg::convert_arrow_to_iceberg_schema;
use crate::platform::registry::Registry;
#[cfg(test)]
use actix::Context;
use actix::{Actor, Addr, AsyncContext, Handler, Message, ResponseFuture};
use iceberg::writer::file_writer::location_generator::DefaultLocationGenerator;
use iceberg::{Catalog, NamespaceIdent, TableCreation};
use iceberg_catalog_rest::RestCatalog;
use log::error;
use std::collections::HashMap;
use std::fmt::Error;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use tracing::info;

#[derive(Clone, Message)]
#[rtype(result = "()")]
pub enum IcebergActorAddr {
    Real(Addr<IcebergActor>),
    #[cfg(test)]
    Mock(Addr<MockIcebergActor>),
    Empty,
}

impl IcebergActorAddr {
    pub async fn get_buffer(
        &self,
        flight_name: String,
    ) -> Result<Vec<RecordBatchWrapper>, std::fmt::Error> {
        match self {
            IcebergActorAddr::Real(iceberg_actor) => iceberg_actor
                .send(GetBuffer {
                    stream: flight_name,
                })
                .await
                .unwrap(),
            _ => {
                panic!("Mock not implemented")
            }
        }
    }
}

#[derive(Clone)]
pub struct IcebergActor {
    catalog: Arc<Mutex<RestCatalog>>,
    namespace: String,
    registry_address: Addr<Registry>,
    buffer_manager: Arc<BufferManager>,
}

impl Actor for IcebergActor {
    type Context = actix::Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        let address = ctx.address();
        let registry_address = self.registry_address.clone();
        tokio::spawn(async move {
            registry_address.do_send(IcebergActorAddr::Real(address));
        });
        println!("Started IcebergActor");
    }
}

/// This handler will send the data to the buffer manager over tokio async channels.
/// This enabled us to have back-pressuring per flight.
impl Handler<RecordBatchWrapper> for IcebergActor {
    type Result = ();

    fn handle(&mut self, msg: RecordBatchWrapper, _ctx: &mut Self::Context) {
        let buffer_manager = self.buffer_manager.clone(); // Arc<BufferManager>

        tokio::spawn(async move {
            let flight = msg.metadata.flight.clone();

            // Get or create a channel for this flight
            let sender = buffer_manager.get_or_create_sender(&*flight);
            // send async over tokio channels
            let _ = sender.send(msg).await;
        });
    }
}

/// This flush is called by the WAL actor. A flush is triggered in every log rotation
#[derive(Message)]
#[rtype(result = "Result<(), IcebergError>")]
pub struct FlushInstruction;

impl Handler<FlushInstruction> for IcebergActor {
    type Result = Result<(), IcebergError>;

    // TODO - Handle errors and retries
    // TODO - Send errors to alerting system
    fn handle(&mut self, _msg: FlushInstruction, _ctx: &mut Self::Context) -> Self::Result {
        info!("Received FlushInstruction. Attempting to flush buffered data.");
        let buffer_manager = self.buffer_manager.clone();
        let catalog = self.catalog.clone();
        let namespace = self.namespace.clone();
        // Fire-and-forget async flush
        tokio::spawn(async move {
            if let Err(e) =
                flush_buffer(buffer_manager.clone(), catalog.clone(), namespace.clone()).await
            {
                // You can log or send to an error-reporting actor here
                error!("Flush failed: {:?}", e);
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
        tokio::spawn(async move {
            let catalog = Arc::new(Mutex::new(
                create_rest_catalog(object_storage_properties).await,
            ));
            IcebergActor {
                catalog,
                namespace,
                registry_address,
                buffer_manager: Arc::new(BufferManager::new(Arc::new(FullDrain {}))),
            }
        })
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
        tokio::spawn(async move {
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
        let buffer_manager = self.buffer_manager.clone();

        Box::pin(async move {
            buffer_manager
                .get(&msg.stream)
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
    type Result = Result<(), IcebergError>;

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
