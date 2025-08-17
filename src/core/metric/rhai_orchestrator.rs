use crate::application::actors::broadcast::RecordBatchWrapper;
use crate::core::metric::rhai_engine::RhaiEngine;
use crate::core::metric::rhai_executor::RhaiExecutor;


// Calls the query planner for plan generation from script
// Gets the Arrow buffers and vector of plans and executes them

pub struct Orchestrator {
    engine: RhaiEngine,
    executor: RhaiExecutor,
}

impl Orchestrator {
    pub fn new(engine: RhaiEngine, executor: RhaiExecutor) -> Self {
        Self {
            engine,
            executor,
        }
    }

    pub fn build_plan(&self, script: &str) {


    }

    pub fn append_log(&self, log: RecordBatchWrapper) {
        self.executor
    }
}