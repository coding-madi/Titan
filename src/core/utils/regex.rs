use arrow::compute::filter_record_batch;
use arrow_array::{Array, BooleanArray, StringArray};
use log::info;
use crate::api::http::messages::regex_messages::{Pattern, RegexPattern};
use crate::application::actors::broadcast::RecordBatchWrapper;
use crate::core::error::exception::regex::RegexError;
use regex::Regex;
use serde_json::Value;
use tracing::debug;
use validator::ValidationError;
use crate::application::actors::parser::TryParsingRegex;

pub async fn apply_regex(record_wrappers: Vec<RecordBatchWrapper>, regex_request: TryParsingRegex) -> Value {
    // get the filter criteria
    let log_group = regex_request.log_group.clone();
    info!("Applying regex test on {}", regex_request.log_group);

    let mut final_obj = serde_json::Map::new();
    for record_wrapper in record_wrappers {
        let mut combined_mask = BooleanArray::from(vec![true; record_wrapper.data.num_rows()]);

        for pattern in &regex_request.pattern {
            match pattern {
                Pattern::RegexPattern(regex_pat) => {
                    let field = &regex_pat.field;
                    let regex = Regex::new(&regex_pat.pattern_string)
                        .expect("invalid regex");

                    let col = record_wrapper
                        .data
                        .column_by_name(field)
                        .unwrap_or_else(|| panic!("column {} not found", field));

                    let string_array = col
                        .as_any()
                        .downcast_ref::<StringArray>()
                        .expect("column not string");

                    let values: Vec<&str> = (0..string_array.len())
                        .take(10)
                        .filter_map(|i| if string_array.is_null(i) { None } else { Some(string_array.value(i)) })
                        .collect();

                    let mut groups = Vec::new();

                    for val in values {
                        if let Some(caps) = regex.captures(val) {
                            let mut obj = serde_json::Map::new();
                            for name in regex.capture_names().flatten() {
                                if let Some(m) = caps.name(name) {
                                    obj.insert(name.to_string(), Value::String(m.as_str().to_string()));
                                }
                            }
                            groups.push(Value::Object(obj));
                        }
                    }

                    final_obj.insert(regex_pat.field.clone(), Value::Array(groups));


                },
                Pattern::GrokPattern(_grok) => {
                    // TODO: grok support here
                }
            }
        }
    }

    Value::Object(final_obj)
}

pub fn validate_patterns(patterns: &Vec<Pattern>) -> Result<(), RegexError> {
    for pattern in patterns {
        match pattern {
            Pattern::RegexPattern(rp) => {
                if !is_valid_regex(rp) {
                    return Err(RegexError::RegexIncorrect("Invalid Regex".to_string()));
                }
            }
            Pattern::GrokPattern(grok) => {
                panic!("Grok pattern not implemented yet")
            }
            _ => return Err(RegexError::RegexIncorrect("Invalid Regex".to_string())),
        }
    }
    Ok(())
}

pub fn validate_regex_pattern(patterns: &Vec<Pattern>) -> Result<(), ValidationError> {
    for pattern in patterns {
        match pattern {
            Pattern::RegexPattern(rp) => {
                if !is_valid_regex(rp) {
                    return Err(ValidationError::new("invalid_regex"));
                }
            }
            _ => return Err(ValidationError::new("unsupported_pattern_type")),
        }
    }
    Ok(())
}

pub fn is_valid_regex(regex: &RegexPattern) -> bool {
    Regex::new(&regex.pattern_string).is_ok()
}
