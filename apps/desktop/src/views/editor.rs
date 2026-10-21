use crate::draft::DraftCoordinator;
use crate::icons::{icon_button, IconId};
use crate::knowledge_navigation::{char_to_byte, CompletionState, WikiFragment};
use crate::views::preview::PreviewAction;
use crane_application::knowledge::Query;
use crane_domain::knowledge::WikiLink;
use crane_domain::MarkdownDocument;
use eframe::egui;
use eframe::egui::{
    text::{CCursor, CCursorRange},
    widgets::text_edit::TextEditState,
};
use eframe::egui::{Event, ImeEvent, Key, Modifiers};

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
    pub export_note: bool,
    pub link_error: Option<String>,
    pub wiki_link: Option<WikiLink>,
    pub toggle_inspector: bool,
}

#[derive(Clone, Copy)]
pub struct NoteFlags {
    pub pinned: bool,
    pub archived: bool,
    pub trashed: bool,
}

pub struct CompletionView<'a> {
    pub state: &'a mut CompletionState,
    pub emit_query: &'a mut dyn FnMut(Query),
}

pub struct EditorView<'a> {
    pub flags: NoteFlags,
    pub preview: bool,
    pub preview_document: Option<&'a MarkdownDocument>,
    pub preview_scroll_anchor: Option<&'a str>,
    pub request_title_focus: bool,
    pub request_body_focus: bool,
    /// Char index to jump the body caret to (wiki-link heading navigation).
    pub caret_jump: Option<usize>,
    pub status: Option<&'a str>,
    pub mutations_enabled: bool,
    pub completion: Option<CompletionView<'a>>,
}

pub fn show(
    ui: &mut egui::Ui,
    draft: &mut DraftCoordinator,
    view: EditorView<'_>,
) -> EditorActions {
    let EditorView {
        flags,
        preview,
        preview_document,
        preview_scroll_anchor,
        request_title_focus,
        mut request_body_focus,
        caret_jump,
        status,
        mutations_enabled,
        mut completion,
    } = view;
    let mut actions = EditorActions::default();
    let note_id = draft.current().note_id.clone();
    ui.horizontal_wrapped(|ui| {
        let pin_name = if flags.pinned {
            "Unpin note"
        } else {
            "Pin note"
        };
        actions.toggle_pin =
            icon_button(ui, IconId::Pin, pin_name, pin_name).clicked() && mutations_enabled;
        let archive_name = if flags.archived {
            "Unarchive note"
        } else {
            "Archive note"
        };
        actions.toggle_archive = icon_button(ui, IconId::Archive, archive_name, archive_name)
            .clicked()
            && mutations_enabled;
        let trash_name = if flags.trashed {
            "Restore note"
        } else {
            "Move to Trash"
        };
        actions.toggle_trash =
            icon_button(ui, IconId::Trash, trash_name, trash_name).clicked() && mutations_enabled;
        let preview_name = if preview {
            "Edit Markdown"
        } else {
            "Preview Markdown"
        };
        let preview_icon = if preview { IconId::Pencil } else { IconId::Eye };
        actions.toggle_preview = icon_button(ui, preview_icon, preview_name, preview_name)
            .clicked()
            && mutations_enabled;
        actions.toggle_inspector =
            icon_button(ui, IconId::Info, "Note information", "Note information").clicked();
        actions.export_note = ui
            .add_enabled(
                mutations_enabled,
                egui::Button::new("Export note").min_size(egui::vec2(44.0, 44.0)),
            )
            .clicked();
        actions.save_now = ui
            .add_enabled(mutations_enabled, egui::Button::new("Save now"))
            .clicked();
    });
    ui.separator();

    let title_response = ui
        .add_enabled(
            mutations_enabled,
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
            match super::preview::show(ui, document, preview_scroll_anchor) {
                PreviewAction::None => {}
                PreviewAction::HttpError(error) => {
                    actions.link_error = Some(error);
                }
                PreviewAction::WikiLink(link) => {
                    actions.wiki_link = Some(link);
                }
            }
        }
    } else {
        if caret_jump.is_some() {
            request_body_focus = true;
        }
        show_body(
            ui,
            draft,
            &note_id,
            request_body_focus,
            caret_jump,
            mutations_enabled,
            &mut completion,
            &mut actions,
        );
    }

    actions
}

