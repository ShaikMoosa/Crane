use crane_application::{AppError, Command, Environment, ErrorCode, Library, LibraryRepository};
use crane_desktop::worker::{spawn_worker, WorkerRequest};
use crane_domain::Store;
use crane_platform::SystemEnvironment;
use crane_storage::JsonRepository;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, Instant};

#[derive(Default)]
struct GateState {
    permits: usize,
    active: usize,
    max_active: usize,
    commits: usize,
}

#[derive(Clone, Default)]
struct CommitGate(Arc<(Mutex<GateState>, Condvar)>);

impl CommitGate {
    fn wait_until_active(&self) {
        let deadline = Instant::now() + Duration::from_secs(5);
        let (lock, changed) = &*self.0;
        let mut state = lock.lock().unwrap();
        while state.active == 0 {
            let remaining = deadline.saturating_duration_since(Instant::now());
            assert!(!remaining.is_zero(), "worker never entered commit");
            (state, _) = changed.wait_timeout(state, remaining).unwrap();
        }
    }

    fn release(&self, count: usize) {
        let (lock, changed) = &*self.0;
        let mut state = lock.lock().unwrap();
        state.permits += count;
        changed.notify_all();
    }

    fn max_active(&self) -> usize {
        self.0 .0.lock().unwrap().max_active
    }
}

struct GateRepository {
    store: Store,
    gate: CommitGate,
}

impl LibraryRepository for GateRepository {
    fn load_or_seed(&mut self, _now: u64) -> Result<Store, AppError> {
        Ok(self.store.clone())
    }

    fn commit(&mut self, candidate: &Store) -> Result<(), AppError> {
        let (lock, changed) = &*self.gate.0;
        let mut state = lock.lock().unwrap();
        state.active += 1;
        state.max_active = state.max_active.max(state.active);
        changed.notify_all();
        while state.permits == 0 {
            state = changed.wait(state).unwrap();
        }
        state.permits -= 1;
        state.active -= 1;
        state.commits += 1;
        self.store = candidate.clone();
        changed.notify_all();
        Ok(())
    }
}

struct FakeEnvironment {
    next: u64,
}

impl Environment for FakeEnvironment {
    fn now_millis(&self) -> u64 {
        100
    }

    fn monotonic_millis(&self) -> u64 {
        100
    }

    fn new_note_id(&mut self) -> String {
        self.next += 1;
        format!("worker-note-{}", self.next)
    }
}

fn library(gate: CommitGate) -> Library {
    Library::open(
        Box::new(GateRepository {
            store: Store {
                schema_version: 1,
                notes: Vec::new(),
                extensions: Default::default(),
            },
            gate,
        }),
        Box::new(FakeEnvironment { next: 0 }),
    )
    .unwrap()
}

#[test]
fn full_channels_retain_accepted_work_without_blocking_the_gui() {
    let gate = CommitGate::default();
    let wakes = Arc::new(AtomicUsize::new(0));
    let wake_counter = wakes.clone();
    let worker = spawn_worker(
        library(gate.clone()),
        Box::new(move || {
            wake_counter.fetch_add(1, Ordering::SeqCst);
        }),
    );

    worker
        .try_send(WorkerRequest::command(1, Command::CreateNote))
        .unwrap();
    gate.wait_until_active();

    for request_id in 2..=9 {
        worker
            .try_send(WorkerRequest::command(request_id, Command::CreateNote))
            .expect("eight queued requests fit");
    }
    let started = Instant::now();
    let error = worker
        .try_send(WorkerRequest::command(10, Command::CreateNote))
        .expect_err("ninth queued request reports saturation");
    assert_eq!(error.code, ErrorCode::QueueFull);
    assert!(started.elapsed() < Duration::from_millis(100));

    gate.release(9);
    let deadline = Instant::now() + Duration::from_secs(10);
    let mut ids = Vec::new();
    while ids.len() < 9 && Instant::now() < deadline {
        if let Some(result) = worker.try_recv() {
            let crane_desktop::worker::WorkerOutcome::Command(outcome) = result.outcome else {
                panic!("expected command outcome");
            };
            assert!(outcome.is_ok());
            ids.push(result.request_id);
        } else {
            std::thread::sleep(Duration::from_millis(5));
        }
    }

    assert_eq!(ids, (1..=9).collect::<Vec<_>>());
    assert_eq!(gate.max_active(), 1);
    assert_eq!(wakes.load(Ordering::SeqCst), 9);
}

#[test]
fn worker_save_reopens_from_the_isolated_json_library() {
    let root = std::env::temp_dir().join(format!(
        "crane-native-worker-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir(&root).unwrap();
    let path = root.join("notes.json");
    let library = Library::open(
        Box::new(JsonRepository::new(path.clone())),
        Box::new(SystemEnvironment::new()),
    )
    .unwrap();
    let note_id = library.snapshot().store.notes[0].id.clone();
    let worker = spawn_worker(library, Box::new(|| {}));
    worker
        .try_send(WorkerRequest::command(
            42,
            Command::SaveNote {
                id: note_id.clone(),
                title: "Synthetic native title".into(),
                body: "Synthetic native body #native/check".into(),
                draft_revision: 7,
            },
        ))
        .unwrap();

    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        if let Some(result) = worker.try_recv() {
            assert_eq!(result.request_id, 42);
            let crane_desktop::worker::WorkerOutcome::Command(outcome) = result.outcome else {
                panic!("expected command outcome");
            };
            assert!(outcome.is_ok());
            break;
        }
        assert!(Instant::now() < deadline, "worker save timed out");
        std::thread::sleep(Duration::from_millis(5));
    }
    drop(worker);

    let reopened = Library::open(
        Box::new(JsonRepository::new(path)),
        Box::new(SystemEnvironment::new()),
    )
    .unwrap();
    let note = reopened
        .snapshot()
        .store
        .notes
        .into_iter()
        .find(|note| note.id == note_id)
        .unwrap();
    assert_eq!(note.title, "Synthetic native title");
    assert_eq!(note.body, "Synthetic native body #native/check");
    assert_eq!(note.tags, vec!["native/check"]);

    std::fs::remove_dir_all(root).unwrap();
}
