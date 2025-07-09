use std::sync::atomic::{AtomicU64, Ordering};

pub struct WalOffsetManager {
    pub current_offset: AtomicU64,
    pub max_file_size: u64,
}

impl WalOffsetManager {
    pub fn new(max_file_size: u64) -> Self {
        WalOffsetManager {
            current_offset: AtomicU64::new(0),
            max_file_size,
        }
    }

    pub fn get_current_offset(&self) -> u64 {
        self.current_offset
            .load(std::sync::atomic::Ordering::Relaxed)
    }

    pub fn update_offset(&self, offset: u64) {
        self.current_offset
            .store(offset, std::sync::atomic::Ordering::Relaxed);
    }

    pub fn reset_offset(&self) {
        self.current_offset
            .store(0, std::sync::atomic::Ordering::Relaxed);
    }

    pub fn is_max_size_reached(&self) -> bool {
        self.get_current_offset() >= self.max_file_size
    }

    pub fn increment_offset(&self, increment: u64) {
        let new_offset = self.get_current_offset().saturating_add(increment);
        self.update_offset(new_offset);
    }

    pub fn claim_offset(&self, buffer_size: u64) -> Result<u64, String> {
        loop {
            let current_offset = self.get_current_offset();
            let next = current_offset + buffer_size;

            if next > self.max_file_size {
                // If the next offset exceeds max file size, reset to 0
                self.reset_offset();
                return Err(format!(
                    "Max file size reached. Resetting offset to 0. Current offset: {current_offset}, next: {next}"
                ));
            }

            // Optimistic concurrency control: try to update the offset
            match self.current_offset.compare_exchange(
                current_offset,
                next,
                Ordering::AcqRel,
                Ordering::Acquire,
            ) {
                Ok(claimed_offset) => {
                    // Successfully updated the offset
                    tracing::info!(
                        "Offset claimed successfully. Current offset: {}, next: {}",
                        current_offset,
                        next
                    );
                    return Ok(claimed_offset);
                }
                Err(_) => {
                    // Failed to update, return an error
                    return Err(format!(
                        "Failed to claim offset. Current offset: {}, next: {}",
                        current_offset, next
                    ));
                }
            }
        }
    }
}
