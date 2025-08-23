use crate::application::actors::broadcast::RecordBatchWrapper;

pub struct RhaiExecutor {
    flight_name: String,
    record: Vec<RecordBatchWrapper>,
}

// Takes plan and engine and executes
// has logic for executing the plans

impl RhaiExecutor {
    pub fn new(flight_name: String) -> Self {
        Self {
            flight_name,
            record: vec![],
        }
    }

    pub fn append_logs(&mut self, buffer: RecordBatchWrapper) {
        self.record.push(buffer);
    }
}
