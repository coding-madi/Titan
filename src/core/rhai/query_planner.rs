use crate::application::actors::broadcast_actor::RecordBatchWrapper;
use arrow::compute;
use arrow::compute::filter_record_batch;
use arrow::compute::kernels::{cmp, comparison};
use arrow_array::builder::{Int64Builder, StringBuilder};
use arrow_array::{Array, ArrayRef, BooleanArray, Int64Array, RecordBatch, StringArray};
use arrow_schema::{ArrowError, DataType};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::error;

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

#[derive(Debug, Clone, PartialEq)]
pub enum FilterValue {
    Int(i64),
    String(String),
}

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
pub enum Filter {
    // A single condition
    Condition(Condition),
    And(Vec<Filter>),
    Or(Vec<Filter>),
}

#[derive(Debug, Clone)]
pub struct QueryPlanner {
    pub filter: Option<Filter>,
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

/// Create binding functions for DSL
impl QueryPlanner {
    pub fn filter(mut self, f: Filter) -> Self {
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

    pub fn eq_int(col: &str, val: i64) -> Filter {
        Filter::Condition(Condition {
            column: col.into(),
            op: Operator::Eq,
            value: FilterValue::Int(val),
        })
    }

    pub fn gt_int(col: &str, val: i64) -> Filter {
        Filter::Condition(Condition {
            column: col.into(),
            op: Operator::Gt,
            value: FilterValue::Int(val),
        })
    }

    pub fn lt_int(col: &str, val: i64) -> Filter {
        Filter::Condition(Condition {
            column: col.into(),
            op: Operator::Lt,
            value: FilterValue::Int(val),
        })
    }

    pub fn like(col: &str, val: &str) -> Filter {
        Filter::Condition(Condition {
            column: col.into(),
            op: Operator::Like,
            value: FilterValue::String(val.to_string()),
        })
    }
}
