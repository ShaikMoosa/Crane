mod common;

use common::{app, note};
use crane_desktop::view_state::{layout_mode, LayoutMode};
use egui_kittest::{kittest::Queryable, Harness};
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
fn native_navigation_searches_filters_and_changes_note_state() {
    let (app, committed) = app(vec![
        note("alpha", "Alpha plan", "Body #work/process", 30),
        note("beta", "Beta reading", "Body #reading/list", 20),
    ]);
    let mut harness = Harness::builder()
        .with_size([1240.0, 780.0])
        .build_ui_state(|ui, app| app.show(ui), app);

    harness
        .get_by_label("Search notes")
        .set_text_value_accesskit("Beta");
    settle(&mut harness);
    harness.get_by_label("Beta reading");
    assert!(harness.query_by_label("Alpha plan").is_none());

    harness
        .get_by_label("Search notes")
        .set_text_value_accesskit("");
    settle(&mut harness);
    harness.get_by_label("#work/process").click();
    settle(&mut harness);
    harness.get_by_label("Alpha plan");
    assert!(harness.query_by_label("Beta reading").is_none());

    harness.get_by_label("Alpha plan").click();
    harness.run();
    harness.get_by_label("Pin note").click();
    settle(&mut harness);
    harness.get_by_label("Pinned").click();
    settle(&mut harness);
    harness.get_by_label("Alpha plan");
    harness.get_by_label("Alpha plan").click();
    harness.run();

    harness.get_by_label("Archive note").click();
    settle(&mut harness);
    harness.get_by_label("Archive").click();
    settle(&mut harness);
    harness.get_by_label("Alpha plan");
    harness.get_by_label("Alpha plan").click();
    harness.run();

    harness.get_by_label("Move to Trash").click();
    settle(&mut harness);
    harness.get_by_label("Trash").click();
    settle(&mut harness);
    harness.get_by_label("Alpha plan");
    harness.get_by_label("Alpha plan").click();
    harness.run();
    harness.get_by_label("Restore note").click();
    settle(&mut harness);
    let store = committed.lock().unwrap();
    let alpha = store.notes.iter().find(|note| note.id == "alpha").unwrap();
    assert!(alpha.pinned);
    assert!(alpha.archived);
    assert!(!alpha.trashed);
}

#[test]
fn create_note_focuses_title_and_layout_breakpoints_are_exact() {
    let (app, _) = app(vec![note("alpha", "Alpha", "Body", 1)]);
    let mut harness = Harness::builder()
        .with_size([1240.0, 780.0])
        .build_ui_state(|ui, app| app.show(ui), app);

    harness.get_by_label("New note").click();
    harness.run();
    assert!(harness.get_by_label("Note title").is_focused());

    assert_eq!(layout_mode(901.0), LayoutMode::Wide);
    assert_eq!(layout_mode(900.0), LayoutMode::CollapsedRail);
    assert_eq!(layout_mode(681.0), LayoutMode::CollapsedRail);
    assert_eq!(layout_mode(680.0), LayoutMode::SinglePane);
}

#[test]
fn query_cache_only_recomputes_for_relevant_changes() {
    let (app, _) = app(vec![note("alpha", "Alpha", "#work/process", 1)]);
    let mut harness = Harness::builder()
        .with_size([1240.0, 780.0])
        .build_ui_state(|ui, app| app.show(ui), app);
    settle(&mut harness);
    let initial = harness.state().cache_stats().query_recomputes;
    assert!(initial >= 1);
    harness.run();
    assert_eq!(harness.state().cache_stats().query_recomputes, initial);

    harness
        .get_by_label("Search notes")
        .set_text_value_accesskit("Alpha");
    settle(&mut harness);
    harness.get_by_label("Alpha");
    assert_eq!(harness.state().cache_stats().query_recomputes, initial + 1);
    harness.run();
    assert_eq!(harness.state().cache_stats().query_recomputes, initial + 1);
}

#[test]
fn sub_680_layout_has_explicit_editor_list_and_library_navigation() {
    let (app, _) = app(vec![note("alpha", "Alpha", "Body", 1)]);
    let mut harness = Harness::builder()
        .with_size([640.0, 780.0])
        .build_ui_state(|ui, app| app.show(ui), app);

    harness.get_by_label("Back to notes").click();
    harness.run();
    harness.get_by_label("Alpha");
    harness.get_by_label("Alpha").click();
    harness.run();
    harness.get_by_label("Back to notes");
    harness.get_by_label("Back to notes").click();
    harness.run();
    harness.get_by_label("Open library").click();
    harness.run();
    harness.get_by_role_and_label(eframe::egui::accesskit::Role::Button, "Notes");
}

