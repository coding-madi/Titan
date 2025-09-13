use crate::application::actors::broadcast_actor::RecordBatchWrapper;
use dashmap::DashMap;

pub trait BufferDrain {
    fn drain(
        &self,
        buffer: &DashMap<String, Vec<RecordBatchWrapper>>,
        stream: &str,
    ) -> Vec<RecordBatchWrapper>;

    fn drain_all(
        &self,
        buffer: &DashMap<String, Vec<RecordBatchWrapper>>,
    ) -> Vec<RecordBatchWrapper>;
}
