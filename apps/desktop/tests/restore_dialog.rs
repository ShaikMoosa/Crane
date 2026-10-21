mod common;

use common::{app_with_transfers, note};
use crane_application::{
    AppError, Command, CommandData, CommandResult, ErrorCode, ExportSummary, Outcome, TransferIo,
};
use crane_desktop::draft::Draft;
use crane_desktop::transfers::{TransferController, TransferState};
use crane_desktop::views::restore::{self, RestoreAction};
use crane_desktop::worker::{WorkerOutcome, WorkerResult};
use crane_domain::{make_backup, serialize_backup, BackupInspection, NamedBytes, Note, Store};
use eframe::egui::{accesskit::Role, Key};
use egui_kittest::{
    kittest::{NodeT, Queryable},
    Harness,
};
use std::time::{Duration, Instant};

fn clean_draft() -> Draft {
    Draft {
        note_id: "selected".into(),
        title: "Selected".into(),
        body: String::new(),
        revision: 0,
        acknowledged_revision: 0,
    }
}

fn inspection() -> BackupInspection {
    BackupInspection {
        created_at: 1_700_000_000_000,
        total: 8,
        active: 4,
        archived: 3,
        trashed: 1,
    }
}

fn inspected_controller(token: &str) -> TransferController {
    let mut controller = TransferController::default();
    let clean = clean_draft();
    controller.begin(Command::InspectBackup, &clean).unwrap();
    let request = controller.take_ready_request(Some(&clean)).unwrap();
    controller.complete(WorkerResult {
        request_id: request.request_id,
        outcome: WorkerOutcome::Command(Ok(CommandResult {
            revision: 0,
            data: CommandData::Inspection {
                token: token.into(),
                inspection: inspection(),
            },
        })),
    });
    controller
}

#[test]
fn inspection_shows_counts_replacement_copy_and_focuses_cancel_without_writing() {
    let controller = inspected_controller("inspect-only");
    assert!(matches!(
        controller.state(),
        TransferState::AwaitingConfirmation { token, .. } if token == "inspect-only"
    ));
    let mut harness = Harness::builder().with_size([720.0, 560.0]).build_ui_state(
        |ui, state| {
            state.escape_seen |= ui.ctx().input(|input| input.key_pressed(Key::Escape));
            let action = restore::show(
                ui.ctx(),
                state.controller.restore_dialog().unwrap(),
                state.request_focus,
            );
            if action != RestoreAction::None {
                state.action = action;
            }
            state.request_focus = false;
        },
        DialogHarness {
            controller,
            request_focus: true,
            action: RestoreAction::None,
            escape_seen: false,
        },
    );

    harness.get_by_role_and_label(Role::Dialog, "Restore Crane backup");
    harness.get_by_label("Backup date: 2023-11-14 22:13 UTC");
    harness.get_by_label("8 total · 4 active · 3 archived · 1 trash");
    harness
        .get_by_label("This replaces the current Crane library after a safety backup is written.");
    assert!(harness.get_by_label("Cancel restore").is_focused());
    assert_eq!(harness.state().action, RestoreAction::None);

    for _ in 0..4 {
        harness.key_press(Key::Tab);
        harness.run_steps(8);
        assert!(
            harness.get_by_label("Cancel restore").is_focused()
                || harness.get_by_label("Replace library").is_focused(),
            "Tab focus escaped the restore dialog"
        );
    }

    harness.key_press(Key::Escape);
    harness.run();
    assert!(harness.state().escape_seen, "egui did not receive Escape");
    assert_eq!(harness.state().action, RestoreAction::Cancel);
}

struct DialogHarness {
    controller: TransferController,
    request_focus: bool,
    action: RestoreAction,
    escape_seen: bool,
}

#[test]
fn cancel_and_confirm_use_the_exact_token_once() {
    let clean = clean_draft();
    let mut cancelled = inspected_controller("cancel-token");
    assert!(cancelled.can_cancel_restore());
    cancelled
        .begin(
            Command::CancelRestore {
                token: "cancel-token".into(),
            },
            &clean,
        )
        .unwrap();
    let cancel = cancelled.take_ready_request(Some(&clean)).unwrap();
    assert!(matches!(
        cancel.command_ref(),
        Some(Command::CancelRestore { token }) if token == "cancel-token"
    ));
    assert!(!cancelled.can_cancel_restore());

    let mut confirmed = inspected_controller("confirm-token");
    confirmed
        .begin(
            Command::RestoreBackup {
                token: "confirm-token".into(),
            },
            &clean,
        )
        .unwrap();
    let restore = confirmed.take_ready_request(Some(&clean)).unwrap();
    assert!(matches!(
        restore.command_ref(),
        Some(Command::RestoreBackup { token }) if token == "confirm-token"
    ));
    assert!(confirmed.take_ready_request(Some(&clean)).is_none());
    let error = confirmed
        .begin(
            Command::RestoreBackup {
                token: "confirm-token".into(),
            },
            &clean,
        )
        .unwrap_err();
    assert_eq!(error.code, ErrorCode::StaleConfirmation);
}

