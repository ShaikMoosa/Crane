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
const INTERFACE_FAMILY: &str = "Crane Interface";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ThemeTokens {
    pub shell_background: Color32,
    pub rail_background: Color32,
    pub list_background: Color32,
    pub editor_background: Color32,
    pub surface_hover: Color32,
    pub surface_selected: Color32,
    pub surface_pressed: Color32,
    pub text_primary: Color32,
    pub text_secondary: Color32,
    pub text_tertiary: Color32,
    pub text_disabled: Color32,
    pub rule: Color32,
    pub rule_strong: Color32,
    pub focus_ring: Color32,
    pub accent: Color32,
    pub accent_hover: Color32,
    pub accent_soft: Color32,
    pub on_accent: Color32,
    pub success: Color32,
    pub warning: Color32,
    pub danger: Color32,
    pub on_status: Color32,
    pub markdown_marker: Color32,
    pub markdown_heading: Color32,
    pub markdown_emphasis: Color32,
    pub markdown_link: Color32,
    pub markdown_tag: Color32,
    pub markdown_quote: Color32,
    pub markdown_code: Color32,
    pub markdown_code_background: Color32,
}

pub fn tokens(choice: ThemeChoice) -> ThemeTokens {
    match choice {
        ThemeChoice::Light => ThemeTokens {
            shell_background: Color32::from_rgb(244, 245, 243),
            rail_background: Color32::from_rgb(243, 244, 242),
            list_background: Color32::from_rgb(232, 234, 231),
            editor_background: Color32::WHITE,
            surface_hover: Color32::from_rgb(238, 240, 237),
            surface_selected: Color32::WHITE,
            surface_pressed: Color32::from_rgb(224, 227, 223),
            text_primary: Color32::from_rgb(32, 33, 36),
            text_secondary: Color32::from_rgb(85, 89, 94),
            text_tertiary: Color32::from_rgb(102, 107, 112),
            text_disabled: Color32::from_rgb(138, 143, 148),
            rule: Color32::from_rgb(213, 217, 212),
            rule_strong: Color32::from_rgb(184, 189, 183),
            focus_ring: Color32::from_rgb(216, 86, 69),
            accent: Color32::from_rgb(216, 86, 69),
            accent_hover: Color32::from_rgb(201, 77, 58),
            accent_soft: Color32::from_rgb(248, 228, 224),
            on_accent: Color32::WHITE,
            success: Color32::from_rgb(46, 125, 91),
            warning: Color32::from_rgb(147, 107, 0),
            danger: Color32::from_rgb(179, 59, 46),
            on_status: Color32::WHITE,
            markdown_marker: Color32::from_rgb(112, 116, 121),
            markdown_heading: Color32::from_rgb(32, 33, 36),
            markdown_emphasis: Color32::from_rgb(64, 68, 73),
            markdown_link: Color32::from_rgb(179, 59, 46),
            markdown_tag: Color32::from_rgb(179, 59, 46),
            markdown_quote: Color32::from_rgb(85, 89, 94),
            markdown_code: Color32::from_rgb(50, 54, 58),
            markdown_code_background: Color32::from_rgb(241, 243, 240),
        },
        ThemeChoice::Dark => ThemeTokens {
            shell_background: Color32::from_rgb(21, 23, 25),
            rail_background: Color32::from_rgb(35, 38, 40),
            list_background: Color32::from_rgb(42, 45, 48),
            editor_background: Color32::from_rgb(27, 29, 31),
            surface_hover: Color32::from_rgb(52, 56, 59),
            surface_selected: Color32::from_rgb(59, 63, 66),
            surface_pressed: Color32::from_rgb(69, 73, 77),
            text_primary: Color32::from_rgb(242, 244, 246),
            text_secondary: Color32::from_rgb(194, 199, 204),
            text_tertiary: Color32::from_rgb(172, 178, 184),
            text_disabled: Color32::from_rgb(119, 125, 131),
            rule: Color32::from_rgb(59, 63, 67),
            rule_strong: Color32::from_rgb(85, 91, 97),
            focus_ring: Color32::from_rgb(240, 120, 101),
            accent: Color32::from_rgb(240, 120, 101),
            accent_hover: Color32::from_rgb(250, 144, 124),
            accent_soft: Color32::from_rgb(79, 48, 44),
            on_accent: Color32::from_rgb(23, 25, 27),
            success: Color32::from_rgb(101, 196, 148),
            warning: Color32::from_rgb(224, 174, 90),
            danger: Color32::from_rgb(255, 129, 117),
            on_status: Color32::from_rgb(21, 23, 25),
            markdown_marker: Color32::from_rgb(151, 157, 163),
            markdown_heading: Color32::from_rgb(242, 244, 246),
            markdown_emphasis: Color32::from_rgb(217, 221, 225),
            markdown_link: Color32::from_rgb(240, 120, 101),
            markdown_tag: Color32::from_rgb(240, 120, 101),
            markdown_quote: Color32::from_rgb(194, 199, 204),
            markdown_code: Color32::from_rgb(223, 227, 231),
            markdown_code_background: Color32::from_rgb(37, 41, 44),
        },
    }
}

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
    let palette = tokens(choice);
    PaneColors {
        rail: palette.rail_background,
        note_list: palette.list_background,
        editor: palette.editor_background,
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
    ui.visuals_mut().override_text_color = Some(tokens(choice).text_primary);
}

