mod common;

use common::{app_with_transfers, note};
use crane_application::{
    AppError, Command, CommandData, CommandResult, ErrorCode, ExportSummary, Outcome, TransferIo,
};
use crane_desktop::draft::Draft;
use crane_desktop::transfers::{TransferController, TransferEffect, TransferState};
use crane_desktop::worker::WorkerResult;
use crane_domain::{BackupInspection, NamedBytes, Note, Store};
use eframe::egui::accesskit::Role;
use egui_kittest::{
    kittest::{NodeT, Queryable},
    Harness,
};
use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, Instant};

fn draft(dirty: bool) -> Draft {
    Draft {
        note_id: "selected".into(),
        title: "Selected".into(),
        body: "Body".into(),
        revision: if dirty { 1 } else { 0 },
        acknowledged_revision: 0,
    }
}

fn result(request_id: u64, data: CommandData) -> WorkerResult {
    WorkerResult {
        request_id,
        result: Ok(CommandResult { revision: 7, data }),
    }
}

#[test]
fn pending_save_failure_prevents_transfer_dispatch() {
    let mut controller = TransferController::default();
    let dirty = draft(true);

    controller.begin(Command::ImportMarkdown, &dirty).unwrap();

    assert_eq!(controller.state(), &TransferState::Flushing);
    assert!(controller.take_ready_request(Some(&dirty)).is_none());
}

#[test]
fn cancellation_is_silent_and_duplicate_activation_is_ignored() {
    let mut controller = TransferController::default();
    let clean = draft(false);
    controller.begin(Command::CreateBackup, &clean).unwrap();
    let request = controller.take_ready_request(Some(&clean)).unwrap();
    controller.begin(Command::ExportLibrary, &clean).unwrap();

    let effect = controller.complete(result(request.request_id, CommandData::Cancelled));

    assert!(matches!(effect, TransferEffect::None));
    assert_eq!(controller.state(), &TransferState::Idle);
    assert_eq!(controller.status(), None);
}

#[test]
fn useful_errors_persist_and_completed_import_refreshes_the_store() {
    let mut controller = TransferController::default();
    let clean = draft(false);
    controller.begin(Command::ImportMarkdown, &clean).unwrap();
    let request = controller.take_ready_request(Some(&clean)).unwrap();
    let imported = Store {
        schema_version: 1,
        notes: vec![note("imported", "Imported", "Body", 2)],
        extensions: Default::default(),
    };

    let effect = controller.complete(result(
        request.request_id,
        CommandData::Imported {
            store: imported,
            imported_count: 1,
        },
    ));
    let TransferEffect::ReplaceStore(imported) = effect else {
        panic!("completed import did not request a store refresh");
    };
    assert_eq!(imported.notes[0].id, "imported");
    assert_eq!(controller.status(), Some("1 note imported"));

    controller.begin(Command::ExportLibrary, &clean).unwrap();
    let request = controller.take_ready_request(Some(&clean)).unwrap();
    controller.complete(WorkerResult {
        request_id: request.request_id,
        result: Err(AppError::new(
            ErrorCode::Io,
            "Could not publish the library export: disk unavailable",
        )),
    });
    assert_eq!(
        controller.status(),
        Some("Library export failed: Could not publish the library export: disk unavailable")
    );
}

#[derive(Default)]
struct GateState {
    active: bool,
    released: bool,
}

#[derive(Clone, Default)]
struct TransferGate(Arc<(Mutex<GateState>, Condvar)>);

impl TransferGate {
    fn wait_until_active(&self) {
        let deadline = Instant::now() + Duration::from_secs(5);
        let (lock, changed) = &*self.0;
        let mut state = lock.lock().unwrap();
        while !state.active {
            let remaining = deadline.saturating_duration_since(Instant::now());
            assert!(
                !remaining.is_zero(),
                "import never reached its dialog adapter"
            );
            (state, _) = changed.wait_timeout(state, remaining).unwrap();
        }
    }

    fn release(&self) {
        let (lock, changed) = &*self.0;
        let mut state = lock.lock().unwrap();
        state.released = true;
        changed.notify_all();
    }
}

#[derive(Default)]
struct TransferLog {
    exported_ids: Vec<String>,
    import_calls: usize,
}

struct ControlledTransfers {
    gate: Option<TransferGate>,
    log: Arc<Mutex<TransferLog>>,
}

