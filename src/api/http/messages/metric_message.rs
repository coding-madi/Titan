use crate::api::http::messages::validations::validate_group_by;
use crate::application::actors::rhai::handler::metric_rule_dto::MetricRuleDTO;
use serde_derive::{Deserialize, Serialize};
use validator::Validate;

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct MetricRule {
    #[validate(length(min = 1))]
    flight_name: String,
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

impl MetricRule {
    pub fn get_flight_name(&self) -> &str {
        &self.flight_name
    }
    pub fn get_rule_name(&self) -> &str {
        &self.rule_name
    }
    pub fn get_scope(&self) -> &Scope {
        &self.scope
    }
    pub fn get_window_sec(&self) -> u16 {
        self.window_sec
    }
    pub fn get_filter(&self) -> &FilterExpr {
        &self.filter
    }
    pub fn get_group_by(&self) -> &Vec<String> {
        &self.group_by
    }
    pub fn get_aggregations(&self) -> &Vec<AggregationFn> {
        &self.aggregations
    }
}

impl From<MetricRuleDTO> for MetricRule {
    fn from(value: MetricRuleDTO) -> Self {
        let metric_rule = MetricRule {
            flight_name: value.get_flight_name().parse().unwrap(),
            rule_name: value.rule_name.clone(),
            scope: value.scope.clone(),
            window_sec: value.window_sec.clone(),
            filter: value.filter.clone(),
            group_by: value.group_by.clone(),
            aggregations: value.aggregations.clone(),
        };
        metric_rule
    }
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
pub enum AggregationFn {
    Count(Option<String>), // count of rows or count of a field
    Sum(StringOrCast),
    Avg(StringOrCast),
    Min(StringOrCast),
    Max(StringOrCast),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StringOrCast {
    Field(String),
    CastToInt(String),
    CastToFloat(String),
}

impl From<&StringOrCast> for String {
    fn from(value: &StringOrCast) -> Self {
        match value {
            StringOrCast::Field(field) => field.clone(),
            StringOrCast::CastToInt(cast) => cast.clone(),
            StringOrCast::CastToFloat(cast) => cast.clone(),
        }
    }
}
