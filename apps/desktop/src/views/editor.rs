use crate::draft::DraftCoordinator;
use crane_domain::MarkdownDocument;
use eframe::egui;

pub fn editor_widget_id(note_id: &str, field: &str) -> egui::Id {
    egui::Id::unique(("crane-editor", note_id, field))
}

#[derive(Default)]
pub struct EditorActions {
    pub changed: bool,
    pub save_now: bool,
    pub retry: bool,
    pub toggle_preview: bool,
    pub toggle_pin: bool,
    pub toggle_archive: bool,
    pub toggle_trash: bool,
    pub link_error: Option<String>,
}

pub struct NoteFlags {
    pub pinned: bool,
    pub archived: bool,
    pub trashed: bool,
}

pub fn show(
    ui: &mut egui::Ui,
    draft: &mut DraftCoordinator,
    flags: NoteFlags,
    preview: bool,
    preview_document: Option<&MarkdownDocument>,
    request_title_focus: bool,
    status: Option<&str>,
) -> EditorActions {
    let mut actions = EditorActions::default();
    let note_id = draft.current().note_id.clone();
    ui.horizontal_wrapped(|ui| {
        actions.toggle_pin = ui
            .button(if flags.pinned {
                "Unpin note"
            } else {
                "Pin note"
            })
            .clicked();
        actions.toggle_archive = ui
            .button(if flags.archived {
                "Unarchive note"
            } else {
                "Archive note"
            })
            .clicked();
        actions.toggle_trash = ui
            .button(if flags.trashed {
                "Restore note"
            } else {
                "Move to Trash"
            })
            .clicked();
        actions.toggle_preview = ui
            .button(if preview {
                "Edit Markdown"
            } else {
                "Preview Markdown"
            })
            .clicked();
        actions.save_now = ui.button("Save now").clicked();
    });
    ui.separator();

    let title_response = ui
        .add(
            egui::TextEdit::singleline(&mut draft.current_mut().title)
                .id_salt(editor_widget_id(&note_id, "title"))
                .hint_text("Untitled note")
                .font(egui::TextStyle::Heading),
        )
        .accessible_name("Note title");
    if request_title_focus {
        title_response.request_focus();
    }
    actions.changed |= title_response.changed();

    if let Some(error) = draft.last_error() {
        ui.colored_label(ui.visuals().error_fg_color, &error.message);
        actions.retry = ui.button("Retry save").clicked();
    } else if let Some(status) = status {
        ui.small(status);
    }

    if preview {
        if let Some(document) = preview_document {
            actions.link_error = super::preview::show(ui, document);
        }
    } else {
        let body_size = ui.available_size();
        let body_response = ui
            .add_sized(
                body_size,
                egui::TextEdit::multiline(&mut draft.current_mut().body)
                    .id_salt(editor_widget_id(&note_id, "body"))
                    .hint_text("Write in Markdown…")
                    .desired_rows(24)
                    .font(egui::TextStyle::Body),
            )
            .accessible_name("Note body");
        actions.changed |= body_response.changed();
    }

    actions
}
