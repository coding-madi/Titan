use crate::application::actors::broadcaster::broadcast_actor::RecordBatchWrapper;
use crate::core::error::exception::regex::RegexError;
use crate::core::parser::messages::parser::Pattern;
use crate::core::parser::parser_contract::ParserContract;
use crate::core::utils::arrow::extract_col_from_flight_buffer;
use arrow_array::builder::GenericStringBuilder;
use arrow_array::{Array, ArrayRef, RecordBatch, StructArray};
use arrow_schema::{DataType, Field, Fields};
use std::collections::HashMap;
use std::ops::Deref;
use std::sync::Arc;

pub struct RustRegexEngine;

impl RustRegexEngine {
    fn collect_all_unique_field_names(arrays: &Vec<StructArray>) -> Result<Vec<Field>, RegexError> {
        if arrays.is_empty() {
            return Err(RegexError::RegexIncorrect(
                "No parsed arrays to merge".to_string(),
            ));
        }
        let mut all_fields: Vec<Field> = Vec::new();
        let mut seen = std::collections::HashSet::new();
        for arr in arrays {
            for f in arr.fields() {
                if seen.insert(f.name().clone()) {
                    all_fields.push(f.deref().clone());
                }
            }
        }
        Ok(all_fields)
    }

    fn align_children(arrays: Vec<StructArray>, unique_fields: Vec<Field>) -> Vec<Vec<ArrayRef>> {
        // Align the fields
        let mut aligned_children: Vec<Vec<ArrayRef>> = vec![Vec::new(); unique_fields.len()];

        for arr in arrays {
            let struct_fields: HashMap<String, ArrayRef> = arr
                .fields()
                .iter()
                .zip(arr.columns().iter())
                .map(|(f, col)| (f.name().clone(), col.clone()))
                .collect();

            for (i, f) in unique_fields.iter().enumerate() {
                if let Some(col) = struct_fields.get(f.name()) {
                    aligned_children[i].push(col.clone());
                } else {
                    // Fill with nulls of same length
                    let nulls = arrow_array::new_null_array(&f.data_type().clone(), arr.len());
                    aligned_children[i].push(nulls);
                }
            }
        }
        aligned_children
    }

    fn concatenate_struct_fields(aligned_children: Vec<Vec<ArrayRef>>) -> Vec<ArrayRef> {
        let mut merged_columns: Vec<ArrayRef> = Vec::new();
        for child_vec in aligned_children {
            let refs: Vec<&dyn arrow_array::Array> = child_vec.iter().map(|a| a.as_ref()).collect();
            let concat = arrow::compute::concat(refs.as_slice())
                .map_err(|e| RegexError::RegexIncorrect(format!("Concat error: {e}")))
                .unwrap();
            merged_columns.push(concat);
        }
        merged_columns.into()
    }

    fn merge_parsed_arrays(arrays: Vec<StructArray>) -> Result<StructArray, RegexError> {
        let unique_fields = Self::collect_all_unique_field_names(&arrays)?;

        // Align the fields
        let aligned_children: Vec<Vec<ArrayRef>> =
            Self::align_children(arrays, unique_fields.clone());

        let concatenated_struct_fields = Self::concatenate_struct_fields(aligned_children);

        let field_array_pairs: Vec<(Arc<Field>, ArrayRef)> = unique_fields
            .into_iter()
            .zip(concatenated_struct_fields.into_iter())
            .map(|(field, array)| (Arc::new(field), array))
            .collect();
        Ok(StructArray::from(field_array_pairs))
    }

