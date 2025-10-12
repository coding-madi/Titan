use crate::application::actors::iceberg::iceberg_actor::IcebergActor;
#[cfg(test)]
use crate::application::actors::iceberg::iceberg_actor::MockIcebergActor;
use crate::core::utils::iceberg::convert_arrow_to_iceberg_schema;
use actix::{Handler, Message};
use iceberg::{Catalog, NamespaceIdent, TableCreation};
use log::error;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::info;

#[derive(Message)]
#[rtype(result = "()")]
pub struct CreateTable {
    pub table: String,
    pub schema: Arc<arrow_schema::Schema>,
    pub _partition_fields: Vec<String>,
}

impl Handler<CreateTable> for IcebergActor {
    type Result = ();

    fn handle(&mut self, msg: CreateTable, _ctx: &mut Self::Context) -> Self::Result {
        let catalog = self.catalog.clone();
        let namespace = self.namespace.clone();
        tokio::spawn(async move {
            let namespace_ident = NamespaceIdent::from_vec(vec![namespace]).unwrap();
            let namespace_exists = {
                catalog
                    .namespace_exists(&namespace_ident)
                    .await
                    .unwrap_or(false)
            };

            if namespace_exists {
                info!("Namespace already exists. Skipping creation.");

                let tables = { catalog.list_tables(&namespace_ident).await };
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
                if let Err(e) = catalog
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
                match catalog.create_table(&namespace_ident, table_build).await {
                    Ok(_) => println!("Table created"),
                    Err(err) => error!("Failed to create table: {:?}", err),
                }
            }
        });
    }
}

#[cfg(test)]
impl Handler<CreateTable> for MockIcebergActor {
    type Result = ();

    fn handle(&mut self, _msg: CreateTable, _ctx: &mut Self::Context) -> Self::Result {
        todo!()
    }
}
