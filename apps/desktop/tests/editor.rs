mod common;

use common::{app, note};
use eframe::egui::accesskit::Role;
use eframe::egui::{Key, Modifiers};
use egui_kittest::{
    kittest::{NodeT, Queryable},
    Harness,
};
use std::time::{Duration, Instant};

fn settle(harness: &mut Harness<'_, crane_desktop::app::CraneApp>) {
    let deadline = Instant::now() + Duration::from_secs(2);
    while Instant::now() < deadline {
        harness.run_steps(8);
        if !harness.state().has_pending_work() {
            return;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    panic!("native app worker did not settle");
}

#[test]
fn markdown_edit_survives_save_and_note_switch() {
    let (app, committed) = app(vec![
        note("alpha", "Alpha", "Old", 2),
        note("beta", "Beta", "Other", 1),
    ]);
    let mut harness = Harness::builder()
        .with_size([1240.0, 780.0])
        .allow_missing_glyphs()
        .build_ui_state(|ui, app| app.show(ui), app);

    harness.get_by_label("Alpha").click();
    harness.run();
    harness
        .get_by_role_and_label(Role::MultilineTextInput, "Note body")
        .set_text_value_accesskit("# Heading\n\nUnicode: नमस्ते 🕊️ #work/process");
    harness.run();
    harness.state_mut().save_now();
    settle(&mut harness);
    harness.get_by_label("Beta").click();
    settle(&mut harness);

    let store = committed.lock().unwrap();
    let alpha = store.notes.iter().find(|note| note.id == "alpha").unwrap();
    assert!(alpha.body.contains("Unicode: नमस्ते"));
    assert_eq!(alpha.tags, vec!["work/process"]);
    assert_eq!(harness.state().selected_note_id(), Some("beta"));
}

#[test]
fn preview_and_theme_toggle_without_invalidating_unrelated_cache() {
    let (app, _) = app(vec![note("alpha", "Alpha", "# Heading", 1)]);
    let mut harness = Harness::builder()
        .with_size([1240.0, 780.0])
        .allow_missing_glyphs()
        .build_ui_state(|ui, app| app.show(ui), app);

    let before = harness.state().cache_stats().preview_recomputes;
    harness.get_by_label("Preview Markdown").click();
    harness.run();
    harness.get_by_role_and_label(Role::Heading, "Heading");
    let parsed = harness.state().cache_stats().preview_recomputes;
    assert_eq!(parsed, before + 1);
    harness.run();
    assert_eq!(harness.state().cache_stats().preview_recomputes, parsed);

    harness.get_by_label("Use light theme").click();
    harness.run();
    assert_eq!(harness.state().cache_stats().preview_recomputes, parsed);
}

#[test]
fn failed_or_missing_save_keeps_draft_and_selection() {
    let (mut app, _) = app(vec![note("alpha", "Alpha", "Old", 1)]);
    app.edit_selected_for_test("Alpha", "Keep this draft");
    app.simulate_save_failure_for_test("disk full");

    assert_eq!(app.selected_note_id(), Some("alpha"));
    assert_eq!(app.selected_draft().unwrap().body, "Keep this draft");
    assert_eq!(app.status(), Some("disk full"));
}

#[test]
fn creating_a_note_flushes_the_dirty_draft_first() {
    let (app, committed) = app(vec![note("alpha", "Alpha", "Old", 1)]);
    let mut harness = Harness::builder()
        .with_size([1240.0, 780.0])
        .build_ui_state(|ui, app| app.show(ui), app);
    harness
        .get_by_role_and_label(Role::MultilineTextInput, "Note body")
        .set_text_value_accesskit("Must survive create");
    harness.run();
    harness.get_by_label("New note").click();
    settle(&mut harness);

    let store = committed.lock().unwrap();
    assert_eq!(
        store
            .notes
            .iter()
            .find(|note| note.id == "alpha")
            .unwrap()
            .body,
        "Must survive create"
    );
    assert_eq!(store.notes.len(), 2);
}

#[test]
fn undo_state_is_isolated_between_notes() {
    let (app, _) = app(vec![
        note("alpha", "Alpha", "Alpha body", 2),
        note("beta", "Beta", "Beta body", 1),
    ]);
    let mut harness = Harness::builder()
        .with_size([1240.0, 780.0])
        .build_ui_state(|ui, app| app.show(ui), app);
    let body = harness.get_by_role_and_label(Role::MultilineTextInput, "Note body");
    body.focus();
    body.set_text_value_accesskit("Changed alpha");
    harness.run();
    harness.state_mut().save_now();
    settle(&mut harness);
    harness.get_by_label("Beta").click();
    settle(&mut harness);

    harness
        .get_by_role_and_label(Role::MultilineTextInput, "Note body")
        .focus();
    harness.run();
    harness.key_press_modifiers(Modifiers::CTRL, Key::Z);
    harness.run();
    assert_eq!(harness.state().selected_draft().unwrap().body, "Beta body");
}

#[test]
fn editor_widget_ids_are_scoped_to_note_and_field() {
    use crane_desktop::views::editor::editor_widget_id;

    assert_ne!(
        editor_widget_id("alpha", "body"),
        editor_widget_id("beta", "body")
    );
    assert_ne!(
        editor_widget_id("alpha", "title"),
        editor_widget_id("alpha", "body")
    );
}

#[test]
fn editor_toolbar_keeps_secondary_actions_in_an_accessible_menu() {
    let (app, _) = app(vec![note("alpha", "Alpha", "Body", 1)]);
    let mut harness = Harness::builder()
        .with_size([1240.0, 780.0])
        .build_ui_state(|ui, app| app.show(ui), app);

    for label in ["Toggle focus mode", "Preview Markdown", "Note information"] {
        harness.get_by_role_and_label(Role::Button, label);
    }
    harness.get_by_label("Saved");
    for label in [
        "Pin note",
        "Archive note",
        "Move to Trash",
        "Export note",
        "Save now",
    ] {
        assert!(
            harness.query_by_label(label).is_none(),
            "{label} leaked into the toolbar"
        );
    }

    let disclosure = harness.get_by_role_and_label(Role::DisclosureTriangle, "Note actions");
    assert_eq!(
        disclosure.accesskit_node().data().is_expanded(),
        Some(false)
    );
    disclosure.click();
    harness.run();

    let disclosure = harness.get_by_role_and_label(Role::DisclosureTriangle, "Note actions");
    assert_eq!(disclosure.accesskit_node().data().is_expanded(), Some(true));
    for label in [
        "Pin note",
        "Archive note",
        "Move to Trash",
        "Export note",
        "Save now",
    ] {
        let action = harness.get_by_role_and_label(Role::Button, label);
        assert!(
            action.rect().width() >= 44.0 && action.rect().height() >= 44.0,
            "{label} was smaller than a 44-point menu target: {:?}",
            action.rect()
        );
    }
}

#[test]
fn escape_closes_note_actions_and_restores_body_focus() {
    let (app, _) = app(vec![note("alpha", "Alpha", "Body", 1)]);
    let mut harness = Harness::builder()
        .with_size([1240.0, 780.0])
        .build_ui_state(|ui, app| app.show(ui), app);

    harness.get_by_label("Note actions").click();
    harness.run();
    harness.key_press(Key::Escape);
    harness.run_steps(2);

    assert!(harness.query_by_label("Pin note").is_none());
    assert!(harness.get_by_label("Note body").is_focused());
}

#[test]
fn save_feedback_moves_from_saved_to_saving_without_a_primary_save_button() {
    let (app, _) = app(vec![note("alpha", "Alpha", "Body", 1)]);
    let mut harness = Harness::builder()
        .with_size([1240.0, 780.0])
        .build_ui_state(|ui, app| app.show(ui), app);

    harness.get_by_label("Saved");
    harness
        .get_by_role_and_label(Role::MultilineTextInput, "Note body")
        .set_text_value_accesskit("Changed body");
    harness.run_steps(2);

    harness.get_by_label("Saving");
    assert!(harness.query_by_label("Save now").is_none());
}

#[test]
fn save_failure_retry_is_visible_above_the_editor_body() {
    let (mut app, committed) = app(vec![note("alpha", "Alpha", "Old", 1)]);
    app.edit_selected_for_test("Alpha", "Keep this draft");
    app.simulate_save_failure_for_test("disk full");
    let mut harness = Harness::builder()
        .with_size([720.0, 560.0])
        .build_ui_state(|ui, app| app.show(ui), app);

    harness.get_by_label("disk full");
    harness.get_by_label("Save failed");
    harness.get_by_label("New note").click();
    harness.run();
    assert_eq!(harness.state().selected_note_id(), Some("alpha"));
    assert_eq!(
        harness.state().selected_draft().unwrap().body,
        "Keep this draft"
    );
    harness
        .get_by_role_and_label(Role::Button, "Retry save")
        .click();
    settle(&mut harness);
    let store = committed.lock().unwrap();
    assert_eq!(
        store
            .notes
            .iter()
            .find(|note| note.id == "alpha")
            .unwrap()
            .body,
        "Keep this draft"
    );
    assert_eq!(store.notes.len(), 2);
}
