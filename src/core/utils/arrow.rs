use crate::application::actors::broadcaster::broadcast_actor::RecordBatchWrapper;
use crate::core::error::exception::regex::RegexError;
use arrow::compute::concat_batches;
use arrow::util::pretty::print_batches;
use arrow_array::{Array, RecordBatch, StringArray, StructArray};
use arrow_schema::{ArrowError, DataType};
use serde_json::{Map, Value};
use std::collections::HashMap;
use std::ops::Deref;
use std::sync::Arc;

pub async fn concat_batches_grouped(
    batches: &Vec<RecordBatchWrapper>,
) -> Result<HashMap<String, Arc<RecordBatch>>, ArrowError> {
    if batches.is_empty() {
        return Err(ArrowError::InvalidArgumentError(
            "Buffer is empty.".to_string(),
        ));
    }

    // Step 1: Group by flight metadata
    let mut groups: HashMap<String, Vec<&RecordBatchWrapper>> = HashMap::new();
    for b in batches {
        let key = b.get_flight_name();
        groups.entry(key.parse().unwrap()).or_default().push(b);
    }

    // Step 2: Concatenate per group
    let mut results = HashMap::new();
    for (flight, group_batches) in groups {
        let schema = group_batches[0].get_data().schema();
        let refs = group_batches
            .iter()
            .map(|b| b.get_data_ref())
            .collect::<Vec<_>>();

        let concatenated = concat_batches(&schema, refs).map(Arc::new).map_err(|e| e);
        match concatenated {
            Ok(concatenated) => {
                results.insert(flight, concatenated);
            }
            Err(e) => {
                return Err(e);
            }
        }
    }
    Ok(results)
}

pub fn arrow_buffer_to_json(data: &RecordBatch) -> Vec<Value> {
    let struct_array = data
        .column_by_name("parsed")
        .unwrap()
        .as_any()
        .downcast_ref::<StructArray>()
        .unwrap();
    struct_array_to_json(struct_array)
}

pub fn struct_array_to_json(struct_array: &StructArray) -> Vec<Value> {
    let num_rows = struct_array.len();
    let mut result = Vec::with_capacity(num_rows);

    // Get field names from data type
    let field_names: Vec<String> = match struct_array.data_type() {
        DataType::Struct(fields) => fields.iter().map(|f| f.name().clone()).collect(),
        _ => panic!("Not a StructArray"),
    };

    for row in 0..10 {
        if struct_array.is_null(row) {
            result.push(Value::Null);
            continue;
        }

        let mut obj = Map::new();

        for (col_idx, field) in struct_array.columns().iter().enumerate() {
            let field_name = &field_names[col_idx];

            // Assuming StringArray for simplicity
            let col = field
                .as_any()
                .downcast_ref::<arrow_array::StringArray>()
                .unwrap();

            let val = if col.is_null(row) {
                Value::Null
            } else {
                Value::String(col.value(row).to_string())
            };

            obj.insert(field_name.clone(), val);
        }

        result.push(Value::Object(obj));
    }
    result
}

pub fn extract_col_from_flight_buffer<'a>(
    data: &'a RecordBatch,
    col_name: &str,
) -> &'a StringArray {
    let log_name_array = data
        .column_by_name(col_name)
        .ok_or_else(|| RegexError::RegexIncorrect("log_group_name column not found".to_string()))
        .unwrap()
        .as_any()
        .downcast_ref::<StringArray>()
        .ok_or_else(|| RegexError::RegexIncorrect("log_group_name is not StringArray".to_string()))
        .unwrap();
    log_name_array
}

pub fn print_record_batch(record_batch: Arc<RecordBatch>) {
    print_batches(&[record_batch.deref().clone()]).unwrap();
}
