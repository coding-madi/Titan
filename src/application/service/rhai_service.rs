use crate::application::actors::broadcast_actor::RecordBatchWrapper;
use crate::core::rhai::rhai_engine::RhaiEngine;
use crate::core::rhai::rhai_executor::RhaiExecutor;

// Calls the query planner for plan generation from script
// Gets the Arrow buffers and vector of plans and executes them

pub struct RhaiService {
    engine: RhaiEngine,
    executor: RhaiExecutor,
}

impl RhaiService {
    pub fn new(engine: RhaiEngine, executor: RhaiExecutor) -> Self {
        Self { engine, executor }
    }

    pub fn build_plan(&self, script: &str) {}

    pub fn append_log(&mut self, log: RecordBatchWrapper) {
        self.executor.append_logs(log);
    }
}
