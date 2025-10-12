// This actor reads Arrow IPC Wal files
// It then constructs a new Arrow buffer, that is partitioned based on the configuration

// TODO: Vectorized read of the Wal file. Also, read some fields from metadata for grouping.
// This should be done on a mmap file and the metadata should be stored in Flatbuf.

use crate::application::actors::broadcaster::broadcast_actor::RecordBatchWrapper;
pub(crate) use crate::application::actors::iceberg::handler::buffer_manager::GetBuffer;
use crate::config::yaml_reader::Storage;
use crate::core::buffer::full_drain::FullDrain;
use crate::core::buffer::manager::BufferManager;
use crate::core::catalog::rest_catalog_factory::create_rest_catalog;
use crate::core::error::exception::actor_errors::ActorError;
use crate::core::error::exception::buffer_error::BufferError;
use crate::platform::registry::{FetchIcebergActor, Registry};
#[cfg(test)]
use actix::Context;
use actix::{Actor, Addr, AsyncContext, MailboxError, Message};
use arrow_schema::{DataType, Field, Schema};
use iceberg_catalog_rest::RestCatalog;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::task::JoinHandle;

#[derive(Clone, Message, Debug)]
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
    ) -> Result<Vec<RecordBatchWrapper>, BufferError> {
        match self {
            IcebergActorAddr::Real(iceberg_actor) => iceberg_actor
                .send(GetBuffer::new(flight_name.clone()))
                .await
                .map_err(|_| BufferError::BufferMissing(flight_name))?,
            _ => {
                panic!("Mock not implemented")
            }
        }
    }
}

#[derive(Clone)]
pub struct IcebergActor {
    pub(crate) catalog: Arc<RestCatalog>,
    pub(crate) namespace: String,
    registry_address: Addr<Registry>,
    pub(crate) buffer_manager: Arc<BufferManager>,
}

impl IcebergActor {
    pub fn new(
        registry_address: Addr<Registry>,
        object_storage_properties: Storage,
        namespace: String,
    ) -> JoinHandle<IcebergActor> {
        tokio::spawn(async move {
            let catalog = Arc::new(create_rest_catalog(object_storage_properties).await);
            IcebergActor {
                catalog,
                namespace,
                registry_address,
                buffer_manager: Arc::new(BufferManager::new(Arc::new(FullDrain {}))),
            }
        })
    }
}

use crate::application::actors::iceberg::handler::create_table::CreateTable;
impl Actor for IcebergActor {
    type Context = actix::Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        let address = ctx.address();
        let registry_address = self.registry_address.clone();
        tokio::spawn(async move {
            registry_address.do_send(IcebergActorAddr::Real(address));
            match registry_address.send(FetchIcebergActor).await {
                Ok(iceberg) => {
                    if let Ok(iceberg_actor) = iceberg {
                        match iceberg_actor {
                            IcebergActorAddr::Real(iceberg_actor) => {
                                iceberg_actor.do_send(CreateTable {
                                    table: "metric".to_string(),
                                    schema: Arc::new(metric_schema()),
                                    _partition_fields: vec!["".to_string()],
                                });
                            }
                            _ => {
                                panic!("Mock not implemented")
                            }
                        }
                    } else {
                        panic!("Iceberg actor not found")
                    };
                }
                _ => {}
            };
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

pub fn metric_schema() -> Schema {
    let mut fields = Vec::new();

    fields.push(Field::new("id", DataType::Int64, false).with_metadata({
        let mut m = HashMap::new();
        m.insert("PARQUET:field_id".to_string(), "1".to_string());
        m
    }));

    fields.push(Field::new("name", DataType::Utf8, false).with_metadata({
        let mut m = HashMap::new();
        m.insert("PARQUET:field_id".to_string(), "2".to_string());
        m
    }));

    fields.push(
        Field::new("active", DataType::Boolean, false).with_metadata({
            let mut m = HashMap::new();
            m.insert("PARQUET:field_id".to_string(), "3".to_string());
            m
        }),
    );

    fields.push(Field::new("binary", DataType::Int64, false).with_metadata({
        let mut m = HashMap::new();
        m.insert("PARQUET:field_id".to_string(), "4".to_string());
        m
    }));

    Schema::new(fields)
}
