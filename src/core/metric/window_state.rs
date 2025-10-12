use crate::application::actors::broadcaster::broadcast_actor::RecordBatchWrapper;
use std::collections::{BTreeMap, HashSet};

pub struct WindowState {
    // window state keyed by window_start (nanoseconds epoch)
    pub window_state: BTreeMap<i64, Vec<RecordBatchWrapper>>,
    // current event-time watermark (nanoseconds epoch)
    pub watermark: i64,
    // window size in seconds
    pub window_size_sec: i64,
    // grace period in seconds before firing a window after seeing next-window events
    pub grace_period_sec: i64,
    // tracks which window purge timers have been scheduled to avoid duplicate timers
    pub scheduled_windows: HashSet<i64>,
}

impl WindowState {
    pub fn new(window_size_sec: i64, grace_period_sec: i64) -> Self {
        Self {
            window_state: BTreeMap::new(),
            watermark: 0,
            window_size_sec,
            grace_period_sec,
            scheduled_windows: HashSet::new(),
        }
    }
}
