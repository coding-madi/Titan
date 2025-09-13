use crate::api::http::messages::validations::validate_group_by;
use serde_derive::{Deserialize, Serialize};
use validator::Validate;
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct MetricRule {
    #[validate(length(min = 1))]
    rule_name: String,
    scope: Scope,
    #[validate(range(min = 1, max = 300))]
    window_sec: u16,
    filter: FilterExpr,
    #[validate(custom(function = "validate_group_by"))]
    group_by: Vec<String>,
    aggregations: Vec<AggregationFn>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Scope {
    Global,
    Local,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FilterExpr {
    And(Vec<FilterExpr>),
    Or(Vec<FilterExpr>),
    Eq(String, String),  // col == value
    Neq(String, String), // col != value
    Gt(String, f64),
    Lt(String, f64),
    RegexMatch(String, String), // col regex match
}

#[derive(Debug, Clone, Serialize, Deserialize)]
enum AggregationFn {
    Count(Option<String>), // count of rows or count of a field
    Sum(StringOrCast),
    Avg(StringOrCast),
    Min(StringOrCast),
    Max(StringOrCast),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
enum StringOrCast {
    Field(String),
    CastToInt(String),
    CastToFloat(String),
}
