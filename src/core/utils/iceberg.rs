use arrow_array::RecordBatch;
use arrow_schema::Schema;
use iceberg::TableIdent;
use iceberg::spec::Schema as IcebergSchema;
use std::sync::Arc;

pub fn convert_arrow_to_iceberg_schema(arrow_schema: &Arc<Schema>) -> IcebergSchema {
    //built-in converter to handle all field ID mappings correctly
    iceberg::arrow::arrow_schema_to_schema(arrow_schema).unwrap()
}

pub fn make_table_ident(table: String) -> Result<TableIdent, String> {
    TableIdent::from_strs(vec!["log", &table.clone()])
        .map_err(|e| format!("Failed to create TableIdent: {:?}", e))
}

pub fn print_schema_info(batch: &RecordBatch) {
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
