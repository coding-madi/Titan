use crate::application::actors::broadcast::RecordBatchWrapper;
use arrow::compute::concat_batches;
use arrow_array::RecordBatch;
use arrow_schema::ArrowError;
use std::collections::HashMap;
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
        let key = b.metadata.flight.clone();
        groups.entry(key).or_default().push(b);
    }

    // Step 2: Concatenate per group
    let mut results = HashMap::new();
    for (flight, group_batches) in groups {
        let schema = group_batches[0].data.schema();
        let refs = group_batches.iter().map(|b| &*b.data).collect::<Vec<_>>();

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
