use crate::application::actors::broadcast::RecordBatchWrapper;
use crate::core::buffer::drain_strategy::BufferDrain;
use arrow::compute::concat_batches;
use arrow_array::RecordBatch;
use dashmap::DashMap;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::mpsc::{Receiver, Sender};

#[derive(Clone)]
pub struct BufferManager {
    pub buffer: Arc<DashMap<String, Vec<RecordBatchWrapper>>>,
    pub channels: Arc<DashMap<String, Sender<RecordBatchWrapper>>>,
    pub drain_strategy: Arc<dyn BufferDrain + Send + Sync>,
}

impl BufferManager {
    pub fn new(drain_strategy: Arc<dyn BufferDrain + Send + Sync>) -> Self {
        Self {
            buffer: Arc::new(DashMap::new()),
            channels: Arc::new(DashMap::new()),
            drain_strategy,
        }
    }

    pub fn get(&self, stream: &str) -> Option<Vec<RecordBatchWrapper>> {
        self.buffer.get(stream).map(|v| v.to_owned())
    }

    pub fn get_or_create_sender(&self, flight: &str) -> Sender<RecordBatchWrapper> {
        // Return channel if exists or create new
        self.channels
            .entry(flight.to_string())
            .or_insert_with(|| {
                let (tx, rx) = tokio::sync::mpsc::channel(1024);
                self.spawn_consumer(rx); // controlled internally
                tx
            })
            .clone()
    }

    pub fn drain(&self, stream: &str) -> Vec<RecordBatchWrapper> {
        self.drain_strategy.drain(&self.buffer, stream)
    }

    pub fn drain_all(&self) -> Vec<RecordBatchWrapper> {
        self.drain_strategy.drain_all(&self.buffer)
    }

    // Will continuously drain the data from a channel into the memory buffer
    fn spawn_consumer(&self, mut rx: Receiver<RecordBatchWrapper>) {
        let buffer = self.buffer.clone();
        tokio::spawn(async move {
            while let Some(el) = rx.recv().await {
                let flight = el.metadata.flight.clone();
                buffer
                    .entry(flight)
                    .or_insert_with(|| Vec::with_capacity(64))
                    .push(el);
            }
        });
    }

    // Estimate the size of the arrow buffer
    fn batch_size(batch: &RecordBatch) -> usize {
        batch
            .columns()
            .iter()
            .map(|array| array.get_array_memory_size())
            .sum()
    }
}
