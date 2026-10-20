use crane_domain::{Collection, Store};
use crane_storage::ThemeChoice;
use eframe::egui;

#[derive(Default)]
pub struct LibraryActions {
    pub create: bool,
    pub collection: Option<Collection>,
    pub tag: Option<Option<String>>,
    pub toggle_theme: bool,
    pub reduced_motion: Option<bool>,
}

pub fn show(
    ui: &mut egui::Ui,
    store: &Store,
    active_collection: Collection,
    active_tag: Option<&str>,
    theme: ThemeChoice,
    reduced_motion: bool,
    collapsed: bool,
) -> LibraryActions {
    let mut actions = LibraryActions::default();
    let mut tags: Vec<_> = store
        .notes
        .iter()
        .flat_map(|note| note.tags.iter().cloned())
        .collect();
    tags.sort();
    tags.dedup();

    let footer_height = 96.0;
    let content_size = egui::vec2(
        ui.available_width(),
        (ui.available_height() - footer_height).max(0.0),
    );
    ui.allocate_ui_with_layout(
        content_size,
        egui::Layout::top_down(egui::Align::Min),
        |ui| {
            ui.heading(if collapsed { "C" } else { "Crane" });
            if ui
                .button(if collapsed { "+" } else { "+  New note" })
                .accessible_name("New note")
                .clicked()
            {
                actions.create = true;
            }
            ui.separator();
            for (collection, label, short) in [
                (Collection::Notes, "Notes", "N"),
                (Collection::Pinned, "Pinned", "P"),
                (Collection::Archive, "Archive", "A"),
                (Collection::Trash, "Trash", "T"),
            ] {
                let count = store
                    .notes
                    .iter()
                    .filter(|note| match collection {
                        Collection::Notes => !note.archived && !note.trashed,
                        Collection::Pinned => note.pinned && !note.archived && !note.trashed,
                        Collection::Archive => note.archived && !note.trashed,
                        Collection::Trash => note.trashed,
                    })
                    .count();
                let expanded = format!("{label}  {count}");
                let text = if collapsed { short } else { &expanded };
                let response = ui.selectable_label(active_collection == collection, text);
                let response = if collapsed {
                    response.on_hover_text(label)
                } else {
                    response
                };
                response.widget_info(|| {
                    egui::WidgetInfo::selected(
                        egui::Role::Button,
                        true,
                        active_collection == collection,
                        label,
                    )
                });
                if response.clicked() {
                    actions.collection = Some(collection);
                    actions.tag = Some(None);
                }
            }
            if !tags.is_empty() {
                ui.separator();
                if !collapsed {
                    ui.small("TAGS");
                }
                egui::ScrollArea::vertical()
                    .id_salt("crane-library-tags")
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        for tag in tags {
                            let label = format!("#{tag}");
                            let response = ui
                                .selectable_label(
                                    active_tag == Some(tag.as_str()),
                                    if collapsed { "#" } else { &label },
                                )
                                .accessible_name(&label);
                            if response.clicked() {
                                actions.collection = Some(Collection::Notes);
                                actions.tag = Some(Some(tag));
                            }
                        }
                    });
            }
            ui.allocate_space(ui.available_size());
        },
    );
    ui.separator();
    let theme_label = match theme {
        ThemeChoice::Dark => "Use light theme",
        ThemeChoice::Light => "Use dark theme",
    };
    actions.toggle_theme = ui
        .button(if collapsed {
            match theme {
                ThemeChoice::Dark => "L",
                ThemeChoice::Light => "D",
            }
        } else {
            theme_label
        })
        .accessible_name(theme_label)
        .clicked();
    let mut reduced = reduced_motion;
    let reduced_response = ui
        .checkbox(&mut reduced, if collapsed { "" } else { "Reduce motion" })
        .accessible_name("Reduce motion");
    if reduced_response.changed() {
        actions.reduced_motion = Some(reduced);
    }
    actions
}
