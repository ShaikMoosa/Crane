use crane_application::Environment;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

static NOTE_COUNTER: AtomicU64 = AtomicU64::new(0);

pub struct SystemEnvironment {
    started_at: Instant,
}

impl SystemEnvironment {
    pub fn new() -> Self {
        Self {
            started_at: Instant::now(),
        }
    }
}

impl Default for SystemEnvironment {
    fn default() -> Self {
        Self::new()
    }
}

impl Environment for SystemEnvironment {
    fn now_millis(&self) -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64
    }

    fn monotonic_millis(&self) -> u64 {
        self.started_at.elapsed().as_millis() as u64
    }

    fn new_note_id(&mut self) -> String {
        let sequence = NOTE_COUNTER.fetch_add(1, Ordering::Relaxed);
        format!("note-{}-{sequence}", self.now_millis())
    }
}
