mod support;

use crane_application::{
    AppError, Command, CommandData, Environment, ErrorCode, ExportSummary, Library, Outcome,
    TransferIo,
};
use crane_domain::{
    make_backup, serialize_backup, BackupInspection, NamedBytes, Note, Store, MAX_BATCH_BYTES,
    MAX_BODY_BYTES, MAX_TITLE_BYTES,
};
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use support::FakeRepository;

#[derive(Clone)]
struct ControlledEnvironment {
    state: Arc<Mutex<EnvironmentState>>,
}

struct EnvironmentState {
    now: u64,
    monotonic: u64,
    next_id: u64,
}

impl ControlledEnvironment {
    fn new(now: u64, monotonic: u64) -> Self {
        Self {
            state: Arc::new(Mutex::new(EnvironmentState {
                now,
                monotonic,
                next_id: 1,
            })),
        }
    }

    fn set_monotonic(&self, value: u64) {
        self.state.lock().unwrap().monotonic = value;
    }
}

impl Environment for ControlledEnvironment {
    fn now_millis(&self) -> u64 {
        self.state.lock().unwrap().now
    }

    fn monotonic_millis(&self) -> u64 {
        self.state.lock().unwrap().monotonic
    }

    fn new_note_id(&mut self) -> String {
        let mut state = self.state.lock().unwrap();
        let id = format!("generated-{}", state.next_id);
        state.next_id += 1;
        id
    }
}

#[derive(Clone, Default)]
struct FakeTransfers {
    state: Arc<Mutex<TransferState>>,
}

#[derive(Default)]
struct TransferState {
    markdown: VecDeque<Outcome<Vec<NamedBytes>>>,
    backups: VecDeque<Outcome<Vec<u8>>>,
    safety_writes: usize,
    fail_safety: bool,
}

impl FakeTransfers {
    fn with_markdown(files: Vec<NamedBytes>) -> Self {
        let value = Self::default();
        value
            .state
            .lock()
            .unwrap()
            .markdown
            .push_back(Outcome::Completed(files));
        value
    }

    fn with_backups(backups: Vec<Vec<u8>>) -> Self {
        let value = Self::default();
        value
            .state
            .lock()
            .unwrap()
            .backups
            .extend(backups.into_iter().map(Outcome::Completed));
        value
    }

    fn fail_safety(&self) {
        self.state.lock().unwrap().fail_safety = true;
    }

    fn safety_writes(&self) -> usize {
        self.state.lock().unwrap().safety_writes
    }
}

impl TransferIo for FakeTransfers {
    fn pick_markdown(&mut self) -> Result<Outcome<Vec<NamedBytes>>, AppError> {
        Ok(self
            .state
            .lock()
            .unwrap()
            .markdown
            .pop_front()
            .unwrap_or(Outcome::Cancelled))
    }

    fn export_note(&mut self, _note: &Note) -> Result<Outcome<usize>, AppError> {
        Ok(Outcome::Cancelled)
    }

    fn export_library(&mut self, _store: &Store) -> Result<Outcome<ExportSummary>, AppError> {
        Ok(Outcome::Cancelled)
    }

    fn create_backup(
        &mut self,
        _store: &Store,
        _created_at: u64,
    ) -> Result<Outcome<usize>, AppError> {
        Ok(Outcome::Cancelled)
    }

    fn pick_backup(&mut self) -> Result<Outcome<Vec<u8>>, AppError> {
        Ok(self
            .state
            .lock()
            .unwrap()
            .backups
            .pop_front()
            .unwrap_or(Outcome::Cancelled))
    }

    fn write_safety_backup(&mut self, _store: &Store, _created_at: u64) -> Result<(), AppError> {
        let mut state = self.state.lock().unwrap();
        state.safety_writes += 1;
        if state.fail_safety {
            return Err(AppError::new(ErrorCode::Io, "injected safety failure"));
        }
        Ok(())
    }
}

fn empty_store() -> Store {
    Store {
        schema_version: crane_domain::SCHEMA_VERSION,
        notes: Vec::new(),
        extensions: Default::default(),
    }
}

fn open(
    store: Store,
    environment: ControlledEnvironment,
    transfers: FakeTransfers,
) -> (Library, FakeRepository) {
    let repository = FakeRepository::new(store);
    let library = Library::open(Box::new(repository.clone()), Box::new(environment))
        .unwrap()
        .with_transfers(Box::new(transfers));
    (library, repository)
}

fn backup(store: &Store) -> Vec<u8> {
    let envelope = make_backup(store, 20, "test").unwrap();
    serialize_backup(&envelope).unwrap()
}

fn inspect_token(library: &mut Library) -> String {
    let result = library.execute(Command::InspectBackup).unwrap();
    let CommandData::Inspection { token, .. } = result.data else {
        panic!("inspection must return a token");
    };
    token
}

#[test]
fn failed_import_adds_zero_notes() {
    let cases = vec![
        NamedBytes {
            name: "bad.md".into(),
            bytes: vec![0xff],
        },
        NamedBytes {
            name: "bad.txt".into(),
            bytes: b"text".to_vec(),
        },
        NamedBytes {
            name: "title.md".into(),
            bytes: format!("# {}", "x".repeat(MAX_TITLE_BYTES + 1)).into_bytes(),
        },
        NamedBytes {
            name: "body.md".into(),
            bytes: vec![b'x'; MAX_BODY_BYTES + 1],
        },
        NamedBytes {
            name: "batch.md".into(),
            bytes: vec![b'x'; MAX_BATCH_BYTES + 1],
        },
    ];

    for file in cases {
        let transfers = FakeTransfers::with_markdown(vec![file]);
        let (mut library, repository) =
            open(empty_store(), ControlledEnvironment::new(10, 0), transfers);
        assert!(library.execute(Command::ImportMarkdown).is_err());
        assert!(library.snapshot().store.notes.is_empty());
        assert_eq!(repository.commit_count(), 0);
        let disk: Store = serde_json::from_slice(&repository.disk_bytes()).unwrap();
        assert!(disk.notes.is_empty());
    }
}

