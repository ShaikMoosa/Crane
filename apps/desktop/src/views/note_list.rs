use crane_domain::{Note, Store};
use eframe::egui;

pub struct NoteListAction {
    pub selected: Option<String>,
    pub quick_open: bool,
}

pub fn show(
    ui: &mut egui::Ui,
    store: &Store,
    visible_ids: &[String],
    selected_id: Option<&str>,
    query: &mut String,
    request_search_focus: bool,
    heading: &str,
) -> NoteListAction {
    let mut action = NoteListAction {
        selected: None,
        quick_open: false,
    };
    ui.horizontal(|ui| {
        ui.heading(heading);
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if crate::icons::icon_button(
                ui,
                crate::icons::IconId::Search,
                "Quick Open",
                "Quick Open",
            )
            .clicked()
            {
                action.quick_open = true;
            }
        });
    });
    let search = ui
        .add(egui::TextEdit::singleline(query).hint_text("Search notes"))
        .accessible_name("Search notes");
    if request_search_focus {
        search.request_focus();
    }
    ui.separator();
    egui::ScrollArea::vertical().show(ui, |ui| {
        for id in visible_ids {
            let Some(note) = store.notes.iter().find(|note| note.id == *id) else {
                continue;
            };
            if note_row(ui, note, selected_id == Some(id.as_str())).clicked() {
                action.selected = Some(id.clone());
            }
        }
    });
    action
}

fn note_row(ui: &mut egui::Ui, note: &Note, selected: bool) -> egui::Response {
    let title = if note.title.trim().is_empty() {
        "Untitled note"
    } else {
        &note.title
    };
    let excerpt = note
        .body
        .split_whitespace()
        .take(12)
        .collect::<Vec<_>>()
        .join(" ");
    ui.vertical(|ui| {
        let response = ui.selectable_label(selected, title);
        if !excerpt.is_empty() {
            ui.small(excerpt);
        }
        let words = note.body.split_whitespace().count();
        ui.small(format!("{words} words · updated {}", note.updated_at));
        response
    })
    .inner
}
