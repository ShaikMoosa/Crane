use crane_domain::{Note, Store};
use eframe::egui;

pub fn show(
    ui: &mut egui::Ui,
    store: &Store,
    visible_ids: &[String],
    selected_id: Option<&str>,
    query: &mut String,
    request_search_focus: bool,
    heading: &str,
) -> Option<String> {
    ui.heading(heading);
    let search = ui
        .add(egui::TextEdit::singleline(query).hint_text("Search notes"))
        .accessible_name("Search notes");
    if request_search_focus {
        search.request_focus();
    }
    ui.separator();
    let mut selected = None;
    egui::ScrollArea::vertical().show(ui, |ui| {
        for id in visible_ids {
            let Some(note) = store.notes.iter().find(|note| note.id == *id) else {
                continue;
            };
            if note_row(ui, note, selected_id == Some(id.as_str())).clicked() {
                selected = Some(id.clone());
            }
        }
    });
    selected
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
