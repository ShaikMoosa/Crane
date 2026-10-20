use crate::draft::Draft;
use crate::worker::{WorkerRequest, WorkerResult};
use crane_application::{AppError, Command, CommandData, ErrorCode};
use crane_domain::{BackupInspection, Store};

const FIRST_TRANSFER_REQUEST_ID: u64 = 1 << 62;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TransferState {
    Idle,
    Flushing,
    Running,
    AwaitingConfirmation {
        token: String,
        inspection: BackupInspection,
    },
}

#[derive(Clone, Debug)]
pub enum TransferEffect {
    None,
    ReplaceStore(Store),
}

#[derive(Clone, Copy)]
pub struct RestoreDialogModel<'a> {
    pub inspection: &'a BackupInspection,
    pub restoring: bool,
}

pub struct TransferController {
    state: TransferState,
    pending_command: Option<Command>,
    running_request_id: Option<u64>,
    running_command: Option<Command>,
    next_request_id: u64,
    status: Option<String>,
    restore_inspection: Option<BackupInspection>,
    request_data_focus: bool,
}

impl Default for TransferController {
    fn default() -> Self {
        Self {
            state: TransferState::Idle,
            pending_command: None,
            running_request_id: None,
            running_command: None,
            next_request_id: FIRST_TRANSFER_REQUEST_ID,
            status: None,
            restore_inspection: None,
            request_data_focus: false,
        }
    }
}

impl TransferController {
    pub fn state(&self) -> &TransferState {
        &self.state
    }

    pub fn status(&self) -> Option<&str> {
        self.status.as_deref()
    }

    pub fn can_mutate(&self) -> bool {
        matches!(self.state, TransferState::Idle)
    }

    pub fn can_cancel_restore(&self) -> bool {
        matches!(self.state, TransferState::AwaitingConfirmation { .. })
    }

    pub fn has_pending_work(&self) -> bool {
        matches!(self.state, TransferState::Flushing | TransferState::Running)
    }

    pub fn begin(&mut self, command: Command, _draft: &Draft) -> Result<(), AppError> {
        self.begin_command(command)
    }

    pub fn begin_without_draft(&mut self, command: Command) -> Result<(), AppError> {
        self.begin_command(command)
    }

    fn begin_command(&mut self, command: Command) -> Result<(), AppError> {
        if !is_transfer_command(&command) {
            return Err(AppError::new(
                ErrorCode::Validation,
                "That operation is not a file transfer.",
            ));
        }

        if matches!(
            command,
            Command::RestoreBackup { .. } | Command::CancelRestore { .. }
        ) {
            let TransferState::AwaitingConfirmation { token, .. } = &self.state else {
                return Err(stale_confirmation());
            };
            let supplied = match &command {
                Command::RestoreBackup { token } | Command::CancelRestore { token } => token,
                _ => unreachable!(),
            };
            if supplied != token {
                return Err(stale_confirmation());
            }
        } else if !matches!(self.state, TransferState::Idle) {
            return Ok(());
        }

        self.pending_command = Some(command);
        self.status = None;
        self.request_data_focus = false;
        self.state = TransferState::Flushing;
        Ok(())
    }

    pub fn abort_prerequisite_save(&mut self) {
        if matches!(self.state, TransferState::Flushing) {
            self.pending_command = None;
            self.state = TransferState::Idle;
            self.status = None;
            self.request_data_focus = true;
        }
    }

    pub fn take_ready_request(&mut self, draft: Option<&Draft>) -> Option<WorkerRequest> {
        if !matches!(self.state, TransferState::Flushing) || draft.is_some_and(Draft::is_dirty) {
            return None;
        }
        let command = self.pending_command.take()?;
        let request_id = self.next_request_id;
        self.next_request_id = self.next_request_id.saturating_add(1);
        self.running_request_id = Some(request_id);
        self.running_command = Some(command.clone());
        self.state = TransferState::Running;
        Some(WorkerRequest {
            request_id,
            command,
        })
    }

    pub fn return_unsent(&mut self, request: WorkerRequest, error: AppError) {
        if self.running_request_id == Some(request.request_id) {
            self.running_request_id = None;
            self.running_command = None;
            if error.code == ErrorCode::QueueFull {
                self.pending_command = Some(request.command);
                self.state = TransferState::Flushing;
            } else {
                self.pending_command = None;
                self.state = TransferState::Idle;
                self.restore_inspection = None;
                self.request_data_focus = true;
            }
            self.status = Some(error.message);
        }
    }

