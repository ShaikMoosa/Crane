#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[cfg(test)]
mod application_adapter;

use crane_application::{Command, CommandData, ExportSummary, Library};
use crane_domain::{BackupInspection, Note, Store};
use crane_platform::{NativeDialogs, SystemEnvironment};
use crane_storage::{FilesystemTransfers, JsonRepository};
use serde::Serialize;
use std::sync::{Arc, Mutex};
use tauri::{Manager, State};

struct AppState {
    library: Arc<Mutex<Library>>,
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
struct ExportSummaryPayload {
    exported_count: usize,
    folder_name: String,
}

impl From<ExportSummary> for ExportSummaryPayload {
    fn from(summary: ExportSummary) -> Self {
        Self {
            exported_count: summary.exported_count,
            folder_name: summary.folder_name,
        }
    }
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

impl BackupInspectionPayload {
    fn new(token: String, inspection: BackupInspection) -> Self {
        Self {
            token,
            created_at: inspection.created_at,
            total: inspection.total,
            active: inspection.active,
            archived: inspection.archived,
            trashed: inspection.trashed,
        }
    }
}

fn execute_command(state: State<'_, AppState>, command: Command) -> Result<CommandData, String> {
    state
        .library
        .lock()
        .map_err(|_| "The note library is unavailable.".to_string())?
        .execute(command)
        .map(|result| result.data)
        .map_err(|error| error.to_string())
}

async fn execute_transfer(
    state: State<'_, AppState>,
    command: Command,
) -> Result<CommandData, String> {
    let library = state.library.clone();
    tauri::async_runtime::spawn_blocking(move || {
        library
            .lock()
            .map_err(|_| "The note library is unavailable.".to_string())?
            .execute(command)
            .map(|result| result.data)
            .map_err(|error| error.to_string())
    })
    .await
    .map_err(|error| format!("The transfer task could not finish: {error}"))?
}

#[tauri::command]
fn bootstrap(state: State<'_, AppState>) -> Result<Store, String> {
    state
        .library
        .lock()
        .map(|library| library.snapshot().store)
        .map_err(|_| "The note library is unavailable.".to_string())
}

#[tauri::command]
fn create_note(state: State<'_, AppState>) -> Result<Note, String> {
    match execute_command(state, Command::CreateNote)? {
        CommandData::Created(note) => Ok(note),
        _ => Err("The create-note command returned an unexpected result.".into()),
    }
}

#[tauri::command]
fn save_note(note: Note, state: State<'_, AppState>) -> Result<Note, String> {
    match execute_command(
        state,
        Command::SaveNote {
            id: note.id,
            title: note.title,
            body: note.body,
            draft_revision: 0,
        },
    )? {
        CommandData::Saved { note, .. } => Ok(note),
        _ => Err("The save-note command returned an unexpected result.".into()),
    }
}

#[tauri::command]
fn set_note_state(
    id: String,
    field: String,
    value: bool,
    state: State<'_, AppState>,
) -> Result<Note, String> {
    let state_field = match field.as_str() {
        "pinned" => crane_application::NoteState::Pinned,
        "archived" => crane_application::NoteState::Archived,
        "trashed" => crane_application::NoteState::Trashed,
        _ => return Err("Unsupported note state.".into()),
    };
    match execute_command(
        state,
        Command::SetNoteState {
            id,
            state: state_field,
            value,
        },
    )? {
        CommandData::StateChanged(note) => Ok(note),
        _ => Err("The note-state command returned an unexpected result.".into()),
    }
}

#[tauri::command]
async fn import_markdown(
    state: State<'_, AppState>,
) -> Result<TransferOutcome<ImportSummary>, String> {
    match execute_transfer(state, Command::ImportMarkdown).await? {
        CommandData::Imported {
            store,
            imported_count,
        } => Ok(TransferOutcome::completed(ImportSummary {
            store,
            imported_count,
        })),
        CommandData::Cancelled => Ok(TransferOutcome::cancelled()),
        _ => Err("The Markdown import returned an unexpected result.".into()),
    }
}

#[tauri::command]
async fn export_note(
    id: String,
    state: State<'_, AppState>,
) -> Result<TransferOutcome<usize>, String> {
    match execute_transfer(state, Command::ExportNote { id }).await? {
        CommandData::Exported(summary) => Ok(TransferOutcome::completed(summary.exported_count)),
        CommandData::Cancelled => Ok(TransferOutcome::cancelled()),
        _ => Err("The note export returned an unexpected result.".into()),
    }
}

#[tauri::command]
async fn export_library(
    state: State<'_, AppState>,
) -> Result<TransferOutcome<ExportSummaryPayload>, String> {
    match execute_transfer(state, Command::ExportLibrary).await? {
        CommandData::Exported(summary) => Ok(TransferOutcome::completed(
            ExportSummaryPayload::from(summary),
        )),
        CommandData::Cancelled => Ok(TransferOutcome::cancelled()),
        _ => Err("The library export returned an unexpected result.".into()),
    }
}

#[tauri::command]
async fn create_backup(state: State<'_, AppState>) -> Result<TransferOutcome<usize>, String> {
    match execute_transfer(state, Command::CreateBackup).await? {
        CommandData::BackupCreated { note_count } => Ok(TransferOutcome::completed(note_count)),
        CommandData::Cancelled => Ok(TransferOutcome::cancelled()),
        _ => Err("The backup command returned an unexpected result.".into()),
    }
}

#[tauri::command]
async fn inspect_backup(
    state: State<'_, AppState>,
) -> Result<TransferOutcome<BackupInspectionPayload>, String> {
    match execute_transfer(state, Command::InspectBackup).await? {
        CommandData::Inspection { token, inspection } => Ok(TransferOutcome::completed(
            BackupInspectionPayload::new(token, inspection),
        )),
        CommandData::Cancelled => Ok(TransferOutcome::cancelled()),
        _ => Err("The backup inspection returned an unexpected result.".into()),
    }
}

#[tauri::command]
async fn restore_backup(
    confirmation_token: String,
    state: State<'_, AppState>,
) -> Result<TransferOutcome<Store>, String> {
    match execute_transfer(
        state,
        Command::RestoreBackup {
            token: confirmation_token,
        },
    )
    .await?
    {
        CommandData::Restored(store) => Ok(TransferOutcome::completed(store)),
        CommandData::Cancelled => Ok(TransferOutcome::cancelled()),
        _ => Err("The restore command returned an unexpected result.".into()),
    }
}

#[tauri::command]
async fn cancel_restore(
    confirmation_token: String,
    state: State<'_, AppState>,
) -> Result<TransferOutcome<()>, String> {
    match execute_transfer(
        state,
        Command::CancelRestore {
            token: confirmation_token,
        },
    )
    .await?
    {
        CommandData::RestoreCancelled => Ok(TransferOutcome::completed(())),
        _ => Err("The restore cancellation returned an unexpected result.".into()),
    }
}

fn main() {
    tauri::Builder::default()
        .setup(|app| {
            let data_dir = app
                .path()
                .app_data_dir()
                .map_err(|error| format!("Could not find the app data directory: {error}"))?;
            let library = Library::open(
                Box::new(JsonRepository::new(data_dir.join("notes.json"))),
                Box::new(SystemEnvironment::new()),
            )
            .map_err(|error| -> Box<dyn std::error::Error> { error.into() })?
            .with_transfers(Box::new(FilesystemTransfers::new(
                Box::new(NativeDialogs),
                data_dir.join("backups"),
                env!("CARGO_PKG_VERSION"),
            )));
            app.manage(AppState {
                library: Arc::new(Mutex::new(library)),
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
            restore_backup,
            cancel_restore
        ])
        .run(tauri::generate_context!())
        .expect("error while running Crane");
}

#[cfg(test)]
mod main_tests {
    use super::*;
    use crane_application::{AppError, Outcome, TransferIo};
    use crane_domain::NamedBytes;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    struct ImportFixture;

    impl TransferIo for ImportFixture {
        fn pick_markdown(&mut self) -> Result<Outcome<Vec<NamedBytes>>, AppError> {
            Ok(Outcome::Completed(vec![NamedBytes {
                name: "Imported.md".into(),
                bytes: b"# Imported\n\nFixture #legacy/path".to_vec(),
            }]))
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
            Ok(Outcome::Cancelled)
        }

        fn write_safety_backup(
            &mut self,
            _store: &Store,
            _created_at: u64,
        ) -> Result<(), AppError> {
            Ok(())
        }
    }

    fn isolated_path(label: &str) -> (PathBuf, PathBuf) {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let directory = std::env::current_dir()
            .unwrap()
            .join("target")
            .join(format!("legacy-{label}-{}-{nonce}", std::process::id()));
        (directory.join("notes.json"), directory)
    }

    #[test]
    fn legacy_adapter_creates_saves_and_reopens_an_isolated_library() {
        let (path, directory) = isolated_path("command");
        let (created, _) = application_adapter::execute(&path, Command::CreateNote).unwrap();
        let CommandData::Created(note) = created.data else {
            panic!("create must return a note");
        };
        application_adapter::execute(
            &path,
            Command::SaveNote {
                id: note.id.clone(),
                title: "Reopened title".into(),
                body: "Persisted body #integration".into(),
                draft_revision: 12,
            },
        )
        .unwrap();

        let reopened = application_adapter::open_library(&path).unwrap().snapshot();
        let saved = reopened
            .store
            .notes
            .iter()
            .find(|item| item.id == note.id)
            .unwrap();
        assert_eq!(saved.title, "Reopened title");
        assert_eq!(saved.tags, ["integration"]);
        let _ = std::fs::remove_dir_all(directory);
    }

    #[test]
    fn legacy_adapter_transfer_path_uses_shared_application() {
        let (path, directory) = isolated_path("transfer");
        let mut library =
            application_adapter::open_library_with_transfers(&path, Box::new(ImportFixture))
                .unwrap();
        let result = library.execute(Command::ImportMarkdown).unwrap();
        let CommandData::Imported {
            store,
            imported_count,
        } = result.data
        else {
            panic!("import must return the shared application result");
        };
        assert_eq!(imported_count, 1);
        assert!(store.notes.iter().any(|note| note.title == "Imported"));
        assert!(path.is_file());
        let _ = std::fs::remove_dir_all(directory);
    }

    #[test]
    fn shared_resolver_matches_tauri_app_data_dir() {
        let mut context = tauri::test::mock_context(tauri::test::noop_assets());
        context.config_mut().identifier = "app.crane.desktop".into();
        let app = tauri::test::mock_builder()
            .build(context)
            .expect("the Tauri path test app must build");
        assert_eq!(
            crane_platform::resolve_legacy_data_dir().unwrap(),
            app.path().app_data_dir().unwrap()
        );
    }

    #[test]
    fn cancelled_outcome_serializes_without_data() {
        let outcome = TransferOutcome::<usize>::cancelled();
        let value = serde_json::to_value(outcome).expect("outcome should serialize");
        assert_eq!(value["status"], "cancelled");
        assert!(value.get("data").is_none());
    }
}
