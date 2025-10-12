pub(crate) use crate::api::http::messages::regex_messages::{Pattern, RegexPattern};
use crate::core::error::exception::regex::RegexError;
use arrow_array::{Array, StringArray};
use regex::Regex;
use std::collections::HashMap;
use validator::ValidationError;

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

pub fn group_logs_by_log_group_name(
    log_group_name_array: &StringArray,
) -> HashMap<&str, Vec<usize>> {
    let mut log_rows: HashMap<&str, Vec<usize>> = HashMap::new();
    for i in 0..log_group_name_array.len() {
        if log_group_name_array.is_valid(i) {
            log_rows
                .entry(log_group_name_array.value(i))
                .or_default()
                .push(i);
        }
    }
    log_rows
}
