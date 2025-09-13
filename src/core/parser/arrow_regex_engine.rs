// use std::collections::HashMap;
// use std::sync::Arc;
// use crate::application::actors::broadcast::RecordBatchWrapper;
// use crate::core::error::exception::regex::RegexError;
// use crate::core::parser::parser_contract::ParserContract;
// use crate::core::parser::messages::parser::Pattern;
//
// use arrow_array::{Array, StringArray, ListArray, GenericStringArray, MapArray, StructArray};
// use arrow::compute::kernels::regexp::regexp_match;
// use iceberg::spec::Datum;
// use rayon::prelude::*;
// use serde_json::{Value, Map};
//
// pub struct ArrowRegexEngine;
//
// /// Intermediate lightweight struct for captures
// #[derive(Debug)]
// struct CaptureRow<'a> {
//     field: &'a str,
//     groups: Vec<Option<&'a str>>,
// }
//
// #[derive(Debug)]
// struct CaptureRowOwned {
//     field: String,
//     groups: Vec<Option<String>>,
// }
//
// impl ParserContract for ArrowRegexEngine {
//     fn parse(
//         &self,
//         arrow_batches: Vec<RecordBatchWrapper>,
//         patterns: HashMap<String, Vec<Pattern>>,
//         sample: bool
//     ) -> Result<StructArray, RegexError> {
//         // Use Rayon to parallelize over batches
//         let results: Vec<CaptureRowOwned> = arrow_batches
//             .into_par_iter() // parallel iterator
//             .flat_map(|batch| {
//                 patterns.par_iter().flat_map(move |pattern| {
//                     match pattern {
//                         Pattern::RegexPattern(regex_pat) => {
//                             let field = &regex_pat.field;
//
//                             let col = match batch
//                                 .data
//                                 .column_by_name(field)
//                             {
//                                 Some(c) => c,
//                                 None => return vec![].into_par_iter(), // skip if column not found
//                             };
//
//                             let string_array = match col
//                                 .as_any()
//                                 .downcast_ref::<StringArray>()
//                             {
//                                 Some(sa) => sa,
//                                 None => return vec![].into_par_iter(), // skip if not StringArray
//                             };
//
//                             let take = if sample { string_array.len().min(10) } else { string_array.len() };
//                             let sliced_input = string_array.slice(0, take);
//
//                             let regex_array = GenericStringArray::<i32>::from(
//                                 vec![Some(regex_pat.pattern_string.as_str()); take]
//                             );
//
//                             let result = match regexp_match(&sliced_input, &regex_array, None) {
//                                 Ok(r) => r,
//                                 Err(_) => return vec![].into_par_iter(), // skip on error
//                             };
//
//                             let list_array = match result.as_any().downcast_ref::<ListArray>() {
//                                 Some(la) => la,
//                                 None => return vec![].into_par_iter(),
//                             };
//
//                             (0..list_array.len())
//                                 .into_par_iter()
//                                 .filter_map(move |row| {
//                                     if list_array.is_null(row) {
//                                         return None;
//                                     }
//
//                                     let group_values: Arc<dyn Array> = list_array.value(row);
//                                     let string_values: &GenericStringArray<i32> = group_values
//                                         .as_any()
//                                         .downcast_ref::<GenericStringArray<i32>>()?;
//
//                                     let groups: Vec<Option<String>> = string_values
//                                         .iter()
//                                         .map(|opt| opt.map(|s| s.to_string()))
//                                         .collect();
//
//                                     Some(CaptureRowOwned {
//                                         field: field.to_string(),
//                                         groups,
//                                     })
//                                 })
//                                 .collect::<Vec<_>>()
//                                 .into_par_iter()
//                         }
//                         Pattern::GrokPattern(_) => {
//                             unimplemented!()
//                         }
//                     }
//                 })
//                     .collect::<Vec<_>>()
//                     .into_par_iter()
//             })
//             .collect();
//
//         // Convert to JSON at the very end (outside hot path)
//         let mut final_obj = Map::new();
//         for capture in results {
//             let arr: Map<String, Value> = capture
//                 .groups
//                 .into_iter()
//                 .enumerate()
//                 .filter_map(|(idx, val_opt)| {
//                     val_opt.map(|v| (idx.to_string(), Value::String(v)))
//                 })
//                 .collect();
//
//             final_obj.insert(capture.field, Value::Object(arr));
//         }
//         unimplemented!()
//     }
// }
//
