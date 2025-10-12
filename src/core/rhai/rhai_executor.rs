use crate::application::actors::broadcaster::broadcast_actor::RecordBatchWrapper;
use crate::core::rhai::planner::filter::{Condition, FilterOperator, FilterValue, Operator};
use crate::core::rhai::query_planner::{AggregateOperation, QueryPlanner};
use arrow::compute;
use arrow::compute::filter_record_batch;
use arrow::compute::kernels::{cmp, comparison};
use arrow_array::builder::{Int64Builder, StringBuilder};
use arrow_array::{Array, ArrayRef, BooleanArray, Int64Array, RecordBatch, StringArray};
use arrow_schema::{ArrowError, DataType};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::error;

pub struct RhaiExecutor {
    flight_name: String,
    record: Vec<RecordBatchWrapper>,
}

// Takes plan and engine and executes
// has logic for executing the plans

impl RhaiExecutor {
    pub fn new(flight_name: String) -> Self {
        Self {
            flight_name,
            record: vec![],
        }
    }

    pub fn append_logs(&mut self, buffer: RecordBatchWrapper) {
        self.record.push(buffer);
    }

    pub fn apply(
        data: &Vec<RecordBatchWrapper>,
        planner: QueryPlanner,
    ) -> Result<Arc<RecordBatch>, ArrowError> {
        // Apply the filter
        if let Some(filter_expr) = planner.filter.clone() {
            // Validate filter has correct columns
            if !Self::validate_columns_in_vector(
                data.iter().map(|s| s.get_data()).collect(),
                &filter_expr,
            ) {
                error!("One or more columns in the filter expression are not present in the data.");
                return Err(ArrowError::InvalidArgumentError(
                    "One or more columns in the filter expression are not present in the data."
                        .to_string(),
                ));
            }
            let service = data.iter().map(|s| s.get_flight_name()).last();
            let start_time = data
                .iter()
                .map(|s| s.get_metadata().oldest_timestamp.unwrap())
                .min()
                .unwrap();
            let end_time = data
                .iter()
                .map(|s| s.get_metadata().newest_timestamp.unwrap())
                .max()
                .unwrap();

            let record_batch =
                Self::apply_filter(data.first().unwrap().get_data().clone(), &filter_expr)?;

            let grouped_record_batch =
                Self::group_by_index(record_batch.clone(), planner.group_by.clone());

            let mut key_builder = StringBuilder::new();
            let mut sum_builder = Int64Builder::new();
            let mut count_builder = Int64Builder::new();
            let length = grouped_record_batch.len();
            let service_ids = repeated_string_array(service.unwrap(), length);
            let start_times = repeated_int_array(start_time, length);
            let end_times = repeated_int_array(end_time, length);

            for (key, indices) in grouped_record_batch {
                key_builder.append_value(&key);

                for (op, col) in &planner.aggregates {
                    let col_idx = record_batch.schema().index_of(col)?;
                    let array = record_batch.column(col_idx);
                    match (op, array.data_type()) {
                        (AggregateOperation::Sum, DataType::Int64) => {
                            let arr = array.as_any().downcast_ref::<Int64Array>().unwrap();
                            let sum: i64 = indices.iter().map(|&i| arr.value(i as usize)).sum();
                            sum_builder.append_value(sum);
                        }
                        (AggregateOperation::Count, DataType::Int64) => {
                            let arr = array.as_any().downcast_ref::<Int64Array>().unwrap();
                            let count: i64 =
                                indices.iter().map(|&i| arr.value(i as usize)).count() as i64;
                            count_builder.append_value(count);
                        }
                        _ => unimplemented!(),
                    }
                }
            }

            let key_array = Arc::new(key_builder.finish()) as ArrayRef;
            let sum_array = Arc::new(sum_builder.finish()) as ArrayRef;

            let schema = Arc::new(arrow_schema::Schema::new(vec![
                arrow_schema::Field::new("service_id", DataType::Utf8, false),
                arrow_schema::Field::new("key", DataType::Utf8, false),
                arrow_schema::Field::new("sum", DataType::Int64, false),
                arrow_schema::Field::new("start_time", DataType::Int64, false),
                arrow_schema::Field::new("end_time", DataType::Int64, false),
            ]));

            let result_batch = RecordBatch::try_new(
                schema,
                vec![service_ids, key_array, sum_array, start_times, end_times],
            )?;

            Ok(Arc::new(result_batch))
        } else {
            Err(ArrowError::InvalidArgumentError(
                "No filter expression provided.".to_string(),
            ))
        }
    }

    fn build_mask(
        batch: &RecordBatch,
        filter: &FilterOperator,
    ) -> Result<BooleanArray, ArrowError> {
        match filter {
            FilterOperator::Condition(cond) => {
                let array_ref =
                    Self::column_as_array(Arc::new(batch.clone()), cond.get_column()).unwrap();
                Self::apply_condition(array_ref, cond)
            }
            FilterOperator::And(filters) => {
                let mut masks = filters
                    .iter()
                    .map(|f| Self::build_mask(batch, f))
                    .collect::<Result<Vec<_>, _>>()?;
                let mut mask = masks.remove(0);
                for m in masks {
                    mask = compute::kernels::boolean::and(&mask, &m)?;
                }
                Ok(mask)
            }
            FilterOperator::Or(filters) => {
                let mut masks = filters
                    .iter()
                    .map(|f| Self::build_mask(batch, f))
                    .collect::<Result<Vec<_>, _>>()?;
                let mut mask = masks.remove(0);
                for m in masks {
                    mask = compute::kernels::boolean::or(&mask, &m)?;
                }
                Ok(mask)
            }
        }
    }

