#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod model;
mod persistence;

use model::{extract_tags, validate_note, Note, Store};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::{Manager, State};

struct AppState {
    path: PathBuf,
    store: Mutex<Store>,
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

fn with_store<T>(state: State<'_, AppState>, operation: impl FnOnce(&mut Store) -> Result<T, String>) -> Result<T, String> {
    let mut store = state.store.lock().map_err(|_| "The note store is unavailable.".to_string())?;
    let result = operation(&mut store)?;
    persistence::save(&state.path, &store)?;
    Ok(result)
}

#[tauri::command]
fn bootstrap(state: State<'_, AppState>) -> Result<Store, String> {
    state.store.lock().map(|store| store.clone()).map_err(|_| "The note store is unavailable.".to_string())
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
        let existing = store.notes.iter_mut().find(|candidate| candidate.id == note.id).ok_or_else(|| "That note no longer exists.".to_string())?;
        note.created_at = existing.created_at;
        note.updated_at = now_millis();
        note.tags = extract_tags(&note.body);
        *existing = note.clone();
        Ok(note)
    })
}

#[tauri::command]
fn set_note_state(id: String, field: String, value: bool, state: State<'_, AppState>) -> Result<Note, String> {
    with_store(state, |store| {
        let note = store.notes.iter_mut().find(|candidate| candidate.id == id).ok_or_else(|| "That note no longer exists.".to_string())?;
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

fn main() {
    tauri::Builder::default()
        .setup(|app| {
            let data_dir = app.path().app_data_dir().map_err(|error| format!("Could not find the app data directory: {error}"))?;
            let path = data_dir.join("notes.json");
            let store = persistence::load_or_seed(&path, now_millis()).map_err(|error| -> Box<dyn std::error::Error> { error.into() })?;
            app.manage(AppState { path, store: Mutex::new(store) });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![bootstrap, create_note, save_note, set_note_state])
        .run(tauri::generate_context!())
        .expect("error while running Crane");
}