impl TransferIo for ControlledTransfers {
    fn pick_markdown(&mut self) -> Result<Outcome<Vec<NamedBytes>>, AppError> {
        self.log.lock().unwrap().import_calls += 1;
        if let Some(gate) = &self.gate {
            let (lock, changed) = &*gate.0;
            let mut state = lock.lock().unwrap();
            state.active = true;
            changed.notify_all();
            while !state.released {
                state = changed.wait(state).unwrap();
            }
        }
        Ok(Outcome::Completed(vec![NamedBytes {
            name: "Imported.md".into(),
            bytes: b"# Imported\n\nImported body".to_vec(),
        }]))
    }

    fn export_note(&mut self, note: &Note) -> Result<Outcome<usize>, AppError> {
        self.log.lock().unwrap().exported_ids.push(note.id.clone());
        Ok(Outcome::Completed(1))
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

    fn write_safety_backup(&mut self, _store: &Store, _created_at: u64) -> Result<(), AppError> {
        Ok(())
    }
}

#[test]
fn failed_prerequisite_save_aborts_import_and_keeps_retry_available() {
    let log = Arc::new(Mutex::new(TransferLog::default()));
    let transfers = ControlledTransfers {
        gate: None,
        log: log.clone(),
    };
    let (mut app, committed) =
        app_with_transfers(vec![note("alpha", "Alpha", "Old", 1)], Box::new(transfers));
    app.edit_selected_for_test("Alpha", "Save before import");
    app.simulate_save_failure_for_test("disk full");
    let mut harness = Harness::builder()
        .with_size([1240.0, 780.0])
        .build_ui_state(|ui, app| app.show(ui), app);

    harness.get_by_label("Data").click();
    harness.run();
    harness.get_by_label("Import Markdown…").click();
    harness.run();
    assert_eq!(log.lock().unwrap().import_calls, 0);
    assert!(!harness
        .get_by_role_and_label(Role::DisclosureTriangle, "Data")
        .accesskit_node()
        .is_disabled());

    harness.get_by_label("Retry save").click();
    settle(&mut harness);

    assert_eq!(log.lock().unwrap().import_calls, 0);
    assert_eq!(committed.lock().unwrap().notes.len(), 1);

    harness.get_by_label("Data").click();
    harness.run();
    harness.get_by_label("Import Markdown…").click();
    settle(&mut harness);

    assert_eq!(log.lock().unwrap().import_calls, 1);
    assert_eq!(committed.lock().unwrap().notes.len(), 2);
}

fn settle(harness: &mut Harness<'_, crane_desktop::app::CraneApp>) {
    let deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline {
        harness.run();
        if !harness.state().has_pending_work() {
            return;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    panic!("native transfer did not settle");
}

#[test]
fn selected_trashed_notes_can_export() {
    let mut trashed = note("trashed", "Discarded draft", "Still readable", 1);
    trashed.trashed = true;
    let log = Arc::new(Mutex::new(TransferLog::default()));
    let transfers = ControlledTransfers {
        gate: None,
        log: log.clone(),
    };
    let (app, _) = app_with_transfers(vec![trashed], Box::new(transfers));
    let mut harness = Harness::builder()
        .with_size([1240.0, 780.0])
        .build_ui_state(|ui, app| app.show(ui), app);

    harness.get_by_label("Trash").click();
    harness.run();
    harness.get_by_label("Discarded draft").click();
    harness.run();
    harness.get_by_label("Export note").click();
    settle(&mut harness);

    assert_eq!(log.lock().unwrap().exported_ids, ["trashed"]);
}

#[test]
fn delayed_import_blocks_typing_create_and_archive_until_refresh() {
    let gate = TransferGate::default();
    let transfers = ControlledTransfers {
        gate: Some(gate.clone()),
        log: Arc::new(Mutex::new(TransferLog::default())),
    };
    let (app, committed) = app_with_transfers(
        vec![note("alpha", "Alpha", "Original", 1)],
        Box::new(transfers),
    );
    let mut harness = Harness::builder()
        .with_size([1240.0, 780.0])
        .build_ui_state(|ui, app| app.show(ui), app);

    harness.get_by_label("Data").click();
    harness.run();
    harness.get_by_label("Import Markdown…").click();
    harness.run();
    gate.wait_until_active();
    harness.run();

    harness
        .get_by_role_and_label(Role::MultilineTextInput, "Note body")
        .set_text_value_accesskit("Must not apply");
    harness.get_by_label("New note").click();
    harness.get_by_label("Archive note").click();
    harness.run();
    let body_while_blocked = harness.state().selected_draft().unwrap().body.clone();

    gate.release();
    settle(&mut harness);
    assert_eq!(body_while_blocked, "Original");
    let store = committed.lock().unwrap();
    assert_eq!(store.notes.len(), 2);
    let alpha = store.notes.iter().find(|item| item.id == "alpha").unwrap();
    assert_eq!(alpha.body, "Original");
    assert!(!alpha.archived);
}

#[test]
fn delayed_import_blocks_empty_editor_creation_until_refresh() {
    let gate = TransferGate::default();
    let transfers = ControlledTransfers {
        gate: Some(gate.clone()),
        log: Arc::new(Mutex::new(TransferLog::default())),
    };
    let (app, committed) = app_with_transfers(Vec::new(), Box::new(transfers));
    let mut harness = Harness::builder()
        .with_size([1240.0, 780.0])
        .build_ui_state(|ui, app| app.show(ui), app);

    harness.get_by_label("Data").click();
    harness.run();
    harness.get_by_label("Import Markdown…").click();
    harness.run();
    gate.wait_until_active();
    harness.run();

    let empty_create: Vec<_> = harness
        .get_all_by_role_and_label(Role::Button, "New note")
        .collect();
    assert!(empty_create
        .iter()
        .all(|node| node.accesskit_node().is_disabled()));
    for node in empty_create {
        node.click();
    }
    harness.run();

    gate.release();
    settle(&mut harness);
    assert_eq!(committed.lock().unwrap().notes.len(), 1);
}

#[test]
fn terminal_worker_send_failure_unlocks_instead_of_retrying_forever() {
    let mut controller = TransferController::default();
    let clean = draft(false);
    controller.begin(Command::ExportLibrary, &clean).unwrap();
    let request = controller.take_ready_request(Some(&clean)).unwrap();

    controller.return_unsent(
        request,
        AppError::new(
            ErrorCode::Busy,
            "The local library worker is not available.",
        ),
    );

    assert_eq!(controller.state(), &TransferState::Idle);
    assert!(controller.can_mutate());
    assert_eq!(
        controller.status(),
        Some("The local library worker is not available.")
    );
    assert!(controller.take_ready_request(Some(&clean)).is_none());
}

#[test]
fn full_worker_queue_keeps_the_transfer_queued_with_visible_status() {
    let mut controller = TransferController::default();
    let clean = draft(false);
    controller.begin(Command::ExportLibrary, &clean).unwrap();
    let request = controller.take_ready_request(Some(&clean)).unwrap();

    controller.return_unsent(
        request,
        AppError::new(ErrorCode::QueueFull, "Earlier changes are still saving."),
    );

    assert_eq!(controller.state(), &TransferState::Flushing);
    assert_eq!(
        controller.status(),
        Some("Earlier changes are still saving.")
    );
    assert!(controller.take_ready_request(Some(&clean)).is_some());
}

#[test]
fn restore_completion_is_a_store_replacement_effect() {
    let mut controller = TransferController::default();
    let clean = draft(false);
    let inspection = BackupInspection {
        created_at: 100,
        total: 0,
        active: 0,
        archived: 0,
        trashed: 0,
    };
    controller.begin(Command::InspectBackup, &clean).unwrap();
    let inspect = controller.take_ready_request(Some(&clean)).unwrap();
    controller.complete(result(
        inspect.request_id,
        CommandData::Inspection {
            token: "one-use".into(),
            inspection,
        },
    ));
    controller
        .begin(
            Command::RestoreBackup {
                token: "one-use".into(),
            },
            &clean,
        )
        .unwrap();
    let restore = controller.take_ready_request(Some(&clean)).unwrap();
    let replacement = Store {
        schema_version: 1,
        notes: Vec::new(),
        extensions: Default::default(),
    };

    let effect = controller.complete(result(
        restore.request_id,
        CommandData::Restored(replacement),
    ));
    let TransferEffect::ReplaceStore(replacement) = effect else {
        panic!("completed restore did not request a store refresh");
    };
    assert!(replacement.notes.is_empty());
}
