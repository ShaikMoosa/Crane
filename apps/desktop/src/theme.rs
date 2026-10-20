use crane_storage::ThemeChoice;
use eframe::egui::{
    self, Color32, CornerRadius, FontData, FontDefinitions, FontFamily, FontId, Stroke, TextStyle,
};
use std::collections::BTreeMap;
use std::sync::Arc;

pub const RAIL_WIDTH: f32 = 236.0;
pub const COLLAPSED_RAIL_WIDTH: f32 = 72.0;
pub const NOTE_LIST_WIDTH: f32 = 330.0;
pub const COLLAPSED_NOTE_LIST_WIDTH: f32 = 290.0;
const SERIF_FAMILY: &str = "Crane Serif";
const INTERFACE_FAMILY: &str = "Crane Interface";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PaneKind {
    Rail,
    NoteList,
    Editor,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PaneColors {
    pub rail: Color32,
    pub note_list: Color32,
    pub editor: Color32,
}

pub fn pane_colors(choice: ThemeChoice) -> PaneColors {
    match choice {
        ThemeChoice::Light => PaneColors {
            rail: Color32::from_rgb(36, 37, 35),
            note_list: Color32::from_rgb(241, 238, 231),
            editor: Color32::from_rgb(251, 250, 247),
        },
        ThemeChoice::Dark => PaneColors {
            rail: Color32::from_rgb(36, 37, 35),
            note_list: Color32::from_rgb(41, 42, 39),
            editor: Color32::from_rgb(32, 33, 31),
        },
    }
}

pub fn paint_pane(ui: &mut egui::Ui, kind: PaneKind, choice: ThemeChoice) {
    let colors = pane_colors(choice);
    let color = match kind {
        PaneKind::Rail => colors.rail,
        PaneKind::NoteList => colors.note_list,
        PaneKind::Editor => colors.editor,
    };
    ui.painter().rect_filled(ui.max_rect(), 0, color);
    if kind == PaneKind::Rail {
        ui.visuals_mut().override_text_color = Some(Color32::from_rgb(236, 232, 223));
    }
}

pub fn install_local_fonts(context: &egui::Context) {
    let mut definitions = FontDefinitions::default();
    let proportional = definitions
        .families
        .get(&FontFamily::Proportional)
        .cloned()
        .unwrap_or_default();
    let mut serif_fonts = proportional.clone();
    let mut interface_fonts = proportional;
    let paths = crane_platform::local_font_paths();

    if let Some(path) = paths.serif {
        if let Ok(bytes) = crane_platform::load_permitted_local_font(&path) {
            let name = "crane-local-serif".to_owned();
            definitions
                .font_data
                .insert(name.clone(), Arc::new(FontData::from_owned(bytes)));
            serif_fonts.insert(0, name);
        }
    }
    if let Some(path) = paths.interface {
        if let Ok(bytes) = crane_platform::load_permitted_local_font(&path) {
            let name = "crane-local-interface".to_owned();
            definitions
                .font_data
                .insert(name.clone(), Arc::new(FontData::from_owned(bytes)));
            interface_fonts.insert(0, name);
        }
    }
    definitions
        .families
        .insert(FontFamily::Name(SERIF_FAMILY.into()), serif_fonts);
    definitions
        .families
        .insert(FontFamily::Name(INTERFACE_FAMILY.into()), interface_fonts);
    context.set_fonts(definitions);
}

pub fn apply(context: &egui::Context, choice: ThemeChoice) {
    let dark = choice == ThemeChoice::Dark;
    let egui_theme = if dark {
        egui::Theme::Dark
    } else {
        egui::Theme::Light
    };
    context.set_theme(egui_theme);
    let mut visuals = if dark {
        egui::Visuals::dark()
    } else {
        egui::Visuals::light()
    };
    let canvas = if dark {
        Color32::from_rgb(32, 33, 31)
    } else {
        Color32::from_rgb(251, 250, 247)
    };
    let ink = if dark {
        Color32::from_rgb(236, 232, 223)
    } else {
        Color32::from_rgb(40, 39, 33)
    };
    let muted = if dark {
        Color32::from_rgb(169, 165, 156)
    } else {
        Color32::from_rgb(95, 92, 85)
    };
    let rule = if dark {
        Color32::from_rgb(59, 61, 57)
    } else {
        Color32::from_rgb(222, 217, 207)
    };
    let ember = Color32::from_rgb(223, 91, 66);
    visuals.panel_fill = canvas;
    visuals.window_fill = canvas;
    visuals.override_text_color = Some(ink);
    visuals.widgets.inactive.fg_stroke = Stroke::new(1.0, muted);
    visuals.widgets.inactive.bg_stroke = Stroke::new(1.0, rule);
    visuals.widgets.hovered.fg_stroke = Stroke::new(1.0, ink);
    visuals.widgets.hovered.bg_stroke = Stroke::new(1.0, ember);
    visuals.widgets.active.fg_stroke = Stroke::new(1.0, ember);
    visuals.widgets.active.bg_stroke = Stroke::new(1.0, ember);
    visuals.selection.bg_fill = ember.gamma_multiply(0.35);
    visuals.selection.stroke = Stroke::new(1.0, ember);
    visuals.widgets.inactive.corner_radius = CornerRadius::same(6);
    visuals.widgets.hovered.corner_radius = CornerRadius::same(6);
    visuals.widgets.active.corner_radius = CornerRadius::same(6);
    context.set_visuals_of(egui_theme, visuals);

    let mut style = (*context.style_of(egui_theme)).clone();
    style.text_styles = BTreeMap::from([
        (
            TextStyle::Heading,
            FontId::new(22.0, FontFamily::Name(SERIF_FAMILY.into())),
        ),
        (
            TextStyle::Body,
            FontId::new(16.0, FontFamily::Name(SERIF_FAMILY.into())),
        ),
        (
            TextStyle::Button,
            FontId::new(13.0, FontFamily::Name(INTERFACE_FAMILY.into())),
        ),
        (
            TextStyle::Small,
            FontId::new(10.0, FontFamily::Name(INTERFACE_FAMILY.into())),
        ),
        (
            TextStyle::Monospace,
            FontId::new(14.0, FontFamily::Monospace),
        ),
    ]);
    style.spacing.item_spacing = egui::vec2(9.0, 9.0);
    style.spacing.button_padding = egui::vec2(10.0, 7.0);
    context.set_style_of(egui_theme, style);
}
