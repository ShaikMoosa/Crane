mod common;

use common::{app, note};
use eframe::egui::{accesskit::Role, Key};
use egui_kittest::{
    kittest::{NodeT, Queryable},
    Harness,
};

#[test]
fn every_primary_interactive_control_has_an_accessible_name() {
    let (app, _) = app(vec![note("alpha", "Alpha", "Body #work/process", 1)]);
    let harness = Harness::builder()
        .with_size([1240.0, 780.0])
        .build_ui_state(|ui, app| app.show(ui), app);

    for label in [
        "New note",
        "Notes",
        "Pinned",
        "Archive",
        "Trash",
        "Alpha",
        "Pin note",
        "Archive note",
        "Move to Trash",
        "Preview Markdown",
        "Save now",
        "Use light theme",
    ] {
        harness.get_by_role_and_label(Role::Button, label);
    }

    harness.get_by_role_and_label(Role::CheckBox, "Reduce motion");
    harness.get_by_role_and_label(Role::Button, "#work/process");
    harness.get_by_role_and_label(Role::TextInput, "Search notes");
    harness.get_by_role_and_label(Role::TextInput, "Note title");
    harness.get_by_role_and_label(Role::MultilineTextInput, "Note body");
}

#[test]
fn data_disclosure_reports_state_and_transfer_targets_are_at_least_44_points() {
    let (app, _) = app(vec![note("alpha", "Alpha", "Body", 1)]);
    let mut harness = Harness::builder()
        .with_size([1240.0, 780.0])
        .build_ui_state(|ui, app| app.show(ui), app);

    let data = harness.get_by_role_and_label(Role::DisclosureTriangle, "Data");
    assert_eq!(data.accesskit_node().data().is_expanded(), Some(false));
    assert!(data.rect().width() >= 44.0 && data.rect().height() >= 44.0);

    data.click();
    harness.run();

    let data = harness.get_by_role_and_label(Role::DisclosureTriangle, "Data");
    assert_eq!(data.accesskit_node().data().is_expanded(), Some(true));
    for label in [
        "Import Markdown…",
        "Export library…",
        "Create backup…",
        "Restore backup…",
    ] {
        let action = harness.get_by_role_and_label(Role::Button, label);
        assert!(
            action.rect().width() >= 44.0 && action.rect().height() >= 44.0,
            "{label} was smaller than a 44-point target: {:?}",
            action.rect()
        );
    }

    let export = harness.get_by_role_and_label(Role::Button, "Export note");
    assert!(export.rect().width() >= 44.0 && export.rect().height() >= 44.0);
}

#[test]
fn preview_links_and_compact_navigation_keep_accessible_roles() {
    let (app, _) = app(vec![note(
        "alpha",
        "Alpha",
        "Read [Example](https://example.com) #work/process",
        1,
    )]);
    let mut harness = Harness::builder()
        .with_size([800.0, 780.0])
        .build_ui_state(|ui, app| app.show(ui), app);
    harness.get_by_label("Preview Markdown").click();
    harness.run();
    // The click is consumed while building this frame; render once more so the
    // accessibility tree reflects the newly selected preview mode.
    harness.run();

    harness.get_by_role_and_label(Role::Link, "Example");
    harness.get_by_role_and_label(Role::Button, "#work/process");
    harness.get_by_role_and_label(Role::Button, "Use light theme");
    harness.get_by_role_and_label(Role::CheckBox, "Reduce motion");
}

#[test]
fn preview_preserves_nested_link_roles_and_reports_open_failures() {
    let document = crane_domain::parse_preview(
        "# [Heading link](https://example.com/heading)\n\n> [Quote link](https://example.com/quote)\n\n- **[List link](https://example.com/list)**",
    );
    let mut harness = Harness::builder()
        .with_size([800.0, 600.0])
        .allow_missing_glyphs()
        .build_ui_state(
            |ui, state| {
                let mut opener = |_url: &str| {
                    Err(crane_platform::PlatformError::new(
                        "synthetic link-open failure",
                    ))
                };
                if let crane_desktop::views::preview::PreviewAction::HttpError(error) =
                    crane_desktop::views::preview::show_with_opener(ui, &state.0, &mut opener, None)
                {
                    state.1 = Some(error);
                }
            },
            (document, None::<String>),
        );

    for label in ["Heading link", "Quote link", "List link"] {
        harness.get_by_role_and_label(Role::Link, label);
    }
    harness.get_by_label("Heading link").click();
    harness.run();
    assert_eq!(
        harness.state().1.as_deref(),
        Some("synthetic link-open failure")
    );
}

#[test]
fn keyboard_tab_traversal_reaches_primary_controls_in_visual_order() {
    let (app, _) = app(vec![note("alpha", "Alpha", "Body", 1)]);
    let mut harness = Harness::builder()
        .with_size([1240.0, 780.0])
        .build_ui_state(|ui, app| app.show(ui), app);

    let mut new_note_step = None;
    let mut search_step = None;
    let mut save_step = None;
    let mut body_step = None;
    for step in 0..32 {
        harness.key_press(Key::Tab);
        harness.run();
        if harness.get_by_label("New note").is_focused() {
            new_note_step.get_or_insert(step);
        }
        if harness.get_by_label("Search notes").is_focused() {
            search_step.get_or_insert(step);
        }
        if harness.get_by_label("Save now").is_focused() {
            save_step.get_or_insert(step);
        }
        if harness.get_by_label("Note body").is_focused() {
            body_step.get_or_insert(step);
        }
    }
    let new_note_step = new_note_step.expect("Tab order never reached New note");
    let search_step = search_step.expect("Tab order never reached search");
    let save_step = save_step.expect("Tab order never reached Save now");
    let body_step = body_step.expect("Tab order never reached the note body");
    assert!(new_note_step < search_step);
    assert!(search_step < save_step);
    assert!(save_step < body_step);
}
