use crate::application::actors::broadcast::RecordBatchWrapper;

pub struct RhaiExecutor {
    record: Vec<RecordBatchWrapper>,
}

// Takes plan and engine and executes
// has logic for executing the plans

impl RhaiExecutor {
    pub fn new() -> Self {
        Self {
            record: vec![]
        }
    }

    pub fn append_logs(&mut self, buffer: RecordBatchWrapper) {
        self.record.push(buffer);
    }
}

