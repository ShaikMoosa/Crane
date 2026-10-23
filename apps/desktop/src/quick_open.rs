use crane_application::knowledge::{Query, QuickOpenHit};
use eframe::egui;
use egui::{Key, Modifiers};

#[derive(Clone, Debug, Default)]
pub struct QuickOpenState {
    pub open: bool,
    pub query: String,
    pub include_archived: bool,
    pub hits: Vec<QuickOpenHit>,
    pub active: usize,
    pub sequence: u64,
    pub focused: bool,
    pub needs_initial_query: bool,
    pub return_note: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum QuickOpenIntent {
    OpenNote(String),
}

pub fn open(state: &mut QuickOpenState, return_note: Option<String>) {
    state.open = true;
    state.query.clear();
    state.hits.clear();
    state.active = 0;
    state.focused = false;
    state.needs_initial_query = true;
    state.return_note = return_note;
}

pub fn close(state: &mut QuickOpenState) {
    state.open = false;
    state.query.clear();
    state.hits.clear();
    state.active = 0;
    state.focused = false;
    state.needs_initial_query = false;
}

fn request(state: &mut QuickOpenState, on_query: &mut dyn FnMut(Query)) {
    state.sequence = state.sequence.saturating_add(1);
    state.hits.clear();
    state.active = 0;
    on_query(Query::QuickOpen {
        query: state.query.clone(),
        include_archived: state.include_archived,
    });
}

pub fn show(
    ctx: &egui::Context,
    state: &mut QuickOpenState,
    on_query: &mut dyn FnMut(Query),
) -> Option<QuickOpenIntent> {
    if !state.open {
        return None;
    }
    let mut intent = None;
    let mut close_requested = false;
    egui::Window::new("Quick Open")
        .title_bar(false)
        .frame(crate::theme::overlay_frame(
            crate::theme::choice_for_context(ctx),
        ))
        .anchor(egui::Align2::CENTER_TOP, [0.0, 80.0])
        .min_width(480.0)
        .max_width(560.0)
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("Quick Open");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if crate::icons::icon_button(
                        ui,
                        crate::icons::IconId::X,
                        "Close Quick Open",
                        "Close Quick Open",
                    )
                    .clicked()
                    {
                        close_requested = true;
                    }
                });
            });
            ui.separator();
            if state.needs_initial_query {
                state.needs_initial_query = false;
                request(state, on_query);
            }
            let move_down =
                ui.input_mut(|input| input.consume_key(Modifiers::NONE, Key::ArrowDown));
            let move_up = ui.input_mut(|input| input.consume_key(Modifiers::NONE, Key::ArrowUp));
            let accept = ui.input_mut(|input| input.consume_key(Modifiers::NONE, Key::Enter));
            let cancel = ui.input_mut(|input| input.consume_key(Modifiers::NONE, Key::Escape));
            if move_down && !state.hits.is_empty() {
                state.active = (state.active + 1).min(state.hits.len() - 1);
            }
            if move_up {
                state.active = state.active.saturating_sub(1);
            }
            let search = ui
                .add(
                    egui::TextEdit::singleline(&mut state.query)
                        .hint_text("Type to search notes…")
                        .desired_width(f32::INFINITY),
                )
                .accessible_name("Quick Open");
            if !state.focused {
                search.request_focus();
                state.focused = true;
            }
            if search.changed() {
                request(state, on_query);
            }
            let mut include_archived = state.include_archived;
            if ui
                .checkbox(&mut include_archived, "Include archived")
                .changed()
            {
                state.include_archived = include_archived;
                request(state, on_query);
            }
            egui::ScrollArea::vertical()
                .max_height(320.0)
                .show(ui, |ui| {
                    for (index, hit) in state.hits.iter().enumerate() {
                        let selected = index == state.active;
                        ui.horizontal(|ui| {
                            let response =
                                ui.add(egui::Button::selectable(selected, hit.title.as_str()));
                            if hit.archived {
                                ui.small("archived");
                            }
                            if response.clicked() {
                                state.active = index;
                                intent = Some(QuickOpenIntent::OpenNote(hit.note_id.clone()));
                                close_requested = true;
                            }
                        });
                    }
                });
            if accept {
                if let Some(hit) = state.hits.get(state.active) {
                    intent = Some(QuickOpenIntent::OpenNote(hit.note_id.clone()));
                    close_requested = true;
                }
            }
            if cancel {
                close_requested = true;
            }
        });
    if close_requested {
        close(state);
    }
    intent
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hit(id: &str) -> QuickOpenHit {
        QuickOpenHit {
            note_id: id.to_owned(),
            title: id.to_owned(),
            excerpt: String::new(),
            updated_at: 0,
            pinned: false,
            archived: false,
        }
    }

    #[test]
    fn requesting_new_results_clears_stale_hits_before_enter_can_accept_them() {
        let mut state = QuickOpenState {
            query: "new query".into(),
            hits: vec![hit("stale")],
            ..QuickOpenState::default()
        };
        let mut emitted = Vec::new();

        request(&mut state, &mut |query| emitted.push(query));

        assert!(state.hits.is_empty());
        assert_eq!(state.active, 0);
        assert_eq!(emitted.len(), 1);
    }
}