#[test]
fn collapsed_rail_keeps_tags_and_preferences_reachable() {
    let (app, _) = app(vec![note("alpha", "Alpha", "#work/process", 1)]);
    let harness = Harness::builder()
        .with_size([800.0, 780.0])
        .build_ui_state(|ui, app| app.show(ui), app);

    harness.get_by_role_and_label(eframe::egui::accesskit::Role::Button, "#work/process");
    harness.get_by_role_and_label(eframe::egui::accesskit::Role::Button, "Use light theme");
    harness.get_by_role_and_label(eframe::egui::accesskit::Role::CheckBox, "Reduce motion");
}

#[test]
fn crowded_collapsed_rail_keeps_preferences_visible_and_usable() {
    let tags = (0..40)
        .map(|index| format!("#topic/{index}"))
        .collect::<Vec<_>>()
        .join(" ");
    let (app, _) = app(vec![note("alpha", "Alpha", &tags, 1)]);
    let mut harness = Harness::builder()
        .with_size([800.0, 560.0])
        .build_ui_state(|ui, app| app.show(ui), app);

    harness.get_by_label("Use light theme").click();
    harness.run();
    harness.get_by_role_and_label(eframe::egui::accesskit::Role::Button, "Use dark theme");
    harness.get_by_role_and_label(eframe::egui::accesskit::Role::CheckBox, "Reduce motion");
}

#[test]
fn startup_selects_a_visible_active_note_not_an_archived_first_record() {
    let mut archived = note("archived", "Archived", "Old", 20);
    archived.archived = true;
    let (app, _) = app(vec![archived, note("active", "Active", "Current", 10)]);
    assert_eq!(app.selected_note_id(), Some("active"));
}

#[test]
fn keyboard_search_and_escape_navigation_are_available() {
    let (wide_app, _) = app(vec![note("alpha", "Alpha", "Body", 1)]);
    let mut wide = Harness::builder()
        .with_size([1240.0, 780.0])
        .build_ui_state(|ui, app| app.show(ui), wide_app);
    wide.key_press_modifiers(eframe::egui::Modifiers::CTRL, eframe::egui::Key::F);
    wide.run();
    assert!(wide.get_by_label("Search notes").is_focused());

    let (narrow_app, _) = app(vec![note("alpha", "Alpha", "Body", 1)]);
    let mut narrow = Harness::builder()
        .with_size([640.0, 780.0])
        .build_ui_state(|ui, app| app.show(ui), narrow_app);
    narrow.key_press(eframe::egui::Key::Escape);
    narrow.run();
    narrow.get_by_label("Open library");
}

#[test]
fn sidebar_shows_derived_collections_and_tag_hierarchy() {
    let (app, _) = app(vec![
        note("a", "Alpha", "Work #project/crane/ui", 30),
        note("b", "Beta", "- [ ] open task", 20),
        note("c", "Gamma", "plain text", 10),
    ]);
    let mut harness = Harness::builder()
        .with_size([1240.0, 780.0])
        .build_ui_state(|ui, app| app.show(ui), app);
    settle(&mut harness);
    harness.get_by_label("Today");
    harness.get_by_label("Todos");
    harness.get_by_label("Untagged");
    harness.get_by_label("#project");
    harness.get_by_label("#project/crane");
    harness.get_by_label("Todos").click();
    settle(&mut harness);
    harness.get_by_label("Beta");
    harness.get_by_label("Notes").click();
    settle(&mut harness);
    harness.get_by_label("#project").click();
    settle(&mut harness);
    harness.get_by_label("Alpha");
}

#[test]
fn light_and_dark_palettes_keep_three_distinct_materials() {
    for choice in [
        crane_storage::ThemeChoice::Light,
        crane_storage::ThemeChoice::Dark,
    ] {
        let colors = crane_desktop::theme::pane_colors(choice);
        assert_ne!(colors.rail, colors.note_list);
        assert_ne!(colors.note_list, colors.editor);
    }
}
