use crate::draft::{Draft, DraftCoordinator};
use crate::view_state::ViewState;
use crate::worker::{spawn_worker, WorkerHandle};
use crane_application::{ErrorCode, Library, LibrarySnapshot};
use eframe::egui;
use std::time::Instant;

pub struct CraneApp {
    draft: Option<DraftCoordinator>,
    worker: WorkerHandle,
    view: ViewState,
    started_at: Instant,
}

impl CraneApp {
    pub fn new(library: Library, snapshot: LibrarySnapshot, context: egui::Context) -> Self {
        let draft = snapshot.store.notes.first().map(|note| {
            DraftCoordinator::new(Draft {
                note_id: note.id.clone(),
                title: note.title.clone(),
                body: note.body.clone(),
                revision: 0,
                acknowledged_revision: 0,
            })
        });
        let wake_context = context.clone();
        let worker = spawn_worker(library, Box::new(move || wake_context.request_repaint()));
        Self {
            draft,
            worker,
            view: ViewState::default(),
            started_at: Instant::now(),
        }
    }

    fn now_millis(&self) -> u64 {
        self.started_at.elapsed().as_millis() as u64
    }

    fn poll_worker(&mut self) {
        while let Some(result) = self.worker.try_recv() {
            if let Some(draft) = self.draft.as_mut() {
                draft.complete(result);
                self.view.status = draft
                    .last_error()
                    .map(|error| error.message.clone())
                    .or_else(|| Some("Saved".into()));
            }
        }
        let now = self.now_millis();
        let Some(request) = self
            .draft
            .as_mut()
            .and_then(|draft| draft.take_ready_request(now))
        else {
            return;
        };
        if let Err(error) = self.worker.try_send(request.clone()) {
            if let Some(draft) = self.draft.as_mut() {
                if error.code == ErrorCode::QueueFull {
                    draft.return_unsent(request);
                } else {
                    draft.complete_failure(request.request_id, error.clone());
                }
            }
            self.view.status = Some(error.message);
        } else {
            self.view.status = Some("Saving…".into());
        }
    }
}

impl eframe::App for CraneApp {
    fn logic(&mut self, context: &egui::Context, _frame: &mut eframe::Frame) {
        let close_requested = context.input(|input| input.viewport().close_requested());
        if close_requested
            && self
                .draft
                .as_mut()
                .is_some_and(|draft| !draft.request_close())
        {
            context.send_viewport_cmd(egui::ViewportCommand::CancelClose);
        }
        self.poll_worker();
        if self.draft.as_ref().is_some_and(DraftCoordinator::can_close) {
            context.send_viewport_cmd(egui::ViewportCommand::Close);
        } else if let Some(delay) = self
            .draft
            .as_ref()
            .and_then(|draft| draft.next_save_delay_millis(self.now_millis()))
        {
            context.request_repaint_after(std::time::Duration::from_millis(delay));
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ui, |ui| {
            ui.heading("Crane");
            ui.label("Native preview — isolated test data");
            ui.separator();
            let now = self.now_millis();
            if let Some(editor) = self.draft.as_mut() {
                let mut title = editor.current().title.clone();
                let mut body = editor.current().body.clone();
                let title_changed = ui
                    .add(egui::TextEdit::singleline(&mut title).hint_text("Title"))
                    .changed();
                let body_changed = ui
                    .add(
                        egui::TextEdit::multiline(&mut body)
                            .desired_rows(24)
                            .hint_text("Write in Markdown…"),
                    )
                    .changed();
                if title_changed || body_changed {
                    editor.edit(title, body, now);
                    ui.ctx()
                        .request_repaint_after(std::time::Duration::from_millis(
                            crate::draft::SAVE_DELAY_MILLIS,
                        ));
                }
                if let Some(error) = editor.last_error() {
                    ui.colored_label(ui.visuals().error_fg_color, &error.message);
                    if ui.button("Retry save").clicked() {
                        editor.retry();
                    }
                }
            } else {
                ui.label("The isolated preview library has no notes.");
            }
            if let Some(status) = &self.view.status {
                ui.separator();
                ui.label(status);
            }
        });
    }
}
