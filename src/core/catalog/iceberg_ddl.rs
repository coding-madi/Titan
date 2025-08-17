use crate::core::error::exception::iceberg_error::IcebergError;
use crate::core::utils::iceberg::convert_arrow_to_iceberg_schema;
use arrow_schema::Schema;
use iceberg::table::Table;
use iceberg::{Catalog, Namespace, NamespaceIdent, TableCreation, TableIdent};
use iceberg_catalog_rest::RestCatalog;
use std::sync::Arc;

// Load table
// Create table
pub async fn create_namespace(
    catalog: Arc<tokio::sync::Mutex<RestCatalog>>,
    namespace: &str,
) -> Result<Namespace, IcebergError> {
    let namespace_ident = NamespaceIdent::from_vec(vec![namespace.to_string()]).unwrap();
    let catalog_guard = catalog.lock().await;
    if catalog_guard
        .namespace_exists(&namespace_ident)
        .await
        .unwrap_or(false)
    {
        catalog_guard
            .get_namespace(&namespace_ident)
            .await
            .map_err(|e| IcebergError::StorageError(e.to_string()))
    } else {
        catalog_guard
            .create_namespace(&namespace_ident, Default::default())
            .await
            .map_err(|e| IcebergError::StorageError(e.to_string()))
    }
}

pub async fn create_table(
    catalog: Arc<tokio::sync::Mutex<RestCatalog>>,
    namespace: &str,
    table: &str,
    schema: Arc<Schema>,
) -> Result<Table, IcebergError> {
    let table_schema = convert_arrow_to_iceberg_schema(&schema);
    let table_build = TableCreation::builder()
        .name(table.to_string())
        .schema(table_schema)
        .build();
    let namespace_ident = NamespaceIdent::from_vec(vec![namespace.to_string()]).unwrap();

    let catalog_guard = catalog.lock().await;
    catalog_guard
        .create_table(&namespace_ident, table_build)
        .await
        .map_err(|e| IcebergError::StorageError(e.to_string()))
}

fn make_table_ident(table: String) -> Result<TableIdent, String> {
    TableIdent::from_strs(vec!["log", &table.clone()])
        .map_err(|e| format!("Failed to create TableIdent: {:?}", e))
}