pub fn install_local_fonts(context: &egui::Context) {
    let mut definitions = FontDefinitions::default();
    let proportional = definitions
        .families
        .get(&FontFamily::Proportional)
        .cloned()
        .unwrap_or_default();
    let mut interface_fonts = proportional;
    let paths = crane_platform::local_font_paths();

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
        .insert(FontFamily::Name(INTERFACE_FAMILY.into()), interface_fonts);
    context.set_fonts(definitions);
}

pub fn apply(context: &egui::Context, choice: ThemeChoice) {
    let dark = choice == ThemeChoice::Dark;
    let palette = tokens(choice);
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
    visuals.panel_fill = palette.shell_background;
    visuals.window_fill = palette.editor_background;
    visuals.window_stroke = Stroke::new(1.0, palette.rule_strong);
    visuals.override_text_color = Some(palette.text_primary);
    visuals.weak_text_color = Some(palette.text_secondary);
    visuals.faint_bg_color = palette.surface_hover;
    visuals.extreme_bg_color = palette.editor_background;
    visuals.text_edit_bg_color = Some(Color32::TRANSPARENT);
    visuals.code_bg_color = palette.markdown_code_background;
    visuals.hyperlink_color = palette.markdown_link;
    visuals.warn_fg_color = palette.warning;
    visuals.error_fg_color = palette.danger;
    visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0, palette.text_primary);
    visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0, palette.rule);
    visuals.widgets.inactive.fg_stroke = Stroke::new(1.0, palette.text_secondary);
    visuals.widgets.inactive.bg_stroke = Stroke::new(1.0, palette.rule);
    visuals.widgets.hovered.fg_stroke = Stroke::new(1.0, palette.text_primary);
    visuals.widgets.hovered.bg_fill = palette.surface_hover;
    visuals.widgets.hovered.bg_stroke = Stroke::new(1.0, palette.focus_ring);
    visuals.widgets.active.fg_stroke = Stroke::new(1.0, palette.text_primary);
    visuals.widgets.active.bg_fill = palette.surface_pressed;
    visuals.widgets.active.bg_stroke = Stroke::new(1.0, palette.focus_ring);
    visuals.widgets.open.bg_fill = palette.surface_selected;
    visuals.widgets.open.bg_stroke = Stroke::new(1.0, palette.focus_ring);
    visuals.selection.bg_fill = palette.accent_soft;
    visuals.selection.stroke = Stroke::new(1.0, palette.accent);
    visuals.widgets.inactive.corner_radius = CornerRadius::same(6);
    visuals.widgets.hovered.corner_radius = CornerRadius::same(6);
    visuals.widgets.active.corner_radius = CornerRadius::same(6);
    context.set_visuals_of(egui_theme, visuals);

    let mut style = (*context.style_of(egui_theme)).clone();
    style.text_styles = BTreeMap::from([
        (
            TextStyle::Heading,
            FontId::new(22.0, FontFamily::Name(INTERFACE_FAMILY.into())),
        ),
        (
            TextStyle::Body,
            FontId::new(16.0, FontFamily::Name(INTERFACE_FAMILY.into())),
        ),
        (
            TextStyle::Button,
            FontId::new(13.0, FontFamily::Name(INTERFACE_FAMILY.into())),
        ),
        (
            TextStyle::Small,
            FontId::new(11.0, FontFamily::Name(INTERFACE_FAMILY.into())),
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
