use crate::draft::{DraftCoordinator, SavePresentation};
use crate::formatting::FormattingState;
use crate::icons::{icon_button, IconId};
use crate::knowledge_navigation::{byte_to_char, char_to_byte, CompletionState, WikiFragment};
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
    pub toggle_focus_mode: bool,
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

pub struct FormattingView<'a> {
    pub state: &'a mut FormattingState,
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
    pub note_actions_open: &'a mut bool,
    pub completion: Option<CompletionView<'a>>,
    pub formatting: Option<FormattingView<'a>>,
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
        note_actions_open,
        mut completion,
        mut formatting,
    } = view;
    let mut actions = EditorActions::default();
    let note_id = draft.current().note_id.clone();
    ui.horizontal_wrapped(|ui| {
        actions.toggle_focus_mode = icon_button(
            ui,
            IconId::PanelLeft,
            "Toggle focus mode",
            "Toggle focus mode",
        )
        .clicked();
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
        let save_label = match draft.save_presentation() {
            SavePresentation::Saved => "Saved",
            SavePresentation::Saving => "Saving",
            SavePresentation::Failed => "Save failed",
        };
        ui.weak(save_label);

        let menu_icon =
            crate::icons::icon_for_text_button(ui, IconId::Ellipsis, ui.visuals().text_color());
        let note_actions = ui
            .add(egui::Button::image(menu_icon).min_size(egui::vec2(38.0, 38.0)))
            .accessible_name("Note actions")
            .on_hover_text("Note actions");
        note_actions.widget_info(|| {
            egui::WidgetInfo::labeled(egui::Role::DisclosureTriangle, true, "Note actions")
        });
        ui.ctx().accesskit_node_builder(note_actions.id, |node| {
            node.set_expanded(*note_actions_open)
        });
        if note_actions.clicked() {
            *note_actions_open = !*note_actions_open;
        }

        let pin_name = if flags.pinned {
            "Unpin note"
        } else {
            "Pin note"
        };
        let archive_name = if flags.archived {
            "Unarchive note"
        } else {
            "Archive note"
        };
        let trash_name = if flags.trashed {
            "Restore note"
        } else {
            "Move to Trash"
        };
        let mut picked = None;
        egui::Popup::menu(&note_actions)
            .open_bool(note_actions_open)
            .show(|ui| {
                ui.set_min_width(176.0);
                for (label, action) in [
                    (pin_name, 0_u8),
                    (archive_name, 1),
                    (trash_name, 2),
                    ("Export note", 3),
                    ("Save now", 4),
                ] {
                    let text = if label == "Move to Trash" {
                        egui::RichText::new(label).color(ui.visuals().error_fg_color)
                    } else {
                        egui::RichText::new(label)
                    };
                    if ui
                        .add_enabled(
                            mutations_enabled,
                            egui::Button::new(text).min_size(egui::vec2(164.0, 44.0)),
                        )
                        .clicked()
                    {
                        picked = Some(action);
                    }
                }
            });
        if let Some(action) = picked {
            *note_actions_open = false;
            match action {
                0 => actions.toggle_pin = true,
                1 => actions.toggle_archive = true,
                2 => actions.toggle_trash = true,
                3 => actions.export_note = true,
                4 => actions.save_now = true,
                _ => unreachable!(),
            }
        }
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
    } else if let Some(status) = status.filter(|status| *status != "Saved") {
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
            &mut formatting,
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
    formatting: &mut Option<FormattingView<'_>>,
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

    if let Some(format_view) = formatting.as_mut() {
        let live_range = TextEditState::load(ui.ctx(), response_id)
            .and_then(|state| state.cursor.char_range())
            .map(|range| (range.primary.index.0, range.secondary.index.0));
        if let Some(action) = crate::formatting::show(ui, format_view.state, mutations_enabled) {
            let selected = match live_range {
                Some((a, b)) if a != b => (a, b),
                _ => format_view.state.last_char_range.unwrap_or((0, 0)),
            };
            actions.changed |= apply_format_action(
                ui.ctx(),
                response_id,
                &mut draft.current_mut().body,
                selected,
                &action,
                &body_response,
            );
        }
        format_view.state.last_char_range = live_range;
    }
}

fn snapshot_undo_point(ctx: &egui::Context, body_id: egui::Id, body: &str) {
    if let Some(text_state) = TextEditState::load(ctx, body_id) {
        if let Some(range) = text_state.cursor.char_range() {
            let mut undoer = text_state.undoer();
            undoer.add_undo(&(range, body.to_owned()));
            let mut text_state = text_state;
            text_state.set_undoer(undoer);
            text_state.store(ctx, body_id);
        }
    }
}

fn apply_format_action(
    ctx: &egui::Context,
    response_id: egui::Id,
    body: &mut String,
    selected: (usize, usize),
    action: &crate::formatting::FormatAction,
    body_response: &egui::Response,
) -> bool {
    let start_char = selected.0.min(selected.1);
    let end_char = selected.0.max(selected.1);
    let byte_range = char_to_byte(body, start_char)..char_to_byte(body, end_char);
    let outcome = crate::formatting::apply_format(body, byte_range, action);
    if !outcome.applied {
        return false;
    }
    snapshot_undo_point(ctx, response_id, body);
    *body = outcome.body;
    let new_start = byte_to_char(body, outcome.selection.start);
    let new_end = byte_to_char(body, outcome.selection.end);
    if let Some(mut text_state) = TextEditState::load(ctx, response_id) {
        let range = if new_start == new_end {
            CCursorRange::one(CCursor::new(new_start))
        } else {
            CCursorRange::two(CCursor::new(new_start), CCursor::new(new_end))
        };
        text_state.cursor.set_char_range(Some(range));
        text_state.store(ctx, response_id);
    }
    body_response.request_focus();
    true
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
    snapshot_undo_point(ctx, body_id, body);
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
