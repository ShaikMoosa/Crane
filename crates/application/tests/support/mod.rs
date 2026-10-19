use crane_application::{AppError, Environment, ErrorCode, LibraryRepository};
use crane_domain::Store;
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct FakeRepository {
    state: Arc<Mutex<RepositoryState>>,
}

struct RepositoryState {
    store: Store,
    disk_bytes: Vec<u8>,
    commits: usize,
    fail_next: bool,
}

impl FakeRepository {
    pub fn new(store: Store) -> Self {
        let disk_bytes = serde_json::to_vec(&store).unwrap();
        Self {
            state: Arc::new(Mutex::new(RepositoryState {
                store,
                disk_bytes,
                commits: 0,
                fail_next: false,
            })),
        }
    }

    pub fn disk_bytes(&self) -> Vec<u8> {
        self.state.lock().unwrap().disk_bytes.clone()
    }

    pub fn commit_count(&self) -> usize {
        self.state.lock().unwrap().commits
    }

    pub fn fail_next_commit(&self) {
        self.state.lock().unwrap().fail_next = true;
    }
}

impl LibraryRepository for FakeRepository {
    fn load_or_seed(&mut self, _now: u64) -> Result<Store, AppError> {
        Ok(self.state.lock().unwrap().store.clone())
    }

    fn commit(&mut self, candidate: &Store) -> Result<(), AppError> {
        let mut state = self.state.lock().unwrap();
        if state.fail_next {
            state.fail_next = false;
            return Err(AppError::new(ErrorCode::Io, "injected commit failure"));
        }
        state.commits += 1;
        state.store = candidate.clone();
        state.disk_bytes = serde_json::to_vec(candidate).unwrap();
        Ok(())
    }
}

pub struct FakeEnvironment {
    now_ms: u64,
    monotonic_ms: u64,
    next_id: u64,
}

impl FakeEnvironment {
    pub fn new(now_ms: u64, monotonic_ms: u64) -> Self {
        Self {
            now_ms,
            monotonic_ms,
            next_id: 1,
        }
    }
}

impl Environment for FakeEnvironment {
    fn now_millis(&self) -> u64 {
        self.now_ms
    }

    fn monotonic_millis(&self) -> u64 {
        self.monotonic_ms
    }

    fn new_note_id(&mut self) -> String {
        let id = format!("fake-note-{}", self.next_id);
        self.next_id += 1;
        id
    }
}
