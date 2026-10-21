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
    pub toggle_theme: bool,
    pub reduced_motion: Option<bool>,
    pub transfer: Option<DataAction>,
}

pub struct LibraryView<'a> {
    pub theme: ThemeChoice,
    pub reduced_motion: bool,
    pub collapsed: bool,
    pub mutations_enabled: bool,
    pub data_menu_open: &'a mut bool,
    pub request_data_focus: bool,
    pub transfer_status: Option<&'a str>,
}

/// Renders the rail footer: Data menu, theme, and reduced-motion controls.
/// Collections and tags live in `crate::sidebar`.
pub fn show(ui: &mut egui::Ui, view: LibraryView<'_>) -> LibraryActions {
    let LibraryView {
        theme,
        reduced_motion,
        collapsed,
        mutations_enabled,
        data_menu_open,
        request_data_focus,
        transfer_status,
    } = view;
    let mut actions = LibraryActions::default();
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
    let mut picked = None;
    egui::Popup::menu(&data)
        .open_bool(data_menu_open)
        .show(|ui| {
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
                    picked = Some(action);
                }
            }
        });
    if let Some(action) = picked {
        actions.transfer = Some(action);
        *data_menu_open = false;
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