#[test]
fn cancel_and_escape_are_disabled_while_replacement_runs() {
    let clean = clean_draft();
    let mut controller = inspected_controller("running-token");
    controller
        .begin(
            Command::RestoreBackup {
                token: "running-token".into(),
            },
            &clean,
        )
        .unwrap();
    let _request = controller.take_ready_request(Some(&clean)).unwrap();
    assert_eq!(controller.state(), &TransferState::Running);
    let mut harness = Harness::builder().with_size([720.0, 560.0]).build_ui_state(
        |ui, state| {
            state.escape_seen |= ui.ctx().input(|input| input.key_pressed(Key::Escape));
            let action = restore::show(ui.ctx(), state.controller.restore_dialog().unwrap(), false);
            if action != RestoreAction::None {
                state.action = action;
            }
        },
        DialogHarness {
            controller,
            request_focus: false,
            action: RestoreAction::None,
            escape_seen: false,
        },
    );

    assert!(harness
        .get_by_role_and_label(Role::Button, "Cancel restore")
        .accesskit_node()
        .is_disabled());
    assert!(harness
        .get_by_role_and_label(Role::Button, "Replace library")
        .accesskit_node()
        .is_disabled());
    harness.key_press(Key::Escape);
    harness.run();
    assert_eq!(harness.state().action, RestoreAction::None);
}

#[test]
fn restore_failure_exposes_the_phase_and_keeps_confirmation_consumed() {
    let clean = clean_draft();
    let mut controller = inspected_controller("failure-token");
    controller
        .begin(
            Command::RestoreBackup {
                token: "failure-token".into(),
            },
            &clean,
        )
        .unwrap();
    let request = controller.take_ready_request(Some(&clean)).unwrap();
    controller.complete(WorkerResult {
        request_id: request.request_id,
        outcome: WorkerOutcome::Command(Err(AppError::new(
            ErrorCode::Io,
            "Could not write the pre-restore safety backup.",
        ))),
    });

    assert_eq!(controller.state(), &TransferState::Idle);
    assert_eq!(
        controller.status(),
        Some("Library restore failed: Could not write the pre-restore safety backup.")
    );
    assert!(controller.restore_dialog().is_none());
    assert!(controller.take_data_focus_request());
}

#[test]
fn terminal_worker_failure_dismisses_restore_modal_and_unlocks() {
    let clean = clean_draft();
    let mut controller = inspected_controller("worker-stop-token");
    controller
        .begin(
            Command::RestoreBackup {
                token: "worker-stop-token".into(),
            },
            &clean,
        )
        .unwrap();
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
    assert!(controller.restore_dialog().is_none());
    assert_eq!(
        controller.status(),
        Some("The local library worker is not available.")
    );
}

struct RestoreTransfers {
    backup: Vec<u8>,
    fail_safety_backup: bool,
}

impl TransferIo for RestoreTransfers {
    fn pick_markdown(&mut self) -> Result<Outcome<Vec<NamedBytes>>, AppError> {
        Ok(Outcome::Cancelled)
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
        Ok(Outcome::Completed(self.backup.clone()))
    }

    fn write_safety_backup(&mut self, _store: &Store, _created_at: u64) -> Result<(), AppError> {
        if self.fail_safety_backup {
            Err(AppError::new(
                ErrorCode::Io,
                "Could not write the pre-restore safety backup.",
            ))
        } else {
            Ok(())
        }
    }
}

fn restore_app(
    fail_safety_backup: bool,
) -> (
    crane_desktop::app::CraneApp,
    std::sync::Arc<std::sync::Mutex<Store>>,
) {
    let replacement = Store {
        schema_version: 1,
        notes: vec![note("replacement", "Replacement", "Recovered", 20)],
        extensions: Default::default(),
    };
    let backup =
        serialize_backup(&make_backup(&replacement, 1_700_000_000_000, "0.1.0").unwrap()).unwrap();
    app_with_transfers(
        vec![note("original", "Original", "Keep me", 10)],
        Box::new(RestoreTransfers {
            backup,
            fail_safety_backup,
        }),
    )
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
    panic!("native restore did not settle");
}

fn inspect_backup(harness: &mut Harness<'_, crane_desktop::app::CraneApp>) {
    harness.get_by_label("Data").click();
    harness.run();
    harness.get_by_label("Restore backup…").click();
    settle(harness);
    harness.get_by_role_and_label(Role::Dialog, "Restore Crane backup");
}

#[test]
fn inspection_never_writes_and_cancel_returns_focus_to_data() {
    let (app, committed) = restore_app(false);
    let mut harness = Harness::builder()
        .with_size([1240.0, 780.0])
        .build_ui_state(|ui, app| app.show(ui), app);

    inspect_backup(&mut harness);
    assert_eq!(committed.lock().unwrap().notes[0].id, "original");
    harness.get_by_label("Cancel restore").click();
    settle(&mut harness);

    assert_eq!(committed.lock().unwrap().notes[0].id, "original");
    assert!(harness.get_by_label("Data").is_focused());
}

#[test]
fn failed_restore_keeps_the_old_library_and_names_the_failed_phase() {
    let (app, committed) = restore_app(true);
    let mut harness = Harness::builder()
        .with_size([1240.0, 780.0])
        .build_ui_state(|ui, app| app.show(ui), app);

    inspect_backup(&mut harness);
    harness.get_by_label("Replace library").click();
    settle(&mut harness);

    assert_eq!(committed.lock().unwrap().notes[0].id, "original");
    assert_eq!(
        harness.state().status(),
        Some("Library restore failed: Could not write the pre-restore safety backup.")
    );
    assert!(harness.get_by_label("Data").is_focused());
}

#[test]
fn completed_restore_refreshes_the_committed_and_visible_store() {
    let (app, committed) = restore_app(false);
    let mut harness = Harness::builder()
        .with_size([1240.0, 780.0])
        .build_ui_state(|ui, app| app.show(ui), app);

    inspect_backup(&mut harness);
    harness.get_by_label("Replace library").click();
    settle(&mut harness);

    assert_eq!(committed.lock().unwrap().notes[0].id, "replacement");
    assert_eq!(harness.state().selected_note_id(), Some("replacement"));
    assert_eq!(harness.state().selected_draft().unwrap().body, "Recovered");
}
