use crane_application::{AppError, Environment, Library, LibraryRepository};
use crane_desktop::app::CraneApp;
use crane_domain::{Note, Store};
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct SharedRepository(pub Arc<Mutex<Store>>);

impl LibraryRepository for SharedRepository {
    fn load_or_seed(&mut self, _now: u64) -> Result<Store, AppError> {
        Ok(self.0.lock().unwrap().clone())
    }

    fn commit(&mut self, candidate: &Store) -> Result<(), AppError> {
        *self.0.lock().unwrap() = candidate.clone();
        Ok(())
    }
}

struct TestEnvironment {
    now: u64,
    next_id: u64,
}

impl Environment for TestEnvironment {
    fn now_millis(&self) -> u64 {
        self.now
    }

    fn monotonic_millis(&self) -> u64 {
        self.now
    }

    fn new_note_id(&mut self) -> String {
        self.next_id += 1;
        format!("created-{}", self.next_id)
    }
}

pub fn note(id: &str, title: &str, body: &str, updated_at: u64) -> Note {
    Note {
        id: id.into(),
        title: title.into(),
        body: body.into(),
        tags: crane_domain::extract_tags(body),
        created_at: 1,
        updated_at,
        pinned: false,
        archived: false,
        trashed: false,
        extensions: Default::default(),
    }
}

pub fn app(notes: Vec<Note>) -> (CraneApp, Arc<Mutex<Store>>) {
    let shared = Arc::new(Mutex::new(Store {
        schema_version: 1,
        notes,
        extensions: Default::default(),
    }));
    let library = Library::open(
        Box::new(SharedRepository(shared.clone())),
        Box::new(TestEnvironment {
            now: 10_000,
            next_id: 0,
        }),
    )
    .unwrap();
    let snapshot = library.snapshot();
    (
        CraneApp::new_with_wake(library, snapshot, Box::new(|| {})),
        shared,
    )
}
