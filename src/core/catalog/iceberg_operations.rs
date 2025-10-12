use crate::core::buffer::manager::BufferManager;
use crate::core::catalog::iceberg_ddl::{create_namespace, create_table};
use crate::core::error::exception::iceberg_error::IcebergError;
use crate::core::utils::arrow::concat_batches_grouped;
use crate::core::utils::iceberg::make_table_ident;
use arrow_array::{Int64Array, RecordBatch, StringArray};
use iceberg::spec::DataFile;
use iceberg::table::Table;
use iceberg::transaction::Transaction;
use iceberg::writer::base_writer::data_file_writer::DataFileWriterBuilder;
use iceberg::writer::file_writer::ParquetWriterBuilder;
use iceberg::writer::file_writer::location_generator::{
    DefaultFileNameGenerator, DefaultLocationGenerator,
};
use iceberg::writer::{IcebergWriter, IcebergWriterBuilder};
use iceberg::{Catalog, TableIdent};
use iceberg_catalog_rest::RestCatalog;
use log::warn;
use parquet::file::properties::WriterProperties;
use std::ops::Deref;
use std::sync::Arc;
use tracing::{error, info};
use uuid::Uuid;

pub async fn load_table(
    catalog: Arc<RestCatalog>,
    table_ident: &TableIdent,
) -> Result<Table, IcebergError> {
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

pub async fn flush_buffer_to_table(
    buffer_manager: Arc<BufferManager>,
    catalog: Arc<RestCatalog>,
    namespace: String,
    table_name: Option<String>,
) -> Result<(), IcebergError> {
    let all_batches = buffer_manager.drain_all();
    let grouped = concat_batches_grouped(&all_batches).await.unwrap();

    if let Err(ns_error) = create_namespace(catalog.clone(), &namespace).await {
        error!("Fatal error in creating namespace: {}", ns_error);
        return Err(IcebergError::NamespaceError(ns_error.to_string()));
    }

    for (flight, batch) in grouped.into_iter() {
        let mut final_table_name: Option<String> = None;

        if table_name.is_none().clone() {
            final_table_name = Some(flight.clone());
        } else {
            final_table_name = Some(table_name.clone().unwrap());
        }

        let table_ident = make_table_ident(final_table_name.unwrap())
            .map_err(|e| IcebergError::TableError(e.to_string()))?;

        let table = match load_table(catalog.clone(), &table_ident).await {
            Ok(t) => t,
            Err(_e) => {
                warn!("Table load failed, attempting create for {}", flight);
                let schema = batch.schema().deref().clone();
                create_table(catalog.clone(), &namespace, &flight, Arc::new(schema))
                    .await
                    .map_err(|e| IcebergError::TableError(e.to_string()))?;
                // try load again
                load_table(catalog.clone(), &table_ident).await?
            }
        };

        let wb = create_parquet_writer(&table)
            .map_err(|e| IcebergError::StorageError(format!("writer build: {}", e)))?;

        // simplistic retry loop
        let mut attempt = 0usize;
        let data_files = loop {
            attempt += 1;
            match write_and_close(wb.clone(), batch.clone()).await {
                Ok(dfiles) => break dfiles,
                Err(e) if attempt < 3 => {
                    warn!(
                        "write_and_close failed (attempt {}): {:?}, retrying",
                        attempt, e
                    );
                    tokio::time::sleep(std::time::Duration::from_millis(200 * attempt as u64))
                        .await;
                    continue;
                }
                Err(e) => {
                    error!("write_and_close failed finally: {:?}", e);
                    return Err(e);
                }
            }
        };

        if let Err(e) = commit_transaction(&table, data_files, catalog.clone()).await {
            error!("commit_transaction failed: {:?}", e);
            return Err(e);
        }
    }
    Ok(())
}

// TODO: Make the table name as an argument
// pub async fn flush_buffer(
//     buffer_manager: Arc<BufferManager>,
//     catalog: Arc<RestCatalog>,
//     namespace: String,
// ) -> Result<(), IcebergError> {
//     let all_batches = buffer_manager.drain_all();
//     let grouped = concat_batches_grouped(&all_batches).await.unwrap();
//
//     if let Err(ns_error) = create_namespace(catalog.clone(), &namespace).await {
//         error!("Fatal error in creating namespace: {}", ns_error);
//         return Err(IcebergError::NamespaceError(ns_error.to_string()));
//     }
//
//     // 4) Process each flight sequentially (no semaphore, no spawning many tasks)
//     for (flight, batch) in grouped.into_iter() {
//         // Resolve table id
//         let table_ident = make_table_ident(flight.clone())
//             .map_err(|e| IcebergError::TableError(e.to_string()))?;
//
//         // Load table. Note: load_table currently locks catalog across await.
//         // If RestCatalog is safe to use without external mutex, remove Mutex wrapper.
//         // For now, call load_table helper (which locks internally).
//         let table = match load_table(catalog.clone(), &table_ident).await {
//             Ok(t) => t,
//             Err(_e) => {
//                 warn!("Table load failed, attempting create for {}", flight);
//                 let schema = batch.schema().deref().clone();
//                 create_table(catalog.clone(), &namespace, &flight, Arc::new(schema))
//                     .await
//                     .map_err(|e| IcebergError::TableError(e.to_string()))?;
//                 // try load again
//                 load_table(catalog.clone(), &table_ident).await?
//             }
//         };
//
//         // 5) Create writer and write with a small retry for transient errors
//         let wb = create_parquet_writer(&table)
//             .map_err(|e| IcebergError::StorageError(format!("writer build: {}", e)))?;
//
//         // simplistic retry loop
//         let mut attempt = 0usize;
//         let data_files = loop {
//             attempt += 1;
//             match write_and_close(wb.clone(), batch.clone()).await {
//                 Ok(dfiles) => break dfiles,
//                 Err(e) if attempt < 3 => {
//                     warn!(
//                         "write_and_close failed (attempt {}): {:?}, retrying",
//                         attempt, e
//                     );
//                     tokio::time::sleep(std::time::Duration::from_millis(200 * attempt as u64))
//                         .await;
//                     continue;
//                 }
//                 Err(e) => {
//                     error!("write_and_close failed finally: {:?}", e);
//                     return Err(e);
//                 }
//             }
//         };
//
//         // 6) Commit
//         if let Err(e) = commit_transaction(&table, data_files, catalog.clone()).await {
//             error!("commit_transaction failed: {:?}", e);
//             return Err(e);
//         }
//     }
//
//     Ok(())
// }

pub async fn write_and_close(
    data_file_writer_builder: DataFileWriterBuilder<
        ParquetWriterBuilder<DefaultLocationGenerator, DefaultFileNameGenerator>,
    >,
    batch: Arc<RecordBatch>,
) -> Result<Vec<DataFile>, IcebergError> {
    let mut writer = data_file_writer_builder.build().await.map_err(|e| {
        IcebergError::StorageError(format!("Failed to build DataFileWriter: {:?}", e))
    })?;
    println!("Arrow RecordBatch schema: {:?}", batch.schema());
    info!("Arrow RecordBatch schema: {:?}", batch.schema());
    println!("Number of columns: {}", batch.num_columns());
    info!("Number of columns: {}", batch.num_columns());
    let schema = batch.schema();
    for i in 0..batch.num_columns() {
        let field = schema.field(i);
        let array = batch.column(i);

        println!(
            "Column {}: Name: {}, Type: {}, Nullable: {}",
            i,
            field.name(),
            field.data_type(),
            field.is_nullable()
        );

        // Get a reference to the array data
        let array_data = array.as_any();

        // Check the data type and print values accordingly
        if let Some(int64_array) = array_data.downcast_ref::<Int64Array>() {
            println!(
                "  Values: {:?}",
                &int64_array.values()[0..std::cmp::min(10, int64_array.len())]
            );
        } else if let Some(str_array) = array_data.downcast_ref::<StringArray>() {
            println!("  Values: {:?}", &str_array.value(0));
            // You can print more values similarly
        } else {
            // Handle other data types as needed
            println!("  Skipping print for this data type.");
        }
    }
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
    catalog: Arc<RestCatalog>,
) -> Result<Table, IcebergError> {
    let tx = Transaction::new(table);
    let commit_id = Some(Uuid::now_v7());
    let key_metadata = vec![];
    info!("Commited to table: {:?}", table);
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

    updated_tx
        .commit(&*catalog)
        .await
        .map_err(|e| IcebergError::CommitError(format!("Failed to commit transaction: {:?}", e)))
}
