use crate::worker::{WorkerRequest, WorkerResult};
use crane_application::{AppError, Command, CommandData, ErrorCode};
use std::sync::atomic::{AtomicU64, Ordering};

pub const SAVE_DELAY_MILLIS: u64 = 550;
static NEXT_DRAFT_REQUEST_ID: AtomicU64 = AtomicU64::new(1);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Draft {
    pub note_id: String,
    pub title: String,
    pub body: String,
    pub revision: u64,
    pub acknowledged_revision: u64,
}

impl Draft {
    pub fn edit(&mut self, title: String, body: String) {
        self.title = title;
        self.body = body;
        self.revision = self.revision.saturating_add(1);
    }

    pub fn acknowledge(&mut self, revision: u64) {
        let acknowledged = revision.min(self.revision);
        self.acknowledged_revision = self.acknowledged_revision.max(acknowledged);
    }

    pub fn is_dirty(&self) -> bool {
        self.revision > self.acknowledged_revision
    }
}

#[derive(Clone, Debug)]
enum SaveState {
    Idle,
    Queued(WorkerRequest),
    InFlight(WorkerRequest),
    Failed(AppError),
}

#[derive(Clone, Debug)]
enum Transition {
    Switch(Draft),
    Close,
}

#[derive(Clone)]
pub struct DraftCoordinator {
    current: Draft,
    state: SaveState,
    transition: Option<Transition>,
    close_ready: bool,
    last_edit_millis: u64,
}

impl DraftCoordinator {
    pub fn new(current: Draft) -> Self {
        Self {
            current,
            state: SaveState::Idle,
            transition: None,
            close_ready: false,
            last_edit_millis: 0,
        }
    }

    pub fn current(&self) -> &Draft {
        &self.current
    }

    pub fn current_mut(&mut self) -> &mut Draft {
        &mut self.current
    }

    pub fn mark_edited(&mut self, now_millis: u64) {
        self.current.revision = self.current.revision.saturating_add(1);
        self.last_edit_millis = now_millis;
        self.close_ready = false;
        if let SaveState::Queued(request) = &mut self.state {
            request.command = Command::SaveNote {
                id: self.current.note_id.clone(),
                title: self.current.title.clone(),
                body: self.current.body.clone(),
                draft_revision: self.current.revision,
            };
        }
    }

    pub fn edit(&mut self, title: String, body: String, now_millis: u64) {
        self.current.edit(title, body);
        self.last_edit_millis = now_millis;
        self.close_ready = false;
        if let SaveState::Queued(request) = &mut self.state {
            request.command = Command::SaveNote {
                id: self.current.note_id.clone(),
                title: self.current.title.clone(),
                body: self.current.body.clone(),
                draft_revision: self.current.revision,
            };
        }
    }

    pub fn request_switch(&mut self, next: Draft) -> bool {
        if self.can_transition_now() {
            self.current = next;
            return true;
        }
        self.transition = Some(Transition::Switch(next));
        self.queue_immediate_if_possible();
        false
    }

    pub fn request_close(&mut self) -> bool {
        if self.can_transition_now() {
            self.close_ready = true;
            return true;
        }
        self.transition = Some(Transition::Close);
        self.queue_immediate_if_possible();
        false
    }

    pub fn can_close(&self) -> bool {
        self.close_ready
    }

    pub fn last_error(&self) -> Option<&AppError> {
        match &self.state {
            SaveState::Failed(error) => Some(error),
            _ => None,
        }
    }

    pub fn next_save_delay_millis(&self, now_millis: u64) -> Option<u64> {
        if !matches!(self.state, SaveState::Idle) || !self.current.is_dirty() {
            return None;
        }
        let elapsed = now_millis.saturating_sub(self.last_edit_millis);
        Some(SAVE_DELAY_MILLIS.saturating_sub(elapsed))
    }

    pub fn take_ready_request(&mut self, now_millis: u64) -> Option<WorkerRequest> {
        if matches!(self.state, SaveState::Idle)
            && self.current.is_dirty()
            && now_millis.saturating_sub(self.last_edit_millis) >= SAVE_DELAY_MILLIS
        {
            self.queue_save();
        }
        let SaveState::Queued(request) = &self.state else {
            return None;
        };
        let request = request.clone();
        self.state = SaveState::InFlight(request.clone());
        Some(request)
    }

    pub fn return_unsent(&mut self, request: WorkerRequest) {
        if matches!(
            &self.state,
            SaveState::InFlight(current) if current.request_id == request.request_id
        ) {
            self.state = SaveState::Queued(request);
        }
    }

    pub fn complete(&mut self, result: WorkerResult) {
        match result.result {
            Ok(command_result) => match command_result.data {
                CommandData::Saved { draft_revision, .. } => {
                    self.complete_success(result.request_id, draft_revision);
                }
                _ => self.complete_failure(
                    result.request_id,
                    AppError::new(
                        ErrorCode::Busy,
                        "The editor received an unexpected worker result.",
                    ),
                ),
            },
            Err(error) => self.complete_failure(result.request_id, error),
        }
    }

    pub fn complete_success(&mut self, request_id: u64, draft_revision: u64) {
        if !self.matches_in_flight(request_id) {
            return;
        }
        self.current.acknowledge(draft_revision);
        self.state = SaveState::Idle;
        self.advance_transition();
    }

    pub fn complete_failure(&mut self, request_id: u64, error: AppError) {
        if !self.matches_in_flight(request_id) {
            return;
        }
        self.state = SaveState::Failed(error);
        self.close_ready = false;
    }

    pub fn retry(&mut self) -> bool {
        if !matches!(self.state, SaveState::Failed(_)) || !self.current.is_dirty() {
            return false;
        }
        self.state = SaveState::Idle;
        self.queue_save();
        true
    }

    pub fn save_now(&mut self) {
        self.queue_immediate_if_possible();
    }

    pub fn has_pending_work(&self) -> bool {
        !matches!(self.state, SaveState::Idle) || self.current.is_dirty()
    }

    fn can_transition_now(&self) -> bool {
        !self.current.is_dirty() && matches!(self.state, SaveState::Idle)
    }

    fn matches_in_flight(&self, request_id: u64) -> bool {
        matches!(
            &self.state,
            SaveState::InFlight(request) if request.request_id == request_id
        )
    }

    fn queue_immediate_if_possible(&mut self) {
        if matches!(self.state, SaveState::Idle) && self.current.is_dirty() {
            self.queue_save();
        }
    }

    fn queue_save(&mut self) {
        let request = WorkerRequest {
            request_id: NEXT_DRAFT_REQUEST_ID.fetch_add(1, Ordering::Relaxed),
            command: Command::SaveNote {
                id: self.current.note_id.clone(),
                title: self.current.title.clone(),
                body: self.current.body.clone(),
                draft_revision: self.current.revision,
            },
        };
        self.state = SaveState::Queued(request);
    }

    fn advance_transition(&mut self) {
        if self.current.is_dirty() {
            if self.transition.is_some() {
                self.queue_save();
            }
            return;
        }
        match self.transition.take() {
            Some(Transition::Switch(next)) => self.current = next,
            Some(Transition::Close) => self.close_ready = true,
            None => {}
        }
    }
}
