use crate::api::http::messages::metric_message::AggregationFn;
use crate::core::rhai::planner::filter::{Condition, FilterOperator, FilterValue, Operator};

/// Rhai gets converted into this object
/// This only store the metadata about the plan.
/// The actual funtion logic is part f rhai executor
#[derive(Debug, Clone, PartialEq)]
pub enum AggregateOperation {
    Sum,
    Count,
    Min,
    Max,
    Avg,
}

#[derive(Debug, Clone)]
pub struct QueryPlanner {
    pub filter: Option<FilterOperator>,
    pub group_by: Vec<String>,
    pub aggregates: Vec<(AggregateOperation, String)>, // (op, column)
    pub window: Option<usize>,
    pub limit: Option<usize>,
}

impl QueryPlanner {
    pub fn new() -> Self {
        Self {
            filter: None,
            aggregates: vec![],
            group_by: vec![],
            window: None,
            limit: None,
        }
    }
}

impl From<&AggregationFn> for (AggregateOperation, String) {
    fn from(agg: &AggregationFn) -> Self {
        match agg {
            AggregationFn::Count(col_opt) => {
                let col_name = col_opt.clone().unwrap_or_else(|| "*".to_string());
                (AggregateOperation::Count, col_name)
            }
            AggregationFn::Sum(expr) => (AggregateOperation::Sum, expr.into()),
            AggregationFn::Avg(expr) => (AggregateOperation::Avg, expr.into()),
            AggregationFn::Min(expr) => (AggregateOperation::Min, expr.into()),
            AggregationFn::Max(expr) => (AggregateOperation::Max, expr.into()),
        }
    }
}

/// Create binding functions for DSL
impl QueryPlanner {
    pub fn filter(mut self, f: FilterOperator) -> Self {
        self.filter = Some(f);
        self
    }

    pub fn agg(mut self, op: AggregateOperation, col: &str) -> Self {
        self.aggregates.push((op.into(), col.into()));
        self
    }

    pub fn agg_str(self, op: &str, col: &str) -> Self {
        let op_enum = match op.to_lowercase().as_str() {
            "sum" => AggregateOperation::Sum,
            "count" => AggregateOperation::Count,
            "min" => AggregateOperation::Min,
            "max" => AggregateOperation::Max,
            "avg" => AggregateOperation::Avg,
            other => panic!("Unsupported aggregate: {}", other),
        };
        self.agg(op_enum, col)
    }

    pub fn group_by(mut self, cols: Vec<String>) -> Self {
        self.group_by = cols;
        self
    }

    pub fn window(mut self, size: i64) -> Self {
        self.window = Some(size as usize);
        self
    }

    pub fn limit(mut self, n: i64) -> Self {
        self.limit = Some(n as usize);
        self
    }

    pub fn eq_int(col: &str, val: i64) -> FilterOperator {
        FilterOperator::Condition(Condition {
            column: col.into(),
            op: Operator::Eq,
            value: FilterValue::Int(val),
        })
    }

    pub fn gt_int(col: &str, val: i64) -> FilterOperator {
        FilterOperator::Condition(Condition {
            column: col.into(),
            op: Operator::Gt,
            value: FilterValue::Int(val),
        })
    }

    pub fn lt_int(col: &str, val: i64) -> FilterOperator {
        FilterOperator::Condition(Condition {
            column: col.into(),
            op: Operator::Lt,
            value: FilterValue::Int(val),
        })
    }

    pub fn like(col: &str, val: &str) -> FilterOperator {
        FilterOperator::Condition(Condition {
            column: col.into(),
            op: Operator::Like,
            value: FilterValue::String(val.to_string()),
        })
    }
}
