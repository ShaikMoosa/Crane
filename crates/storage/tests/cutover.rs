use crane_application::{AppError, Environment, ErrorCode, ExportSummary, Outcome, TransferIo};
use crane_domain::{NamedBytes, Note, Store};
use crane_platform::LegacyProcessStatus;
use crane_storage::{prepare_cutover_with_status, JsonRepository};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

struct TestDirectory(PathBuf);

impl TestDirectory {
    fn new(label: &str) -> Self {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::current_dir()
            .unwrap()
            .join("target")
            .join(format!(
                "crane-cutover-{label}-{}-{nonce}",
                std::process::id()
            ));
        fs::create_dir_all(&path).unwrap();
        Self(path)
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TestDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

struct FixedEnvironment {
    now: u64,
}

impl Environment for FixedEnvironment {
    fn now_millis(&self) -> u64 {
        self.now
    }

    fn monotonic_millis(&self) -> u64 {
        0
    }

    fn new_note_id(&mut self) -> String {
        "unused-cutover-id".into()
    }
}

#[derive(Clone)]
struct RecordingTransfers {
    state: Arc<Mutex<TransferState>>,
}

struct TransferState {
    backup_calls: usize,
    next_result: Result<String, AppError>,
}

impl RecordingTransfers {
    fn succeeds(file_name: &str) -> Self {
        Self {
            state: Arc::new(Mutex::new(TransferState {
                backup_calls: 0,
                next_result: Ok(file_name.into()),
            })),
        }
    }

    fn fails(code: ErrorCode, message: &str) -> Self {
        Self {
            state: Arc::new(Mutex::new(TransferState {
                backup_calls: 0,
                next_result: Err(AppError::new(code, message)),
            })),
        }
    }

    fn backup_calls(&self) -> usize {
        self.state.lock().unwrap().backup_calls
    }
}

impl TransferIo for RecordingTransfers {
    fn pick_markdown(&mut self) -> Result<Outcome<Vec<NamedBytes>>, AppError> {
        unreachable!()
    }

    fn export_note(&mut self, _note: &Note) -> Result<Outcome<usize>, AppError> {
        unreachable!()
    }

    fn export_library(&mut self, _store: &Store) -> Result<Outcome<ExportSummary>, AppError> {
        unreachable!()
    }

    fn create_backup(
        &mut self,
        _store: &Store,
        _created_at: u64,
    ) -> Result<Outcome<usize>, AppError> {
        unreachable!()
    }

    fn pick_backup(&mut self) -> Result<Outcome<Vec<u8>>, AppError> {
        unreachable!()
    }

    fn write_safety_backup(&mut self, _store: &Store, _created_at: u64) -> Result<(), AppError> {
        unreachable!()
    }

    fn write_pre_migration_backup(
        &mut self,
        _store: &Store,
        _created_at: u64,
    ) -> Result<String, AppError> {
        let mut state = self.state.lock().unwrap();
        state.backup_calls += 1;
        state.next_result.clone()
    }
}

fn write_store(root: &Path, store: &Store) -> Vec<u8> {
    let bytes = serde_json::to_vec_pretty(store).unwrap();
    fs::write(root.join("notes.json"), &bytes).unwrap();
    bytes
}

#[test]
fn cutover_refuses_running_or_unknown_legacy_process_without_writing() {
    for status in [LegacyProcessStatus::Running, LegacyProcessStatus::Unknown] {
        let directory = TestDirectory::new("legacy-process");
        let original = write_store(directory.path(), &Store::seed(10));
        let mut repository = JsonRepository::new(directory.path().join("notes.json"));
        let mut transfers = RecordingTransfers::succeeds("pre-migration.crane-backup.json");
        let mut environment = FixedEnvironment { now: 20 };

        let error =
            prepare_cutover_with_status(&mut repository, &mut transfers, &mut environment, status)
                .expect_err("a possible legacy writer must block cutover");

        assert_eq!(error.code, ErrorCode::Busy);
        assert_eq!(transfers.backup_calls(), 0);
        assert_eq!(
            fs::read(directory.path().join("notes.json")).unwrap(),
            original
        );
        assert!(!directory.path().join("native-migration-v1.json").exists());
    }
}

#[test]
fn failed_or_oversized_pre_migration_backup_causes_no_write() {
    for (code, message) in [
        (ErrorCode::Io, "injected backup write failure"),
        (ErrorCode::Validation, "backup exceeds the 256 MiB limit"),
    ] {
        let directory = TestDirectory::new("backup-failure");
        let original = write_store(directory.path(), &Store::seed(10));
        let mut repository = JsonRepository::new(directory.path().join("notes.json"));
        let mut transfers = RecordingTransfers::fails(code, message);
        let mut environment = FixedEnvironment { now: 20 };

        let error = prepare_cutover_with_status(
            &mut repository,
            &mut transfers,
            &mut environment,
            LegacyProcessStatus::Absent,
        )
        .expect_err("backup failure must stop cutover");

        assert_eq!(error.code, code);
        assert_eq!(transfers.backup_calls(), 1);
        assert_eq!(
            fs::read(directory.path().join("notes.json")).unwrap(),
            original
        );
        assert!(!directory.path().join("native-migration-v1.json").exists());
    }
}

#[test]
fn repeat_startup_reuses_receipt_and_preserves_v1_data() {
    let directory = TestDirectory::new("repeat");
    let mut store = Store::seed(10);
    store.notes[0].title = "Unicode — नमस्ते".into();
    let original = write_store(directory.path(), &store);
    let mut repository = JsonRepository::new(directory.path().join("notes.json"));
    let mut transfers = RecordingTransfers::succeeds("pre-migration-20.crane-backup.json");
    let mut environment = FixedEnvironment { now: 20 };

    let first = prepare_cutover_with_status(
        &mut repository,
        &mut transfers,
        &mut environment,
        LegacyProcessStatus::Absent,
    )
    .unwrap();
    let second = prepare_cutover_with_status(
        &mut repository,
        &mut transfers,
        &mut environment,
        LegacyProcessStatus::Absent,
    )
    .unwrap();

    assert_eq!(first, second);
    assert_eq!(first.note_count, store.notes.len());
    assert_eq!(first.backup_file_name, "pre-migration-20.crane-backup.json");
    assert_eq!(transfers.backup_calls(), 1);
    assert_eq!(
        fs::read(directory.path().join("notes.json")).unwrap(),
        original
    );
    let legacy_readable: Store = serde_json::from_slice(&original).unwrap();
    assert_eq!(
        serde_json::to_value(legacy_readable).unwrap(),
        serde_json::to_value(store).unwrap()
    );
}

#[test]
fn startup_distinguishes_absent_empty_corrupt_inaccessible_and_future_data() {
    let directory = TestDirectory::new("classification");
    let absent_path = directory.path().join("absent.json");
    assert!(JsonRepository::new(absent_path.clone())
        .load_existing()
        .unwrap()
        .is_none());
    assert!(!absent_path.exists());

    let empty_path = directory.path().join("empty.json");
    fs::write(&empty_path, br#"{"schemaVersion":1,"notes":[]}"#).unwrap();
    assert!(JsonRepository::new(empty_path)
        .load_existing()
        .unwrap()
        .unwrap()
        .notes
        .is_empty());

    let corrupt_path = directory.path().join("corrupt.json");
    fs::write(&corrupt_path, b"not json").unwrap();
    assert!(JsonRepository::new(corrupt_path).load_existing().is_err());

    let inaccessible_path = directory.path().join("directory.json");
    fs::create_dir(&inaccessible_path).unwrap();
    assert!(JsonRepository::new(inaccessible_path)
        .load_existing()
        .is_err());

    let future_path = directory.path().join("future.json");
    fs::write(&future_path, br#"{"schemaVersion":2,"notes":[]}"#).unwrap();
    assert!(JsonRepository::new(future_path).load_existing().is_err());
}

#[test]
fn cutover_preserves_notes_backups_and_settings_for_reinstall() {
    let directory = TestDirectory::new("preserve-data");
    let original_notes = write_store(directory.path(), &Store::seed(10));
    fs::create_dir(directory.path().join("backups")).unwrap();
    fs::write(directory.path().join("backups/keep.json"), b"backup").unwrap();
    fs::write(directory.path().join("settings.json"), b"settings").unwrap();
    let mut repository = JsonRepository::new(directory.path().join("notes.json"));
    let mut transfers = RecordingTransfers::succeeds("pre-migration-20.crane-backup.json");
    let mut environment = FixedEnvironment { now: 20 };

    prepare_cutover_with_status(
        &mut repository,
        &mut transfers,
        &mut environment,
        LegacyProcessStatus::Absent,
    )
    .unwrap();

    assert_eq!(
        fs::read(directory.path().join("notes.json")).unwrap(),
        original_notes
    );
    assert_eq!(
        fs::read(directory.path().join("backups/keep.json")).unwrap(),
        b"backup"
    );
    assert_eq!(
        fs::read(directory.path().join("settings.json")).unwrap(),
        b"settings"
    );
}