#[test]
fn restore_requires_fresh_unrevoked_confirmation() {
    let replacement = empty_store();
    let bytes = backup(&replacement);

    let (mut cancelled, _) = open(
        Store::seed(10),
        ControlledEnvironment::new(100, 0),
        FakeTransfers::with_backups(vec![bytes.clone()]),
    );
    let cancelled_token = inspect_token(&mut cancelled);
    assert!(matches!(
        cancelled
            .execute(Command::CancelRestore {
                token: cancelled_token.clone()
            })
            .unwrap()
            .data,
        CommandData::RestoreCancelled
    ));
    assert!(matches!(
        cancelled
            .execute(Command::RestoreBackup {
                token: cancelled_token
            })
            .unwrap_err()
            .code,
        ErrorCode::StaleConfirmation
    ));

    let (mut superseded, _) = open(
        Store::seed(10),
        ControlledEnvironment::new(100, 0),
        FakeTransfers::with_backups(vec![bytes.clone(), bytes.clone()]),
    );
    let old_token = inspect_token(&mut superseded);
    let _new_token = inspect_token(&mut superseded);
    assert_eq!(
        superseded
            .execute(Command::RestoreBackup { token: old_token })
            .unwrap_err()
            .code,
        ErrorCode::StaleConfirmation
    );

    let (mut changed, _) = open(
        Store::seed(10),
        ControlledEnvironment::new(100, 0),
        FakeTransfers::with_backups(vec![bytes.clone()]),
    );
    let changed_token = inspect_token(&mut changed);
    changed.execute(Command::CreateNote).unwrap();
    assert_eq!(
        changed
            .execute(Command::RestoreBackup {
                token: changed_token
            })
            .unwrap_err()
            .code,
        ErrorCode::StaleConfirmation
    );

    let (mut reused, _) = open(
        Store::seed(10),
        ControlledEnvironment::new(100, 0),
        FakeTransfers::with_backups(vec![bytes.clone()]),
    );
    let reuse_token = inspect_token(&mut reused);
    reused
        .execute(Command::RestoreBackup {
            token: reuse_token.clone(),
        })
        .unwrap();
    assert_eq!(
        reused
            .execute(Command::RestoreBackup { token: reuse_token })
            .unwrap_err()
            .code,
        ErrorCode::StaleConfirmation
    );

    let environment = ControlledEnvironment::new(100, 0);
    let (mut expired, _) = open(
        Store::seed(10),
        environment.clone(),
        FakeTransfers::with_backups(vec![bytes]),
    );
    let expired_token = inspect_token(&mut expired);
    environment.set_monotonic(300_000);
    assert_eq!(
        expired
            .execute(Command::RestoreBackup {
                token: expired_token
            })
            .unwrap_err()
            .code,
        ErrorCode::StaleConfirmation
    );
}

#[test]
fn safety_failure_skips_replacement() {
    let original = Store::seed(10);
    let replacement = empty_store();
    let transfers = FakeTransfers::with_backups(vec![backup(&replacement)]);
    transfers.fail_safety();
    let (mut library, repository) = open(
        original.clone(),
        ControlledEnvironment::new(100, 0),
        transfers.clone(),
    );
    let token = inspect_token(&mut library);

    assert!(library.execute(Command::RestoreBackup { token }).is_err());
    assert_eq!(transfers.safety_writes(), 1);
    assert_eq!(repository.commit_count(), 0);
    assert_eq!(library.snapshot().store.notes.len(), original.notes.len());
}

#[test]
fn replacement_failure_keeps_memory_and_disk() {
    let original = Store::seed(10);
    let original_disk = serde_json::to_vec(&original).unwrap();
    let replacement = empty_store();
    let transfers = FakeTransfers::with_backups(vec![backup(&replacement)]);
    let (mut library, repository) = open(
        original.clone(),
        ControlledEnvironment::new(100, 0),
        transfers.clone(),
    );
    let token = inspect_token(&mut library);
    repository.fail_next_commit();

    assert!(library.execute(Command::RestoreBackup { token }).is_err());
    assert_eq!(transfers.safety_writes(), 1);
    assert_eq!(library.snapshot().store.notes.len(), original.notes.len());
    assert_eq!(repository.disk_bytes(), original_disk);
}

#[test]
fn transfer_commands_require_an_adapter() {
    let repository = FakeRepository::new(empty_store());
    let mut library = Library::open(
        Box::new(repository),
        Box::new(ControlledEnvironment::new(10, 0)),
    )
    .unwrap();
    assert_eq!(
        library.execute(Command::ImportMarkdown).unwrap_err().code,
        ErrorCode::Busy
    );
}

#[test]
fn inspection_reports_domain_counts() {
    let replacement = Store::seed(10);
    let transfers = FakeTransfers::with_backups(vec![backup(&replacement)]);
    let (mut library, _) = open(empty_store(), ControlledEnvironment::new(100, 0), transfers);
    let result = library.execute(Command::InspectBackup).unwrap();
    let CommandData::Inspection { inspection, .. } = result.data else {
        panic!("expected inspection");
    };
    assert_eq!(
        inspection,
        BackupInspection {
            created_at: 20,
            total: 3,
            active: 3,
            archived: 0,
            trashed: 0,
        }
    );
}
