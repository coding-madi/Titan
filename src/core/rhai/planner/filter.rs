use crate::api::http::messages::metric_message::FilterExpr;

#[derive(Debug, Clone, PartialEq)]
pub struct Condition {
    pub column: String,
    pub op: Operator,
    pub value: FilterValue,
}

impl Condition {
    pub fn get_column(&self) -> &str {
        &self.column
    }
}

#[derive(Debug, Clone)]
pub enum FilterOperator {
    // A single condition
    Condition(Condition),
    And(Vec<FilterOperator>),
    Or(Vec<FilterOperator>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum FilterValue {
    Int(i64),
    String(String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Operator {
    Eq,
    NotEq,
    Gt,
    Gte,
    Lt,
    Lte,

    // string operations
    Like,
    ILike,
    In,
    NotIn,
    Regex,
    NotRegex,
    IsNull,
    IsNotNull,
    BeginsWith,
    EndsWith,
}

impl From<FilterExpr> for FilterOperator {
    fn from(expr: FilterExpr) -> Self {
        match expr {
            FilterExpr::And(exprs) => {
                FilterOperator::And(exprs.into_iter().map(FilterOperator::from).collect())
            }
            FilterExpr::Or(exprs) => {
                FilterOperator::Or(exprs.into_iter().map(FilterOperator::from).collect())
            }
            FilterExpr::Eq(col, val) => FilterOperator::Condition(Condition {
                column: col,
                op: Operator::Eq,
                value: FilterValue::String(val),
            }),
            FilterExpr::Neq(col, val) => FilterOperator::Condition(Condition {
                column: col,
                op: Operator::NotEq,
                value: FilterValue::String(val),
            }),
            FilterExpr::Gt(col, val) => FilterOperator::Condition(Condition {
                column: col,
                op: Operator::Gt,
                value: FilterValue::Int(val as i64), // or Float if you add support
            }),
            FilterExpr::Lt(col, val) => FilterOperator::Condition(Condition {
                column: col,
                op: Operator::Lt,
                value: FilterValue::Int(val as i64),
            }),
            FilterExpr::RegexMatch(col, regex) => FilterOperator::Condition(Condition {
                column: col,
                op: Operator::Regex,
                value: FilterValue::String(regex),
            }),
        }
    }
}