    fn parse_batch_grouped(
        &self,
        batch: &RecordBatchWrapper,
        patterns: &HashMap<String, Vec<Pattern>>,
        sample: bool,
    ) -> Result<StructArray, RegexError> {
        let data = batch.get_data();
        let log_name_array = extract_col_from_flight_buffer(&*data, "log_group_name");

        // Collect all possible capture fields across patterns
        let mut all_fields: Vec<Field> = Vec::new();
        let mut seen = std::collections::HashSet::new();

        for pats in patterns.values() {
            for pattern in pats {
                if let Pattern::RegexPattern(rp) = pattern {
                    for idx in 1..rp.regex.captures_len() {
                        let cap_name = format!("{}_group{}", rp.field, idx);
                        if seen.insert(cap_name.clone()) {
                            all_fields.push(Field::new(&cap_name, DataType::Utf8, true));
                        }
                    }
                }
            }
        }

        // Initialize builders in deterministic order
        let mut builders: HashMap<String, GenericStringBuilder<i32>> = all_fields
            .iter()
            .map(|f| (f.name().clone(), GenericStringBuilder::<i32>::new()))
            .collect();

        let batch_len = data.num_rows();

        // For each row, apply regex or append nulls
        for row_idx in 0..batch_len {
            let log_group_name = log_name_array.value(row_idx);
            let log_patterns = match patterns.get(log_group_name) {
                Some(p) => p,
                None => {
                    for b in builders.values_mut() {
                        b.append_null();
                    }
                    continue;
                }
            };

            let mut matched_any = false;

            for pattern in log_patterns {
                if let Pattern::RegexPattern(rp) = pattern {
                    let string_array = extract_col_from_flight_buffer(&*data, &rp.field);
                    let val = string_array.value(row_idx);

                    if let Some(caps) = rp.regex.captures(val) {
                        matched_any = true;
                        for idx in 1..caps.len() {
                            let cap_name = format!("{}_group{}", rp.field, idx);
                            if let Some(builder) = builders.get_mut(&cap_name) {
                                builder.append_option(caps.get(idx).map(|m| m.as_str()));
                            }
                        }
                    } else {
                        for idx in 1..rp.regex.captures_len() {
                            let cap_name = format!("{}_group{}", rp.field, idx);
                            if let Some(builder) = builders.get_mut(&cap_name) {
                                builder.append_null();
                            }
                        }
                    }
                }
            }

            if !matched_any {
                for b in builders.values_mut() {
                    b.append_null();
                }
            }
        }

        // Build arrays in the same order as all_fields
        let arrays: Vec<ArrayRef> = all_fields
            .iter()
            .map(|f| {
                let mut builder = builders.remove(f.name()).unwrap();
                Arc::new(builder.finish()) as ArrayRef
            })
            .collect();

        if all_fields.is_empty() {
            // No regex fields → create an empty StructArray with length = batch_len

            let res = StructArray::try_new_with_length(Fields::empty(), vec![], None, 0);
            return Ok(res.unwrap());
        }

        let field_array_pairs: Vec<(Arc<Field>, ArrayRef)> = all_fields
            .into_iter()
            .zip(arrays.into_iter())
            .map(|(f, arr)| (Arc::new(f), arr))
            .collect();

        Ok(StructArray::from(field_array_pairs))
    }
}

impl ParserContract for RustRegexEngine {
    /// Parses the structs, and then updates the current buffer, with the structs and returns it
    fn parse(
        &self,
        record_wrappers: Vec<RecordBatchWrapper>,
        patterns: HashMap<String, Vec<Pattern>>, // log_group, Pattern per column
        sample: bool,
    ) -> Result<Vec<RecordBatchWrapper>, RegexError> {
        let mut updated_wrappers: Vec<RecordBatchWrapper> = Vec::new();

        for batch in record_wrappers {
            let parsed_struct = self.parse_batch_grouped(&batch, &patterns, false)?;
            let orig_batch = batch.get_data();

            let mut fields: Vec<Arc<Field>> =
                orig_batch.schema().fields().iter().cloned().collect();
            fields.push(Arc::new(Field::new(
                "parsed",
                parsed_struct.data_type().clone(),
                true,
            )));

            let new_schema = Arc::new(arrow_schema::Schema::new(fields));
            let mut cols = orig_batch.columns().to_vec();
            cols.push(Arc::new(parsed_struct) as ArrayRef);
            let new_batch = RecordBatch::try_new(new_schema, cols)
                .map(Arc::new)
                .unwrap_or_else(|_err| {
                    // Fallback: return original batch unchanged
                    orig_batch.clone()
                });

            let metadata = batch.get_metadata();
            updated_wrappers.push(RecordBatchWrapper::new(metadata.clone(), &new_batch));
        }

        Ok(updated_wrappers)
    }
}