    fn filter_with_mask(
        batch: &RecordBatch,
        mask: &BooleanArray,
    ) -> Result<Arc<RecordBatch>, ArrowError> {
        let filtered_records: Result<RecordBatch, ArrowError> = filter_record_batch(&*batch, mask);
        filtered_records.map(|x| Arc::new(x))
    }

    fn apply_condition(array: ArrayRef, condition: &Condition) -> Result<BooleanArray, ArrowError> {
        match (&condition.op, &condition.value) {
            (_, FilterValue::Int(value)) => {
                if let Some(int_array) = array.as_any().downcast_ref::<Int64Array>() {
                    let scalar_array = Int64Array::from(vec![*value; int_array.len()]);
                    match condition.op {
                        Operator::Eq => cmp::eq(int_array, &scalar_array),
                        Operator::NotEq => cmp::neq(int_array, &scalar_array),
                        Operator::Gt => cmp::gt(int_array, &scalar_array),
                        Operator::Lt => cmp::lt(int_array, &scalar_array),
                        Operator::Gte => cmp::gt_eq(int_array, &scalar_array),
                        Operator::Lte => cmp::lt_eq(int_array, &scalar_array),
                        _ => Err(ArrowError::NotYetImplemented(
                            "Unsupported operator or value type".to_string(),
                        )),
                    }
                } else {
                    Err(ArrowError::NotYetImplemented(
                        "Unsupported operator or value type".to_string(),
                    ))
                }
            }
            (_, FilterValue::String(value)) => {
                let string_array = array
                    .as_any()
                    .downcast_ref::<StringArray>()
                    .ok_or_else(|| ArrowError::ComputeError("Expected StringArray".into()))?;

                let scalar_array: StringArray = (0..string_array.len())
                    .map(|_| value.as_str())
                    .collect::<Vec<&str>>()
                    .into();

                match condition.op {
                    Operator::Eq => comparison::contains(string_array, &scalar_array),
                    Operator::Like => comparison::like(string_array, &scalar_array),
                    Operator::ILike => comparison::ilike(string_array, &scalar_array),
                    Operator::BeginsWith => comparison::starts_with(string_array, &scalar_array),
                    Operator::EndsWith => comparison::ends_with(string_array, &scalar_array),
                    _ => Err(ArrowError::NotYetImplemented(
                        "Unsupported operator or value type".to_string(),
                    )),
                }
            }
        }
    }

    /// Given a RecordBatch and a column name, get a reference to the entire column
    fn column_as_array(batch: Arc<RecordBatch>, col_name: &str) -> Option<ArrayRef> {
        let idx = batch.schema().index_of(col_name).ok()?;
        Some(batch.column(idx).clone())
    }

    fn apply_filter(
        batch: Arc<RecordBatch>,
        filter: &FilterOperator,
    ) -> Result<Arc<RecordBatch>, ArrowError> {
        Self::build_mask(&batch, filter).and_then(|mask| Self::filter_with_mask(&batch, &mask))
    }

    fn validate_columns_in_vector(batches: Vec<Arc<RecordBatch>>, filter: &FilterOperator) -> bool {
        let mut is_col_valid = true;
        for batch in batches {
            if !Self::validate_columns(batch, filter) {
                is_col_valid = false;
            }
        }
        is_col_valid
    }

    fn validate_columns(batch: Arc<RecordBatch>, filter: &FilterOperator) -> bool {
        match filter {
            FilterOperator::Condition(condition) => {
                batch.schema().field_with_name(&condition.column).is_ok()
            }
            FilterOperator::And(filters) | FilterOperator::Or(filters) => filters
                .iter()
                .all(|f| Self::validate_columns(batch.clone(), f)),
        }
    }

    fn group_by_index(batch: Arc<RecordBatch>, group_by: Vec<String>) -> HashMap<String, Vec<i64>> {
        let keys: Vec<ArrayRef> = group_by
            .iter()
            .map(|name| batch.column(batch.schema().index_of(name).unwrap()).clone())
            .collect();

        let mut groups: HashMap<String, Vec<i64>> = HashMap::new();
        for i in 0..batch.num_rows() {
            let mut key_tuple = Vec::new();
            for k in &keys {
                match k.data_type() {
                    DataType::Int64 => {
                        let arr = k.as_any().downcast_ref::<Int64Array>().unwrap();
                        key_tuple.push(arr.value(i).to_string());
                    }
                    DataType::Utf8 => {
                        let arr = k.as_any().downcast_ref::<StringArray>().unwrap();
                        key_tuple.push(arr.value(i).to_string());
                    }
                    _ => unreachable!(),
                }
            }
            let key_str = key_tuple.join("-"); // "name-address"
            groups.entry(key_str).or_default().push(i as i64);
        }
        groups
    }
}

fn repeated_string_array(value: &str, len: usize) -> ArrayRef {
    let repeated = std::iter::repeat(value).take(len);
    Arc::new(StringArray::from_iter_values(repeated)) as ArrayRef
}

fn repeated_int_array(value: i64, len: usize) -> ArrayRef {
    let repeated = std::iter::repeat(value).take(len);
    Arc::new(Int64Array::from_iter_values(repeated)) as ArrayRef
}
