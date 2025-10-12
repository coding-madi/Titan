use crate::application::actors::broadcaster::broadcast_actor::RecordBatchWrapper;
use crate::core::rhai::query_planner::QueryPlanner;
use crate::core::rhai::rhai_executor::RhaiExecutor;
use crate::core::rhai::rhai_parser::RhaiParser;
use arrow_array::RecordBatch;
use arrow_schema::ArrowError;
use std::sync::Arc;
use tracing::error;
// Calls the query planner for plan generation from script
// Gets the Arrow buffers and vector of plans and executes them

pub struct RhaiService {
    parser: RhaiParser,
    executor: RhaiExecutor,
}

impl RhaiService {
    pub fn new(parser: RhaiParser, executor: RhaiExecutor) -> Self {
        Self { parser, executor }
    }

    pub fn build_plan(&self, script: &str) -> Vec<QueryPlanner> {
        self.parser.parse(script).unwrap_or_else(|e| {
            error!("Fatal error in Rhai script: {}", e);
            vec![]
        })
    }

    pub fn append_log(&mut self, log: RecordBatchWrapper) {
        self.executor.append_logs(log);
    }

    pub fn evaluate(
        &self,
        query_planner: QueryPlanner,
        log: RecordBatchWrapper,
    ) -> Result<Arc<RecordBatch>, ArrowError> {
        RhaiExecutor::apply(&vec![log], query_planner)
    }
}