    pub fn owns_request(&self, request_id: u64) -> bool {
        self.running_request_id == Some(request_id)
    }

    pub fn complete(&mut self, result: WorkerResult) -> TransferEffect {
        if !self.owns_request(result.request_id) {
            return TransferEffect::None;
        }
        let command = self.running_command.take();
        self.running_request_id = None;
        match result.result {
            Ok(command_result) => self.complete_success(command, command_result.data),
            Err(error) => {
                self.state = TransferState::Idle;
                self.restore_inspection = None;
                self.request_data_focus = true;
                self.status = Some(format_error(command.as_ref(), &error.message));
                TransferEffect::None
            }
        }
    }

    fn complete_success(&mut self, command: Option<Command>, data: CommandData) -> TransferEffect {
        match data {
            CommandData::Inspection { token, inspection } => {
                self.restore_inspection = Some(inspection.clone());
                self.state = TransferState::AwaitingConfirmation { token, inspection };
                self.status = None;
                TransferEffect::None
            }
            CommandData::Imported {
                store,
                imported_count,
            } => {
                self.finish(format_count(imported_count, "note", "notes", "imported"));
                TransferEffect::ReplaceStore(store)
            }
            CommandData::Restored(store) => {
                self.finish("Library restored".into());
                TransferEffect::ReplaceStore(store)
            }
            CommandData::Exported(summary) => {
                let message = if matches!(command, Some(Command::ExportNote { .. })) {
                    "Note exported".into()
                } else if summary.folder_name.is_empty() {
                    format_count(summary.exported_count, "note", "notes", "exported")
                } else {
                    format!(
                        "{} to {}",
                        format_count(summary.exported_count, "note", "notes", "exported"),
                        summary.folder_name
                    )
                };
                self.finish(message);
                TransferEffect::None
            }
            CommandData::BackupCreated { note_count } => {
                self.finish(format!(
                    "Backup created for {}",
                    if note_count == 1 {
                        "1 note".into()
                    } else {
                        format!("{note_count} notes")
                    }
                ));
                TransferEffect::None
            }
            CommandData::Cancelled | CommandData::RestoreCancelled => {
                self.state = TransferState::Idle;
                self.restore_inspection = None;
                self.status = None;
                self.request_data_focus = true;
                TransferEffect::None
            }
            _ => {
                self.state = TransferState::Idle;
                self.restore_inspection = None;
                self.status = Some("The transfer returned an unexpected result.".into());
                self.request_data_focus = true;
                TransferEffect::None
            }
        }
    }

    fn finish(&mut self, message: String) {
        self.state = TransferState::Idle;
        self.restore_inspection = None;
        self.status = Some(message);
        self.request_data_focus = true;
    }

    pub fn restore_dialog(&self) -> Option<RestoreDialogModel<'_>> {
        self.restore_inspection
            .as_ref()
            .map(|inspection| RestoreDialogModel {
                inspection,
                restoring: matches!(self.running_command, Some(Command::RestoreBackup { .. })),
            })
    }

    pub fn take_data_focus_request(&mut self) -> bool {
        std::mem::take(&mut self.request_data_focus)
    }
}

fn is_transfer_command(command: &Command) -> bool {
    matches!(
        command,
        Command::ImportMarkdown
            | Command::ExportNote { .. }
            | Command::ExportLibrary
            | Command::CreateBackup
            | Command::InspectBackup
            | Command::RestoreBackup { .. }
            | Command::CancelRestore { .. }
    )
}

fn format_count(count: usize, singular: &str, plural: &str, verb: &str) -> String {
    let noun = if count == 1 { singular } else { plural };
    format!("{count} {noun} {verb}")
}

fn format_error(command: Option<&Command>, message: &str) -> String {
    let phase = match command {
        Some(Command::ImportMarkdown) => "Markdown import failed",
        Some(Command::ExportNote { .. }) => "Note export failed",
        Some(Command::ExportLibrary) => "Library export failed",
        Some(Command::CreateBackup) => "Backup creation failed",
        Some(Command::InspectBackup) => "Backup inspection failed",
        Some(Command::RestoreBackup { .. }) => "Library restore failed",
        Some(Command::CancelRestore { .. }) => "Restore cancellation failed",
        _ => "Transfer failed",
    };
    format!("{phase}: {message}")
}

fn stale_confirmation() -> AppError {
    AppError::new(
        ErrorCode::StaleConfirmation,
        "That restore confirmation is no longer valid.",
    )
}
