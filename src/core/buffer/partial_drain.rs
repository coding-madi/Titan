use crate::application::actors::broadcaster::broadcast_actor::RecordBatchWrapper;
use crate::core::buffer::drain_strategy::BufferDrain;
use dashmap::DashMap;

pub struct PartialDrain {
    pub limit: usize, // remove all data in excess of this limit
}

impl PartialDrain {
    pub fn batch_size(wrapper: &RecordBatchWrapper) -> usize {
        wrapper.get_data().get_array_memory_size() // approx size of each buffer
    }

    pub fn total_size(all_batches: &Vec<(String, RecordBatchWrapper)>) -> usize {
        all_batches
            .iter()
            .map(|(_, batch)| Self::batch_size(&batch))
            .sum()
    }
}

impl BufferDrain for PartialDrain {
    // TODO - consider VecDeq for avoiding shifting during drain
    fn drain(
        &self,
        buffer: &DashMap<String, Vec<RecordBatchWrapper>>,
        stream: &str,
    ) -> Vec<RecordBatchWrapper> {
        if let Some(mut entry) = buffer.get_mut(stream) {
            let n = self.limit.min(entry.len());
            entry.drain(..n).collect()
        } else {
            Vec::new()
        }
    }

    fn drain_all(
        &self,
        _buffer: &DashMap<String, Vec<RecordBatchWrapper>>,
    ) -> Vec<RecordBatchWrapper> {
        todo!()
    }
}
