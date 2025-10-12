use crate::application::actors::broadcaster::broadcast_actor::RecordBatchWrapper;
use crate::core::buffer::drain_strategy::BufferDrain;
use dashmap::DashMap;

pub struct FullDrain;
impl BufferDrain for FullDrain {
    fn drain(
        &self,
        buffer: &DashMap<String, Vec<RecordBatchWrapper>>,
        stream: &str,
    ) -> Vec<RecordBatchWrapper> {
        if let Some(mut entry) = buffer.get_mut(stream) {
            std::mem::take(&mut *entry)
        } else {
            Vec::new()
        }
    }

    fn drain_all(
        &self,
        buffer: &DashMap<String, Vec<RecordBatchWrapper>>,
    ) -> Vec<RecordBatchWrapper> {
        let mut all_batches = Vec::new();

        // Iterate over all keys and drain each stream
        for mut entry in buffer.iter_mut() {
            all_batches.append(&mut std::mem::take(&mut *entry.value_mut()));
        }

        all_batches
    }
}
