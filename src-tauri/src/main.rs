#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod model;
mod persistence;
mod transfer;

use model::{extract_tags, validate_note, validate_store, Note, Store};
use serde::Serialize;
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Manager, State};
use tauri_plugin_dialog::{DialogExt, FilePath, MessageDialogButtons};
use transfer::backup::{self, PendingRestore};
use transfer::markdown::{self, ExportSummary, NamedBytes};
use transfer::TransferError;

struct AppState {
    path: PathBuf,
    store: Arc<Mutex<Store>>,
    pending_restore: Arc<Mutex<Option<PendingRestore>>>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct TransferOutcome<T> {
    status: TransferStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<T>,
}

impl<T> TransferOutcome<T> {
    fn completed(data: T) -> Self {
        Self {
            status: TransferStatus::Completed,
            data: Some(data),
        }
    }

    fn cancelled() -> Self {
        Self {
            status: TransferStatus::Cancelled,
            data: None,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
enum TransferStatus {
    Completed,
    Cancelled,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ImportSummary {
    store: Store,
    imported_count: usize,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct BackupInspectionPayload {
    token: String,
    created_at: u64,
    total: usize,
    active: usize,
    archived: usize,
    trashed: usize,
}

fn take_pending_restore(
    pending: &Mutex<Option<PendingRestore>>,
    token: &str,
) -> Result<PendingRestore, String> {
    let mut guard = pending
        .lock()
        .map_err(|_| "The pending restore is unavailable.".to_string())?;
    match guard.as_ref() {
        Some(candidate) if candidate.token == token => guard
            .take()
            .ok_or_else(|| "That restore confirmation is no longer valid.".to_string()),
        _ => Err("That restore confirmation is no longer valid.".into()),
    }
}

static NOTE_COUNTER: AtomicU64 = AtomicU64::new(0);

fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

fn new_id() -> String {
    let sequence = NOTE_COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("note-{}-{sequence}", now_millis())
}

fn selected_path(path: FilePath) -> Result<PathBuf, String> {
    path.into_path()
        .map_err(|error| format!("The selected item is not a local file: {error}"))
}

fn read_file_limited(path: &Path, byte_limit: usize, too_large: &str) -> Result<Vec<u8>, String> {
    let file =
        File::open(path).map_err(|error| format!("Could not read {}: {error}", path.display()))?;
    let mut bytes = Vec::with_capacity(byte_limit.min(64 * 1024));
    file.take(byte_limit.saturating_add(1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|error| format!("Could not read {}: {error}", path.display()))?;
    if bytes.len() > byte_limit {
        return Err(too_large.to_string());
    }
    Ok(bytes)
}

fn format_export_timestamp(timestamp_millis: u64) -> String {
    let seconds = (timestamp_millis / 1_000).min(i64::MAX as u64) as i64;
    let days = seconds / 86_400;
    let seconds_in_day = seconds % 86_400;
    let z = days + 719_468;
    let era = z / 146_097;
    let day_of_era = z - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let mut year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_prime = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_prime + 2) / 5 + 1;
    let month = month_prime + if month_prime < 10 { 3 } else { -9 };
    if month <= 2 {
        year += 1;
    }
    let hour = seconds_in_day / 3_600;
    let minute = (seconds_in_day % 3_600) / 60;
    let second = seconds_in_day % 60;
    format!("{year:04}-{month:02}-{day:02} {hour:02}{minute:02}{second:02}")
}

fn with_store<T>(
    state: State<'_, AppState>,
    operation: impl FnOnce(&mut Store) -> Result<T, String>,
) -> Result<T, String> {
    let mut store = state
        .store
        .lock()
        .map_err(|_| "The note store is unavailable.".to_string())?;
    let result = operation(&mut store)?;
    persistence::save(&state.path, &store)?;
    Ok(result)
}

#[tauri::command]
fn bootstrap(state: State<'_, AppState>) -> Result<Store, String> {
    state
        .store
        .lock()
        .map(|store| store.clone())
        .map_err(|_| "The note store is unavailable.".to_string())
}

#[tauri::command]
fn create_note(state: State<'_, AppState>) -> Result<Note, String> {
    with_store(state, |store| {
        let timestamp = now_millis();
        let note = Note {
            id: new_id(),
            title: "Untitled note".into(),
            body: String::new(),
            tags: Vec::new(),
            created_at: timestamp,
            updated_at: timestamp,
            pinned: false,
            archived: false,
            trashed: false,
        };
        store.notes.push(note.clone());
        Ok(note)
    })
}

#[tauri::command]
fn save_note(mut note: Note, state: State<'_, AppState>) -> Result<Note, String> {
    validate_note(&note)?;
    with_store(state, |store| {
        let existing = store
            .notes
            .iter_mut()
            .find(|candidate| candidate.id == note.id)
            .ok_or_else(|| "That note no longer exists.".to_string())?;
        note.created_at = existing.created_at;
        note.updated_at = now_millis();
        note.tags = extract_tags(&note.body);
        *existing = note.clone();
        Ok(note)
    })
}

#[tauri::command]
fn set_note_state(
    id: String,
    field: String,
    value: bool,
    state: State<'_, AppState>,
) -> Result<Note, String> {
    with_store(state, |store| {
        let note = store
            .notes
            .iter_mut()
            .find(|candidate| candidate.id == id)
            .ok_or_else(|| "That note no longer exists.".to_string())?;
        match field.as_str() {
            "pinned" => note.pinned = value,
            "archived" => note.archived = value,
            "trashed" => note.trashed = value,
            _ => return Err("Unsupported note state.".into()),
        }
        note.updated_at = now_millis();
        Ok(note.clone())
    })
}

#[tauri::command]
async fn import_markdown(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<TransferOutcome<ImportSummary>, String> {
    let Some(selection) = app
        .dialog()
        .file()
        .set_title("Import Markdown")
        .add_filter("Markdown", &["md"])
        .blocking_pick_files()
    else {
        return Ok(TransferOutcome::cancelled());
    };
    if selection.is_empty() {
        return Ok(TransferOutcome::cancelled());
    }

    let paths = selection
        .into_iter()
        .map(selected_path)
        .collect::<Result<Vec<_>, _>>()?;
    let timestamp = now_millis();
    let ids: Vec<_> = (0..paths.len()).map(|_| new_id()).collect();
    let prepared = tauri::async_runtime::spawn_blocking(move || {
        let mut remaining = markdown::MAX_BATCH_BYTES;
        let files = paths
            .into_iter()
            .map(|path| {
                let name = path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .ok_or_else(|| {
                        "A selected Markdown filename is not valid Unicode.".to_string()
                    })?
                    .to_string();
                let bytes = read_file_limited(
                    &path,
                    remaining,
                    "The selected Markdown files are larger than the 64 MB batch limit.",
                )?;
                remaining = remaining.saturating_sub(bytes.len());
                Ok(NamedBytes { name, bytes })
            })
            .collect::<Result<Vec<_>, String>>()?;
        markdown::prepare_import(files, timestamp, ids.into_iter())
            .map_err(|error| error.to_string())
    })
    .await
    .map_err(|error| format!("The Markdown import task could not finish: {error}"))??;

    let imported_count = prepared.len();
    let store_handle = state.store.clone();
    let data_path = state.path.clone();
    let store = tauri::async_runtime::spawn_blocking(move || {
        let mut current = store_handle
            .lock()
            .map_err(|_| "The note store is unavailable.".to_string())?;
        let mut candidate = current.clone();
        candidate.notes.extend(prepared);
        validate_store(&candidate)?;
        persistence::save(&data_path, &candidate)?;
        *current = candidate.clone();
        Ok::<Store, String>(candidate)
    })
    .await
    .map_err(|error| format!("The Markdown import task could not finish: {error}"))??;

    Ok(TransferOutcome::completed(ImportSummary {
        store,
        imported_count,
    }))
}

#[tauri::command]
async fn export_note(
    id: String,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<TransferOutcome<usize>, String> {
    let note = state
        .store
        .lock()
        .map_err(|_| "The note store is unavailable.".to_string())?
        .notes
        .iter()
        .find(|note| note.id == id)
        .cloned()
        .ok_or_else(|| "That note no longer exists.".to_string())?;
    let Some(selection) = app
        .dialog()
        .file()
        .set_title("Export Note")
        .set_file_name(markdown::suggested_markdown_filename(&note))
        .add_filter("Markdown", &["md"])
        .blocking_save_file()
    else {
        return Ok(TransferOutcome::cancelled());
    };
    let path = ensure_suffix(selected_path(selection)?, ".md");
    tauri::async_runtime::spawn_blocking(move || markdown::write_note_export(&path, &note))
        .await
        .map_err(|error| format!("The note export task could not finish: {error}"))?
        .map_err(|error| error.to_string())?;
    Ok(TransferOutcome::completed(1))
}

#[tauri::command]
async fn export_library(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<TransferOutcome<ExportSummary>, String> {
    let Some(selection) = app
        .dialog()
        .file()
        .set_title("Export Crane Library")
        .blocking_pick_folder()
    else {
        return Ok(TransferOutcome::cancelled());
    };
    let destination = selected_path(selection)?;
    let snapshot = state
        .store
        .lock()
        .map(|store| store.clone())
        .map_err(|_| "The note store is unavailable.".to_string())?;
    let folder_name = format!("Crane Export {}", format_export_timestamp(now_millis()));
    let summary = tauri::async_runtime::spawn_blocking(move || {
        let manifest = markdown::build_export_manifest(&snapshot)?;
        markdown::write_library_export(&destination, &manifest, &folder_name)
    })
    .await
    .map_err(|error| format!("The library export task could not finish: {error}"))?
    .map_err(|error| error.to_string())?;
    Ok(TransferOutcome::completed(summary))
}

#[tauri::command]
async fn create_backup(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<TransferOutcome<usize>, String> {
    let snapshot = state
        .store
        .lock()
        .map(|store| store.clone())
        .map_err(|_| "The note store is unavailable.".to_string())?;
    let created_at = now_millis();
    let default_name = format!(
        "Crane Backup {}.crane-backup.json",
        format_export_timestamp(created_at)
    );
    let Some(selection) = app
        .dialog()
        .file()
        .set_title("Create Crane Backup")
        .set_file_name(default_name)
        .add_filter("Crane backup", &["json"])
        .blocking_save_file()
    else {
        return Ok(TransferOutcome::cancelled());
    };
    let selected = selected_path(selection)?;
    let path = ensure_backup_suffix(selected.clone());
    if backup_overwrite_requires_confirmation(&selected, &path)
        && !app
            .dialog()
            .message(format!(
                "A file named {} already exists. Replace it?",
                path.file_name()
                    .map(|name| name.to_string_lossy())
                    .unwrap_or_default()
            ))
            .title("Replace existing backup?")
            .buttons(MessageDialogButtons::OkCancelCustom(
                "Replace".into(),
                "Cancel".into(),
            ))
            .blocking_show()
    {
        return Ok(TransferOutcome::cancelled());
    }
    let note_count = snapshot.notes.len();
    tauri::async_runtime::spawn_blocking(move || {
        let envelope = backup::make_envelope(&snapshot, created_at, env!("CARGO_PKG_VERSION"))?;
        backup::write_backup(&path, &envelope)
    })
    .await
    .map_err(|error| format!("The backup task could not finish: {error}"))?
    .map_err(|error| error.to_string())?;
    Ok(TransferOutcome::completed(note_count))
}

#[tauri::command]
async fn inspect_backup(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<TransferOutcome<BackupInspectionPayload>, String> {
    state
        .pending_restore
        .lock()
        .map_err(|_| "The pending restore is unavailable.".to_string())?
        .take();
    let Some(selection) = app
        .dialog()
        .file()
        .set_title("Restore Crane Backup")
        .add_filter("Crane backup", &["json"])
        .blocking_pick_file()
    else {
        return Ok(TransferOutcome::cancelled());
    };
    let path = selected_path(selection)?;
    let token = format!("restore-{}", new_id());
    let token_for_task = token.clone();
    let pending = tauri::async_runtime::spawn_blocking(move || {
        let bytes = read_file_limited(
            &path,
            backup::MAX_BACKUP_BYTES,
            "The backup is larger than the 256 MB limit.",
        )
        .map_err(TransferError::Io)?;
        backup::inspect_backup(&bytes, token_for_task)
    })
    .await
    .map_err(|error| format!("The backup inspection task could not finish: {error}"))?
    .map_err(|error| error.to_string())?;
    let inspection = pending.inspection.clone();
    *state
        .pending_restore
        .lock()
        .map_err(|_| "The pending restore is unavailable.".to_string())? = Some(pending);
    Ok(TransferOutcome::completed(BackupInspectionPayload {
        token,
        created_at: inspection.created_at,
        total: inspection.total,
        active: inspection.active,
        archived: inspection.archived,
        trashed: inspection.trashed,
    }))
}

#[tauri::command]
async fn restore_backup(
    confirmation_token: String,
    state: State<'_, AppState>,
) -> Result<TransferOutcome<Store>, String> {
    let pending = take_pending_restore(&state.pending_restore, &confirmation_token)?;
    let store_handle = state.store.clone();
    let data_path = state.path.clone();
    let backup_directory = data_path
        .parent()
        .ok_or_else(|| "The Crane data path has no parent folder.".to_string())?
        .join("backups");
    let timestamp = now_millis();
    let restored = tauri::async_runtime::spawn_blocking(move || {
        let mut current = store_handle
            .lock()
            .map_err(|_| "The note store is unavailable.".to_string())?;
        let current_snapshot = current.clone();
        let restored = backup::restore_transaction(
            &current_snapshot,
            pending.replacement,
            |store| {
                backup::write_safety_backup(
                    &backup_directory,
                    store,
                    timestamp,
                    env!("CARGO_PKG_VERSION"),
                )
                .map(|_| ())
                .map_err(|error| {
                    TransferError::Io(format!(
                        "Could not create the pre-restore safety backup: {error}"
                    ))
                })
            },
            |store| {
                persistence::save(&data_path, store).map_err(|error| {
                    TransferError::Io(format!("Could not replace the Crane library: {error}"))
                })
            },
        )
        .map_err(|error| error.to_string())?;
        *current = restored.clone();
        Ok::<Store, String>(restored)
    })
    .await
    .map_err(|error| format!("The restore task could not finish: {error}"))??;
    Ok(TransferOutcome::completed(restored))
}

fn ensure_suffix(path: PathBuf, suffix: &str) -> PathBuf {
    if path
        .to_string_lossy()
        .to_ascii_lowercase()
        .ends_with(&suffix.to_ascii_lowercase())
    {
        return path;
    }
    let mut value = path.into_os_string();
    value.push(suffix);
    PathBuf::from(value)
}

fn ensure_backup_suffix(path: PathBuf) -> PathBuf {
    const SUFFIX: &str = ".crane-backup.json";
    if path
        .to_string_lossy()
        .to_ascii_lowercase()
        .ends_with(SUFFIX)
    {
        return path;
    }
    if path
        .extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("json"))
    {
        let mut without_json = path;
        without_json.set_extension("");
        let mut value = without_json.into_os_string();
        value.push(SUFFIX);
        return PathBuf::from(value);
    }
    ensure_suffix(path, SUFFIX)
}

fn backup_overwrite_requires_confirmation(selected: &Path, destination: &Path) -> bool {
    selected != destination && destination.exists()
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let data_dir = app
                .path()
                .app_data_dir()
                .map_err(|error| format!("Could not find the app data directory: {error}"))?;
            let path = data_dir.join("notes.json");
            let store = persistence::load_or_seed(&path, now_millis())
                .map_err(|error| -> Box<dyn std::error::Error> { error.into() })?;
            app.manage(AppState {
                path,
                store: Arc::new(Mutex::new(store)),
                pending_restore: Arc::new(Mutex::new(None)),
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            bootstrap,
            create_note,
            save_note,
            set_note_state,
            import_markdown,
            export_note,
            export_library,
            create_backup,
            inspect_backup,
            restore_backup
        ])
        .run(tauri::generate_context!())
        .expect("error while running Crane");
}

#[cfg(test)]
mod main_tests {
    use super::*;
    use crate::transfer::backup::{BackupInspection, PendingRestore};

    fn pending(token: &str) -> PendingRestore {
        PendingRestore {
            token: token.into(),
            inspection: BackupInspection {
                created_at: 10,
                total: 3,
                active: 1,
                archived: 1,
                trashed: 1,
            },
            replacement: Store::seed(10),
        }
    }

    #[test]
    fn cancelled_outcome_serializes_without_data() {
        let outcome = TransferOutcome::<usize>::cancelled();
        let value = serde_json::to_value(outcome).expect("outcome should serialize");

        assert_eq!(value["status"], "cancelled");
        assert!(value.get("data").is_none());
    }

    #[test]
    fn restore_token_is_single_use() {
        let state = Mutex::new(Some(pending("valid")));

        let restored = take_pending_restore(&state, "valid").expect("token should be accepted");
        assert_eq!(restored.token, "valid");
        assert!(take_pending_restore(&state, "valid").is_err());
    }

    #[test]
    fn stale_restore_token_is_rejected() {
        let state = Mutex::new(Some(pending("current")));

        let error = take_pending_restore(&state, "stale").expect_err("stale tokens must fail");
        assert!(error.contains("no longer valid"));
        assert_eq!(
            take_pending_restore(&state, "current")
                .expect("a stale attempt must not consume the current token")
                .token,
            "current"
        );
    }

    #[test]
    fn formats_export_timestamp_in_utc() {
        assert_eq!(format_export_timestamp(0), "1970-01-01 000000");
        assert_eq!(
            format_export_timestamp(1_760_870_400_000),
            "2025-10-19 104000"
        );
    }

    #[test]
    fn bounded_reader_rejects_before_retaining_bytes_past_the_limit() {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("the system clock should be after the Unix epoch")
            .as_nanos();
        let path = std::env::current_dir()
            .expect("the test should have a working directory")
            .join("target")
            .join(format!("crane-read-test-{}-{nonce}", std::process::id()));
        std::fs::write(&path, b"12345").expect("fixture should be written");

        let error = read_file_limited(&path, 4, "The selected file is too large.")
            .expect_err("the fifth byte must trip the four-byte limit");

        assert_eq!(error, "The selected file is too large.");
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn normalized_backup_targets_that_exist_require_confirmation() {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("the system clock should be after the Unix epoch")
            .as_nanos();
        let folder = std::env::current_dir()
            .expect("the test should have a working directory")
            .join("target")
            .join(format!(
                "crane-overwrite-test-{}-{nonce}",
                std::process::id()
            ));
        std::fs::create_dir_all(&folder).expect("fixture folder should exist");
        let selected = folder.join("daily.json");
        let normalized = ensure_backup_suffix(selected.clone());
        std::fs::write(&normalized, b"existing").expect("normalized target should exist");

        assert!(backup_overwrite_requires_confirmation(
            &selected,
            &normalized
        ));
        let _ = std::fs::remove_dir_all(folder);
    }
}
