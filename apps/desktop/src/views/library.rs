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
    let text_color = ui.visuals().text_color();
    let data_icon =
        crate::icons::icon_for_text_button(ui, crate::icons::IconId::DatabaseBackup, text_color);
    let data_button = if collapsed {
        egui::Button::image(data_icon)
    } else {
        egui::Button::image_and_text(data_icon, "Data")
    };
    let data = ui
        .add_enabled(
            mutations_enabled,
            data_button.min_size(egui::vec2(44.0, 44.0)),
        )
        .accessible_name("Data")
        .on_hover_text("Data and backups");
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
        .frame(crate::theme::menu_frame(theme))
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
    let settings_icon =
        crate::icons::icon_for_text_button(ui, crate::icons::IconId::Settings, text_color);
    let settings_button = if collapsed {
        egui::Button::image(settings_icon)
    } else {
        egui::Button::image_and_text(settings_icon, "Settings")
    };
    actions.toggle_theme = ui
        .add(settings_button.min_size(egui::vec2(44.0, 44.0)))
        .accessible_name(theme_label)
        .on_hover_text(theme_label)
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
