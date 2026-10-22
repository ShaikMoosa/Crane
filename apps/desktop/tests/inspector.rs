mod common;

use common::{app, note};
use crane_desktop::view_state::{
    inspector_presentation, InspectorPresentation, EDITOR_SIDE_INSET, INSPECTOR_WIDTH,
};
use eframe::egui::accesskit::Role;
use egui_kittest::{kittest::Queryable, Harness};
use std::time::{Duration, Instant};

fn settle(harness: &mut Harness<'_, crane_desktop::app::CraneApp>) {
    let deadline = Instant::now() + Duration::from_secs(4);
    while Instant::now() < deadline {
        harness.run_steps(8);
        if !harness.state().has_pending_work() {
            return;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    panic!("inspector did not settle");
}

#[test]
fn inspector_module_exists() {
    let _ = crane_desktop::inspector::InspectorTab::Statistics;
}

#[test]
fn inspector_presentation_is_decided_by_remaining_editor_measure() {
    let below_minimum = INSPECTOR_WIDTH + EDITOR_SIDE_INSET * 2.0 + 659.0;
    let at_minimum = INSPECTOR_WIDTH + EDITOR_SIDE_INSET * 2.0 + 660.0;

    assert_eq!(
        inspector_presentation(below_minimum),
        InspectorPresentation::Sheet
    );
    assert_eq!(
        inspector_presentation(at_minimum),
        InspectorPresentation::Docked
    );
}

#[test]
fn ordinary_wide_inspector_is_a_sheet_and_does_not_shrink_the_editor() {
    let (app, _) = app(vec![note("a", "Alpha", "body", 3)]);
    let mut harness = Harness::builder()
        .with_size([1440.0, 900.0])
        .build_ui_state(|ui, app| app.show(ui), app);
    settle(&mut harness);
    let before = harness
        .get_by_role_and_label(Role::MultilineTextInput, "Note body")
        .rect();

    harness.get_by_label("Note information").click();
    harness.run_steps(4);

    harness.get_by_role_and_label(Role::Window, "Inspector");
    let after = harness
        .get_by_role_and_label(Role::MultilineTextInput, "Note body")
        .rect();
    assert!((before.width() - after.width()).abs() < 1.0);
    assert!((before.min.x - after.min.x).abs() < 1.0);
}

#[test]
fn inspector_statistics_show_draft_counts_before_save() {
    let (app, _) = app(vec![note("a", "Alpha", "old", 3)]);
    let mut harness = Harness::builder()
        .with_size([1600.0, 900.0])
        .build_ui_state(|ui, app| app.show(ui), app);
    settle(&mut harness);
    harness
        .get_by_role_and_label(Role::MultilineTextInput, "Note body")
        .focus();
    harness
        .get_by_role_and_label(Role::MultilineTextInput, "Note body")
        .set_text_value_accesskit("Hello world task list\n- [ ] do this\n- [x] done");
    harness.run_steps(2);
    harness.get_by_label("Note information").click();
    harness.run_steps(4);

    assert!(harness.state().selected_draft().unwrap().is_dirty());
    harness.get_by_label("12 words");
    harness.get_by_label("2 tasks");

    harness.get_by_label("Links").click();
    harness.run_steps(2);
    harness.get_by_label("Save to update links");
}

#[test]
fn inspector_outline_jumps_caret_to_heading() {
    let (app, _) = app(vec![note(
        "a",
        "Alpha",
        "# Top\n\ntext\n\n## Deep\nmore",
        3,
    )]);
    let mut harness = Harness::builder()
        .with_size([1600.0, 900.0])
        .build_ui_state(|ui, app| app.show(ui), app);
    settle(&mut harness);
    harness.get_by_label("Note information").click();
    harness.run_steps(4);
    harness.get_by_label("Outline").click();
    harness.run_steps(2);
    harness.get_by_label("Top");
    harness.get_by_label("Deep").click();
    harness.run_steps(4);
    harness.event(eframe::egui::Event::Text("X".to_owned()));
    harness.run_steps(2);
    assert!(harness
        .state()
        .selected_draft()
        .unwrap()
        .body
        .contains("## XDeep"));
}

#[test]
fn inspector_links_show_all_states_and_open_resolved() {
    let (app, _) = app(vec![
        note("a", "Alpha", "[[Beta]] [[Missing]] [[Dupe]]", 4),
        note("b", "Beta", "second", 3),
        note("d1", "Dupe", "first", 2),
        note("d2", "Dupe", "second dupe", 1),
    ]);
    let mut harness = Harness::builder()
        .with_size([1600.0, 900.0])
        .build_ui_state(|ui, app| app.show(ui), app);
    settle(&mut harness);
    harness.get_by_label("Note information").click();
    settle(&mut harness);
    harness.get_by_label("Links").click();
    settle(&mut harness);

    harness.get_by_label("Missing");
    harness.get_by_label("Link target not found");
    assert_eq!(harness.query_all_by_label("Dupe").count(), 3);
    harness.get_by_label("Multiple notes share this title");
    harness.get_by_label("Open linked note: Beta").click();
    settle(&mut harness);
    assert_eq!(harness.state().selected_note_id(), Some("b"));
}

#[test]
fn inspector_backlinks_open_source_through_gate() {
    let (app, committed) = app(vec![
        note("a", "Alpha", "A body", 3),
        note("b", "Beta", "zxq [[Alpha]]", 2),
    ]);
    let mut harness = Harness::builder()
        .with_size([1600.0, 900.0])
        .build_ui_state(|ui, app| app.show(ui), app);
    settle(&mut harness);
    harness.get_by_label("Note information").click();
    settle(&mut harness);
    harness.get_by_label("Backlinks").click();
    settle(&mut harness);

    harness.get_by_label("Open backlink: Beta");
    assert_eq!(harness.query_all_by_label_contains("zxq").count(), 2);

    harness
        .get_by_role_and_label(Role::MultilineTextInput, "Note body")
        .focus();
    harness
        .get_by_role_and_label(Role::MultilineTextInput, "Note body")
        .set_text_value_accesskit("A body plus unsaved");
    harness.run_steps(2);
    harness.get_by_label("Open backlink: Beta").click();
    settle(&mut harness);

    assert_eq!(harness.state().selected_note_id(), Some("b"));
    let store = committed.lock().unwrap();
    assert_eq!(
        store.notes.iter().find(|note| note.id == "a").unwrap().body,
        "A body plus unsaved"
    );
}

#[test]
fn unsaved_body_never_appears_as_backlink() {
    let (app, _) = app(vec![note("a", "Alpha", "hello", 3)]);
    let mut harness = Harness::builder()
        .with_size([1600.0, 900.0])
        .build_ui_state(|ui, app| app.show(ui), app);
    settle(&mut harness);
    harness.get_by_label("Note information").click();
    settle(&mut harness);
    harness.get_by_label("Backlinks").click();
    settle(&mut harness);
    harness.get_by_label("No backlinks yet");

    harness
        .get_by_role_and_label(Role::MultilineTextInput, "Note body")
        .focus();
    harness
        .get_by_role_and_label(Role::MultilineTextInput, "Note body")
        .set_text_value_accesskit("see [[Alpha]]");
    harness.run_steps(4);
    harness.get_by_label("No backlinks yet");
    assert_eq!(
        harness
            .query_all_by_label_contains("Open backlink:")
            .count(),
        0
    );
}

#[test]
fn inspector_tab_survives_note_switch() {
    let (app, _) = app(vec![
        note("a", "Alpha", "see [[Beta]]", 3),
        note("b", "Beta", "plain", 2),
    ]);
    let mut harness = Harness::builder()
        .with_size([1600.0, 900.0])
        .build_ui_state(|ui, app| app.show(ui), app);
    settle(&mut harness);
    harness.get_by_label("Note information").click();
    settle(&mut harness);
    harness.get_by_label("Links").click();
    settle(&mut harness);
    harness.get_by_label("Open linked note: Beta");

    harness.get_by_label("Beta").click();
    settle(&mut harness);
    harness.get_by_label("No outgoing links");
}

#[test]
fn inspector_floats_and_closes_on_navigation_narrow() {
    let (app, _) = app(vec![note("a", "Alpha", "body", 3)]);
    let mut harness = Harness::builder()
        .with_size([500.0, 800.0])
        .build_ui_state(|ui, app| app.show(ui), app);
    settle(&mut harness);
    harness.get_by_label("Note information").click();
    harness.run_steps(4);
    harness.get_by_label("Statistics");

    harness.get_by_label("Back to notes").click();
    settle(&mut harness);
    assert!(harness.query_by_label("Statistics").is_none());
}

#[test]
fn inspector_empty_states() {
    let (app, _) = app(vec![note("a", "Alpha", "", 3)]);
    let mut harness = Harness::builder()
        .with_size([1600.0, 900.0])
        .build_ui_state(|ui, app| app.show(ui), app);
    settle(&mut harness);
    harness.get_by_label("Note information").click();
    settle(&mut harness);
    harness.get_by_label("0 words");

    harness.get_by_label("Outline").click();
    harness.run_steps(2);
    harness.get_by_label("No headings yet");

    harness.get_by_label("Links").click();
    harness.run_steps(2);
    harness.get_by_label("No outgoing links");

    harness.get_by_label("Backlinks").click();
    harness.run_steps(2);
    harness.get_by_label("No backlinks yet");
}
