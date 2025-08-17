use crate::core::buffer::manager::BufferManager;
use crate::core::catalog::iceberg_ddl::{create_namespace, create_table};
use crate::core::error::exception::iceberg_error::IcebergError;
use crate::core::utils::arrow::concat_batches_grouped;
use crate::core::utils::iceberg::{convert_arrow_to_iceberg_schema, make_table_ident};
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
use iceberg_catalog_rest::RestCatalog;
use parquet::file::properties::WriterProperties;
use std::collections::HashMap;
use std::ops::Deref;
use std::sync::Arc;
use log::warn;
use tokio::sync::Mutex;
use tracing::{error, info};
use uuid::Uuid;

pub async fn load_table(
    catalog: Arc<Mutex<RestCatalog>>,
    table_ident: &TableIdent,
) -> Result<Table, IcebergError> {
    let catalog = catalog.lock().await;
    match catalog.load_table(table_ident).await {
        Ok(table) => {
            info!("Table loaded successfully: {:?}", table_ident);
            Ok(table)
        }
        Err(e) => {
            error!("Failed to load table {:?}: {:?}", table_ident, e);
            return Err(IcebergError::TableError(format!(
                "Failed to load table for flush: {:?}",
                e
            )));
        }
    }
}

pub fn create_parquet_writer(
    table: &Table,
) -> Result<
    DataFileWriterBuilder<ParquetWriterBuilder<DefaultLocationGenerator, DefaultFileNameGenerator>>,
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

// Needs buffer manager
// restCatalog
// namespace
pub async fn flush_buffer(
    buffer_manager: Arc<BufferManager>,
    catalog: Arc<Mutex<RestCatalog>>,
    namespace: String,
) -> Result<(), IcebergError> {
    // Drain the data from memory
    let all_batches = buffer_manager.drain_all();
    // Group and concatenate for flushing
    let grouped = concat_batches_grouped(&all_batches).await.unwrap();

    if let Err(ns_error) = create_namespace(catalog.clone(), &namespace).await {
        error!("Fatal error in creating namespace: {}", ns_error);
        return Err(IcebergError::NamespaceError(ns_error.to_string()));
    }

    let tasks: Vec<_> = grouped
        .into_iter()
        .map(|(flight, batch)| {
            let catalog_clone = catalog.clone();
            let namespace_clone = namespace.clone();
            tokio::spawn(async move {
                let table_ident = make_table_ident(flight.clone())?;

                // Load table, if it fails create the table and try loading
                let table = match load_table(catalog_clone.clone(), &table_ident).await {
                    Ok(t) => t,
                    Err(_e) => {
                        warn!("Error in loading table - Attempting to re-create table - {}", _e.to_string());
                        let arrow_schema = batch.schema().deref().clone();
                        let _ = create_table(
                            catalog_clone.clone(),
                            &*namespace_clone,
                            &*flight,
                            Arc::new(arrow_schema),
                        )
                        .await?;
                        // Table missing → create it
                        load_table(catalog_clone.clone(), &table_ident).await?
                    }
                };
                let writer_builder = create_parquet_writer(&table)?;
                let data_files = tokio::task::spawn_blocking(move || {
                    futures::executor::block_on(write_and_close(writer_builder, batch))
                })
                .await
                .unwrap();

                commit_transaction(&table, data_files.unwrap(), catalog_clone)
                    .await
                    .map_err(|e| format!("Failed to commit transaction: {:?}", e))?;
                Ok::<(), IcebergError>(())
            })
        })
        .collect();

    futures::future::try_join_all(tasks).await.unwrap();
    Ok(())
}

pub async fn write_and_close(
    data_file_writer_builder: DataFileWriterBuilder<
        ParquetWriterBuilder<DefaultLocationGenerator, DefaultFileNameGenerator>,
    >,
    batch: Arc<RecordBatch>,
) -> Result<Vec<DataFile>, IcebergError> {
    let mut writer = data_file_writer_builder.build().await.map_err(|e| {
        IcebergError::StorageError(format!("Failed to build DataFileWriter: {:?}", e))
    })?;

    writer
        .write((*batch).clone())
        .await
        .map_err(|e| IcebergError::StorageError(format!("Failed to write batch: {:?}", e)))?;

    writer
        .close()
        .await
        .map_err(|e| IcebergError::StorageError(format!("Failed to close DataFileWriter: {:?}", e)))
}

pub async fn commit_transaction(
    table: &Table,
    data_files: Vec<DataFile>,
    catalog: Arc<Mutex<RestCatalog>>,
) -> Result<Table, IcebergError> {
    let tx = Transaction::new(table);
    let commit_id = Some(Uuid::now_v7());
    let key_metadata = vec![];

    let mut fast_append = tx.fast_append(commit_id, key_metadata).map_err(|e| {
        IcebergError::StorageError(format!("Failed to create fast append: {:?}", e))
    })?;

    fast_append
        .add_data_files(data_files)
        .map_err(|e| IcebergError::StorageError(format!("Failed to add data files: {:?}", e)))?;

    let updated_tx = fast_append
        .apply()
        .await
        .map_err(|e| IcebergError::StorageError(format!("Failed to apply transaction: {:?}", e)))?;

    let catalog = catalog.lock().await;
    updated_tx
        .commit(&*catalog)
        .await
        .map_err(|e| IcebergError::CommitError(format!("Failed to commit transaction: {:?}", e)))
}