#[allow(clippy::too_many_arguments)]
fn show_body(
    ui: &mut egui::Ui,
    draft: &mut DraftCoordinator,
    note_id: &str,
    request_body_focus: bool,
    caret_jump: Option<usize>,
    mutations_enabled: bool,
    completion: &mut Option<CompletionView<'_>>,
    actions: &mut EditorActions,
) {
    let mut accept_index = None;
    let mut cancel_popup = false;
    if let Some(view) = completion.as_mut() {
        track_ime(ui, view.state);
        if view.state.popup_visible() {
            let move_down =
                ui.input_mut(|input| input.consume_key(Modifiers::NONE, Key::ArrowDown));
            let move_up = ui.input_mut(|input| input.consume_key(Modifiers::NONE, Key::ArrowUp));
            let accept = ui.input_mut(|input| input.consume_key(Modifiers::NONE, Key::Enter));
            let cancel = ui.input_mut(|input| input.consume_key(Modifiers::NONE, Key::Escape));
            let len = view.state.candidates.len();
            if move_down && len > 0 {
                view.state.active = (view.state.active + 1) % len;
            }
            if move_up && len > 0 {
                view.state.active = (view.state.active + len - 1) % len;
            }
            if accept {
                accept_index = Some(view.state.active);
            }
            cancel_popup = cancel;
        }
    }

    let body_size = ui.available_size();
    let body_id = editor_widget_id(note_id, "body");
    let body_response = ui
        .add_enabled(
            mutations_enabled,
            egui::TextEdit::multiline(&mut draft.current_mut().body)
                .id_salt(body_id)
                .hint_text("Write in Markdown…")
                .desired_rows(24)
                .font(egui::TextStyle::Body)
                .desired_width(body_size.x),
        )
        .accessible_name("Note body");
    if request_body_focus {
        body_response.request_focus();
    }
    actions.changed |= body_response.changed();
    let response_id = body_response.id;
    if let Some(char_index) = caret_jump {
        if let Some(mut text_state) = TextEditState::load(ui.ctx(), response_id) {
            text_state
                .cursor
                .set_char_range(Some(CCursorRange::one(CCursor::new(char_index))));
            text_state.store(ui.ctx(), response_id);
        }
    }

    let Some(view) = completion.as_mut() else {
        return;
    };
    if cancel_popup {
        view.state.dismiss();
    } else {
        let fragment = caret_fragment(ui.ctx(), response_id, &draft.current().body);
        view.state.set_fragment(fragment, view.emit_query);
    }
    if let Some(index) = accept_index {
        actions.changed |= apply_completion_at(
            ui.ctx(),
            response_id,
            &mut draft.current_mut().body,
            view.state,
            index,
            &body_response,
        );
    }
    if view.state.popup_visible() {
        let anchor = body_response.rect.left_bottom() + egui::vec2(0.0, 4.0);
        let mut clicked = None;
        egui::Area::new(egui::Id::unique("completion-popup"))
            .fixed_pos(anchor)
            .show(ui.ctx(), |ui| {
                egui::Frame::popup(ui.style()).show(ui, |ui| {
                    ui.set_min_width(240.0);
                    for (index, candidate) in view.state.candidates.iter().enumerate() {
                        let text = candidate.insert_text().to_owned();
                        let name = format!("Complete link: {text}");
                        let selected = index == view.state.active;
                        if ui
                            .add(egui::Button::new(&text).selected(selected))
                            .accessible_name(&name)
                            .clicked()
                        {
                            clicked = Some(index);
                        }
                    }
                });
            });
        if let Some(index) = clicked {
            actions.changed |= apply_completion_at(
                ui.ctx(),
                response_id,
                &mut draft.current_mut().body,
                view.state,
                index,
                &body_response,
            );
        }
    }
}

fn track_ime(ui: &egui::Ui, state: &mut CompletionState) {
    for event in ui.input(|input| input.filtered_events(&egui::EventFilter::default())) {
        match event {
            Event::Ime(ImeEvent::Preedit { text, .. }) => {
                state.ime_composing = !text.is_empty();
            }
            Event::Ime(_) => state.ime_composing = false,
            _ => {}
        }
    }
}

fn caret_fragment(ctx: &egui::Context, body_id: egui::Id, body: &str) -> Option<WikiFragment> {
    let caret_char = TextEditState::load(ctx, body_id)
        .and_then(|state| state.cursor.char_range())
        .and_then(|range| range.single())
        .map(|cursor| cursor.index.0)?;
    let caret_byte = char_to_byte(body, caret_char);
    crate::knowledge_navigation::wiki_fragment_at(body, caret_byte)
}

fn apply_completion_at(
    ctx: &egui::Context,
    body_id: egui::Id,
    body: &mut String,
    state: &mut CompletionState,
    index: usize,
    body_response: &egui::Response,
) -> bool {
    // Snapshot the pre-completion state as an explicit undo point: the
    // automatic undoer only banks points after the text rests, so without
    // this Ctrl+Z would jump past the pre-completion body.
    if let Some(text_state) = TextEditState::load(ctx, body_id) {
        if let Some(range) = text_state.cursor.char_range() {
            let mut undoer = text_state.undoer();
            undoer.add_undo(&(range, body.clone()));
            let mut text_state = text_state;
            text_state.set_undoer(undoer);
            text_state.store(ctx, body_id);
        }
    }
    let Some(caret_char) = state.apply_candidate(body, index) else {
        return false;
    };
    if let Some(mut text_state) = TextEditState::load(ctx, body_id) {
        text_state
            .cursor
            .set_char_range(Some(CCursorRange::one(CCursor::new(caret_char))));
        text_state.store(ctx, body_id);
    }
    body_response.request_focus();
    true
}
