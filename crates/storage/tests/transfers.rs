use crane_application::{Outcome, TransferIo};
use crane_domain::{build_export_manifest, make_backup, Store};
use crane_platform::{DialogProvider, DialogSelection, PlatformError};
use crane_storage::{write_backup_with_limit, write_library_export_with, FilesystemTransfers};
use std::fs::OpenOptions;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

struct TestDirectory(PathBuf);

impl TestDirectory {
    fn new(label: &str) -> Self {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::current_dir()
            .unwrap()
            .join("target")
            .join(format!(
                "crane-transfer-{label}-{}-{nonce}",
                std::process::id()
            ));
        std::fs::create_dir_all(&path).unwrap();
        Self(path)
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TestDirectory {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[derive(Clone)]
struct FakeDialogs {
    state: Arc<Mutex<DialogState>>,
}

struct DialogState {
    note_path: Option<PathBuf>,
    backup_path: Option<PathBuf>,
    confirm: bool,
    confirmed_path: Option<PathBuf>,
}

impl FakeDialogs {
    fn backup(path: PathBuf, confirm: bool) -> Self {
        Self {
            state: Arc::new(Mutex::new(DialogState {
                note_path: None,
                backup_path: Some(path),
                confirm,
                confirmed_path: None,
            })),
        }
    }

    fn note(path: PathBuf, confirm: bool) -> Self {
        Self {
            state: Arc::new(Mutex::new(DialogState {
                note_path: Some(path),
                backup_path: None,
                confirm,
                confirmed_path: None,
            })),
        }
    }

    fn confirmed_path(&self) -> Option<PathBuf> {
        self.state.lock().unwrap().confirmed_path.clone()
    }
}

impl DialogProvider for FakeDialogs {
    fn pick_markdown(&mut self) -> Result<DialogSelection<Vec<PathBuf>>, PlatformError> {
        Ok(DialogSelection::Cancelled)
    }

    fn save_note(
        &mut self,
        _suggested_name: &str,
    ) -> Result<DialogSelection<PathBuf>, PlatformError> {
        let path = self.state.lock().unwrap().note_path.take();
        Ok(path.map_or(DialogSelection::Cancelled, DialogSelection::Selected))
    }

    fn pick_export_folder(&mut self) -> Result<DialogSelection<PathBuf>, PlatformError> {
        Ok(DialogSelection::Cancelled)
    }

    fn save_backup(
        &mut self,
        _suggested_name: &str,
    ) -> Result<DialogSelection<PathBuf>, PlatformError> {
        let path = self.state.lock().unwrap().backup_path.take();
        Ok(path.map_or(DialogSelection::Cancelled, DialogSelection::Selected))
    }

    fn confirm_replace(&mut self, path: &Path) -> Result<bool, PlatformError> {
        let mut state = self.state.lock().unwrap();
        state.confirmed_path = Some(path.to_path_buf());
        Ok(state.confirm)
    }

    fn pick_backup(&mut self) -> Result<DialogSelection<PathBuf>, PlatformError> {
        Ok(DialogSelection::Cancelled)
    }
}

fn write_new(path: &Path, contents: &[u8]) -> io::Result<()> {
    let mut file = OpenOptions::new().write(true).create_new(true).open(path)?;
    file.write_all(contents)?;
    file.sync_all()
}

#[test]
fn windows_compatible_export_names_create_distinct_files() {
    let directory = TestDirectory::new("names");
    let titles = ["Plan", "plan", "Σ", "ς", "COM¹", "LPT³"];
    let mut store = Store::seed(100);
    store.notes = titles
        .into_iter()
        .enumerate()
        .map(|(index, title)| {
            let mut note = Store::seed(index as u64).notes.remove(0);
            note.id = format!("note-{index}");
            note.title = title.into();
            note
        })
        .collect();
    let manifest = build_export_manifest(&store).unwrap();

    let summary =
        write_library_export_with(directory.path(), &manifest, "Crane Export names", write_new)
            .unwrap();

    assert_eq!(summary.exported_count, 6);
    for name in [
        "Plan.md",
        "plan 2.md",
        "Σ.md",
        "ς 2.md",
        "Untitled note.md",
        "Untitled note 2.md",
    ] {
        assert!(directory
            .path()
            .join("Crane Export names/Notes")
            .join(name)
            .is_file());
    }
}

#[test]
fn normalized_existing_backup_target_requires_confirmation() {
    let directory = TestDirectory::new("confirm");
    let selected = directory.path().join("daily.json");
    let normalized = directory.path().join("daily.crane-backup.json");
    std::fs::write(&normalized, b"existing").unwrap();
    let dialogs = FakeDialogs::backup(selected, false);
    let mut transfers = FilesystemTransfers::new(
        Box::new(dialogs.clone()),
        directory.path().join("safety"),
        "test",
    );

    let result = transfers.create_backup(&Store::seed(10), 20).unwrap();

    assert!(matches!(result, Outcome::Cancelled));
    assert_eq!(dialogs.confirmed_path(), Some(normalized.clone()));
    assert_eq!(std::fs::read(normalized).unwrap(), b"existing");
}

#[test]
fn normalized_existing_note_target_requires_confirmation() {
    let directory = TestDirectory::new("note-confirm");
    let selected = directory.path().join("draft");
    let normalized = directory.path().join("draft.md");
    std::fs::write(&normalized, b"existing").unwrap();
    let dialogs = FakeDialogs::note(selected, false);
    let mut transfers = FilesystemTransfers::new(
        Box::new(dialogs.clone()),
        directory.path().join("safety"),
        "test",
    );
    let note = Store::seed(10).notes.remove(0);

    let result = transfers.export_note(&note).unwrap();

    assert!(matches!(result, Outcome::Cancelled));
    assert_eq!(dialogs.confirmed_path(), Some(normalized.clone()));
    assert_eq!(std::fs::read(normalized).unwrap(), b"existing");
}

#[test]
fn failed_library_export_keeps_unrelated_siblings_and_hides_final_folder() {
    let directory = TestDirectory::new("failure");
    let sibling = directory.path().join("keep.txt");
    std::fs::write(&sibling, b"keep").unwrap();
    let store = Store::seed(100);
    let manifest = build_export_manifest(&store).unwrap();
    let mut writes = 0;

    let result = write_library_export_with(
        directory.path(),
        &manifest,
        "Crane Export failed",
        |path, contents| {
            writes += 1;
            if writes == 2 {
                return Err(io::Error::other("injected write failure"));
            }
            write_new(path, contents)
        },
    );

    assert!(result.is_err());
    assert!(sibling.is_file());
    assert!(!directory.path().join("Crane Export failed").exists());
    assert_eq!(std::fs::read_dir(directory.path()).unwrap().count(), 1);
}

#[test]
fn backup_size_is_checked_before_any_target_is_published() {
    let directory = TestDirectory::new("backup-limit");
    let path = directory.path().join("limited.crane-backup.json");
    let envelope = make_backup(&Store::seed(10), 20, "test").unwrap();

    let error = write_backup_with_limit(&path, &envelope, 16)
        .expect_err("the serialized backup exceeds the injected limit");

    assert!(error.to_string().contains("limit"));
    assert!(!path.exists());
}
