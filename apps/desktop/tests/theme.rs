use crane_desktop::theme;
use crane_storage::ThemeChoice;
use eframe::egui::{self, Color32, FontFamily, TextStyle};

fn linear_channel(channel: u8) -> f32 {
    let value = f32::from(channel) / 255.0;
    if value <= 0.04045 {
        value / 12.92
    } else {
        ((value + 0.055) / 1.055).powf(2.4)
    }
}

fn luminance(color: Color32) -> f32 {
    0.2126 * linear_channel(color.r())
        + 0.7152 * linear_channel(color.g())
        + 0.0722 * linear_channel(color.b())
}

fn contrast(a: Color32, b: Color32) -> f32 {
    let (lighter, darker) = if luminance(a) >= luminance(b) {
        (luminance(a), luminance(b))
    } else {
        (luminance(b), luminance(a))
    };
    (lighter + 0.05) / (darker + 0.05)
}

fn near(actual: Color32, reference: Color32, tolerance: u8) -> bool {
    actual.r().abs_diff(reference.r()) <= tolerance
        && actual.g().abs_diff(reference.g()) <= tolerance
        && actual.b().abs_diff(reference.b()) <= tolerance
}

#[test]
fn nordic_palettes_expose_complete_distinct_semantic_roles() {
    for choice in [ThemeChoice::Light, ThemeChoice::Dark] {
        let palette = theme::tokens(choice);
        let required = [
            palette.shell_background,
            palette.rail_background,
            palette.list_background,
            palette.editor_background,
            palette.surface_hover,
            palette.surface_selected,
            palette.surface_pressed,
            palette.text_primary,
            palette.text_secondary,
            palette.text_tertiary,
            palette.text_disabled,
            palette.rule,
            palette.rule_strong,
            palette.focus_ring,
            palette.accent,
            palette.accent_hover,
            palette.accent_soft,
            palette.on_accent,
            palette.success,
            palette.warning,
            palette.danger,
            palette.on_status,
            palette.markdown_marker,
            palette.markdown_heading,
            palette.markdown_emphasis,
            palette.markdown_link,
            palette.markdown_tag,
            palette.markdown_quote,
            palette.markdown_code,
            palette.markdown_code_background,
        ];

        assert!(required.iter().all(|color| *color != Color32::TRANSPARENT));
        assert_ne!(palette.rail_background, palette.list_background);
        assert_ne!(palette.list_background, palette.editor_background);
        assert_ne!(palette.accent, palette.danger);
    }
}

#[test]
fn nordic_text_and_focus_roles_meet_contrast_contract() {
    for choice in [ThemeChoice::Light, ThemeChoice::Dark] {
        let palette = theme::tokens(choice);
        for (foreground, background, role) in [
            (
                palette.text_primary,
                palette.editor_background,
                "primary/editor",
            ),
            (
                palette.text_primary,
                palette.list_background,
                "primary/list",
            ),
            (
                palette.text_secondary,
                palette.editor_background,
                "secondary/editor",
            ),
            (
                palette.text_secondary,
                palette.rail_background,
                "secondary/rail",
            ),
            (
                palette.text_tertiary,
                palette.editor_background,
                "tertiary/editor",
            ),
        ] {
            assert!(
                contrast(foreground, background) >= 4.5,
                "{choice:?} {role} contrast was {}",
                contrast(foreground, background)
            );
        }
        for (foreground, background, role) in [
            (
                palette.focus_ring,
                palette.editor_background,
                "focus/editor",
            ),
            (palette.focus_ring, palette.list_background, "focus/list"),
            (palette.accent, palette.editor_background, "accent/editor"),
        ] {
            assert!(
                contrast(foreground, background) >= 3.0,
                "{choice:?} {role} contrast was {}",
                contrast(foreground, background)
            );
        }
    }
}

#[test]
fn light_palette_replaces_retired_warm_materials() {
    let palette = theme::tokens(ThemeChoice::Light);
    assert!(near(
        palette.shell_background,
        Color32::from_rgb(244, 245, 243),
        8
    ));
    assert!(near(
        palette.list_background,
        Color32::from_rgb(232, 234, 231),
        8
    ));
    assert!(near(palette.editor_background, Color32::WHITE, 4));
    assert!(near(palette.text_primary, Color32::from_rgb(32, 33, 36), 8));
    assert!(near(palette.accent, Color32::from_rgb(216, 86, 69), 12));
    assert_ne!(palette.rail_background, Color32::from_rgb(36, 37, 35));
    assert_ne!(palette.list_background, Color32::from_rgb(241, 238, 231));
    assert_ne!(palette.editor_background, Color32::from_rgb(251, 250, 247));
}

#[test]
fn authored_and_interface_text_share_the_local_humanist_sans() {
    let context = egui::Context::default();
    theme::apply(&context, ThemeChoice::Light);
    let style = context.style_of(egui::Theme::Light);
    let interface = FontFamily::Name("Crane Interface".into());

    assert_eq!(style.text_styles[&TextStyle::Heading].family, interface);
    assert_eq!(style.text_styles[&TextStyle::Body].family, interface);
    assert_eq!(style.text_styles[&TextStyle::Button].family, interface);
    assert_eq!(
        style.text_styles[&TextStyle::Monospace].family,
        FontFamily::Monospace
    );
}
