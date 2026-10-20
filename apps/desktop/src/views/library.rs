use crane_domain::{Collection, Store};
use crane_storage::ThemeChoice;
use eframe::egui;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DataAction {
    ImportMarkdown,
    ExportLibrary,
    CreateBackup,
    InspectBackup,
}

#[derive(Default)]
pub struct LibraryActions {
    pub create: bool,
    pub collection: Option<Collection>,
    pub tag: Option<Option<String>>,
    pub toggle_theme: bool,
    pub reduced_motion: Option<bool>,
    pub transfer: Option<DataAction>,
}

pub struct LibraryView<'a> {
    pub store: &'a Store,
    pub active_collection: Collection,
    pub active_tag: Option<&'a str>,
    pub theme: ThemeChoice,
    pub reduced_motion: bool,
    pub collapsed: bool,
    pub mutations_enabled: bool,
    pub data_menu_open: &'a mut bool,
    pub request_data_focus: bool,
    pub transfer_status: Option<&'a str>,
}

pub fn show(ui: &mut egui::Ui, view: LibraryView<'_>) -> LibraryActions {
    let LibraryView {
        store,
        active_collection,
        active_tag,
        theme,
        reduced_motion,
        collapsed,
        mutations_enabled,
        data_menu_open,
        request_data_focus,
        transfer_status,
    } = view;
    let mut actions = LibraryActions::default();
    let mut tags: Vec<_> = store
        .notes
        .iter()
        .flat_map(|note| note.tags.iter().cloned())
        .collect();
    tags.sort();
    tags.dedup();

    let footer_height = if *data_menu_open { 248.0 } else { 128.0 };
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
                .add_enabled(
                    mutations_enabled,
                    egui::Button::new(if collapsed { "+" } else { "+  New note" }),
                )
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
                let response = ui.add_enabled(
                    mutations_enabled,
                    egui::Button::selectable(active_collection == collection, text),
                );
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
                                .add_enabled(
                                    mutations_enabled,
                                    egui::Button::selectable(
                                        active_tag == Some(tag.as_str()),
                                        if collapsed { "#" } else { &label },
                                    ),
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
    let data = ui
        .add_enabled(
            mutations_enabled,
            egui::Button::new(if collapsed { "D" } else { "Data" })
                .min_size(egui::vec2(44.0, 44.0)),
        )
        .accessible_name("Data");
    data.widget_info(|| {
        egui::WidgetInfo::labeled(egui::Role::DisclosureTriangle, mutations_enabled, "Data")
    });
    ui.ctx()
        .accesskit_node_builder(data.id, |node| node.set_expanded(*data_menu_open));
    if request_data_focus {
        data.request_focus();
    }
    if data.clicked() {
        *data_menu_open = !*data_menu_open;
    }
    if *data_menu_open {
        for (label, action) in [
            ("Import Markdown…", DataAction::ImportMarkdown),
            ("Export library…", DataAction::ExportLibrary),
            ("Create backup…", DataAction::CreateBackup),
            ("Restore backup…", DataAction::InspectBackup),
        ] {
            if ui
                .add_enabled(
                    mutations_enabled,
                    egui::Button::new(label).min_size(egui::vec2(44.0, 44.0)),
                )
                .clicked()
            {
                actions.transfer = Some(action);
                *data_menu_open = false;
            }
        }
    }
    if let Some(status) = transfer_status {
        ui.small(status);
    }
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
