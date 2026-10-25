mod common;

use common::{app, note};
use eframe::egui::accesskit::Role;
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
    panic!("quick open did not settle");
}

fn open_overlay(harness: &mut Harness<'_, crane_desktop::app::CraneApp>) {
    harness.key_press_modifiers(eframe::egui::Modifiers::CTRL, eframe::egui::Key::O);
    settle(harness);
}

#[test]
fn quick_open_finds_and_opens_second_hit() {
    let (app, _) = app(vec![
        note("a", "Alpha", "first", 3),
        note("b", "Beta", "second", 2),
        note("c", "Gamma", "third", 1),
    ]);
    let mut harness = Harness::builder()
        .with_size([1240.0, 780.0])
        .build_ui_state(|ui, app| app.show(ui), app);
    settle(&mut harness);
    open_overlay(&mut harness);
    harness.get_by_role_and_label(Role::Button, "Close Quick Open");
    harness
        .get_by_role_and_label(Role::TextInput, "Quick Open")
        .set_text_value_accesskit("a");
    settle(&mut harness);
    harness.key_press(eframe::egui::Key::ArrowDown);
    harness.run_steps(2);
    harness.key_press(eframe::egui::Key::Enter);
    settle(&mut harness);
    assert_eq!(harness.state().selected_note_id(), Some("b"));
    assert!(harness
        .query_by_role_and_label(Role::TextInput, "Quick Open")
        .is_none());
}

#[test]
fn quick_open_escape_closes_without_selection_change() {
    let (app, _) = app(vec![note("a", "Alpha", "first", 3)]);
    let mut harness = Harness::builder()
        .with_size([1240.0, 780.0])
        .build_ui_state(|ui, app| app.show(ui), app);
    settle(&mut harness);
    let before = harness.state().selected_note_id().map(str::to_owned);
    open_overlay(&mut harness);
    harness.key_press(eframe::egui::Key::Escape);
    settle(&mut harness);
    assert_eq!(
        harness.state().selected_note_id().map(str::to_owned),
        before
    );
    assert!(harness
        .query_by_role_and_label(Role::TextInput, "Quick Open")
        .is_none());
}

#[test]
fn quick_open_traps_tab_focus_inside_the_dialog() {
    let (app, _) = app(vec![note("a", "Alpha", "first", 3)]);
    let mut harness = Harness::builder()
        .with_size([1240.0, 780.0])
        .build_ui_state(|ui, app| app.show(ui), app);
    settle(&mut harness);
    open_overlay(&mut harness);

    for _ in 0..12 {
        harness.key_press(eframe::egui::Key::Tab);
        harness.run_steps(2);
        let result_focused = harness
            .query_all_by_label("Alpha")
            .any(|node| node.is_focused());
        assert!(
            harness.get_by_label("Close Quick Open").is_focused()
                || harness
                    .get_by_role_and_label(Role::TextInput, "Quick Open")
                    .is_focused()
                || harness.get_by_label("Include archived").is_focused()
                || result_focused,
            "Tab focus escaped Quick Open"
        );
    }
}

#[test]
fn quick_open_blocks_background_navigation() {
    let (app, _) = app(vec![note("a", "Alpha", "first", 3)]);
    let mut harness = Harness::builder()
        .with_size([1240.0, 780.0])
        .build_ui_state(|ui, app| app.show(ui), app);
    settle(&mut harness);
    open_overlay(&mut harness);

    harness
        .get_by_role_and_label(Role::Button, "Pinned")
        .click();
    harness.run_steps(2);
    harness.key_press(eframe::egui::Key::Escape);
    harness.run_steps(2);

    harness.get_by_label("Notes browser");
    assert!(harness.query_by_label("Pinned browser").is_none());
}

#[test]
fn quick_open_blocks_global_application_shortcuts() {
    let (app, committed) = app(vec![note("a", "Alpha", "first", 3)]);
    let mut harness = Harness::builder()
        .with_size([1240.0, 780.0])
        .build_ui_state(|ui, app| app.show(ui), app);
    settle(&mut harness);
    open_overlay(&mut harness);

    for key in [
        eframe::egui::Key::N,
        eframe::egui::Key::S,
        eframe::egui::Key::P,
        eframe::egui::Key::F,
        eframe::egui::Key::O,
    ] {
        harness.key_press_modifiers(eframe::egui::Modifiers::CTRL, key);
        harness.run_steps(2);
        harness.get_by_role_and_label(Role::TextInput, "Quick Open");
    }

    assert_eq!(harness.state().selected_note_id(), Some("a"));
    assert_eq!(committed.lock().unwrap().notes.len(), 1);
    harness.get_by_label("Preview Markdown");
    assert!(harness.query_by_label("Edit Markdown").is_none());
}

#[test]
fn quick_open_visible_control_and_archived_toggle() {
    let (app, _) = app(vec![
        note("a", "Alpha", "first", 3),
        note("g", "Gamma archive", "old", 2),
    ]);
    let mut harness = Harness::builder()
        .with_size([1240.0, 780.0])
        .build_ui_state(|ui, app| app.show(ui), app);
    settle(&mut harness);
    harness.get_by_label("Gamma archive").click();
    settle(&mut harness);
    harness.get_by_label("Note actions").click();
    harness.run();
    harness.get_by_label("Archive note").click();
    settle(&mut harness);
    harness.get_by_label("Quick Open").click();
    settle(&mut harness);
    harness
        .get_by_role_and_label(Role::TextInput, "Quick Open")
        .set_text_value_accesskit("gamma");
    settle(&mut harness);
    assert!(harness.query_by_label("Gamma archive").is_none());
    harness.get_by_label("Include archived").click();
    settle(&mut harness);
    harness
        .get_by_role_and_label(Role::TextInput, "Quick Open")
        .set_text_value_accesskit("gamma");
    settle(&mut harness);
    harness.get_by_label("Gamma archive");
}

#[test]
fn quick_open_defers_through_failed_save() {
    let (app, _) = app(vec![
        note("a", "Alpha", "first", 3),
        note("b", "Beta", "second", 2),
    ]);
    let mut harness = Harness::builder()
        .with_size([1240.0, 780.0])
        .build_ui_state(|ui, app| app.show(ui), app);
    settle(&mut harness);
    open_overlay(&mut harness);
    harness
        .get_by_role_and_label(Role::TextInput, "Quick Open")
        .set_text_value_accesskit("Beta");
    settle(&mut harness);
    harness
        .get_by_role_and_label(Role::MultilineTextInput, "Note body")
        .set_text_value_accesskit("dirty unsaved body");
    harness.run_steps(2);
    harness
        .state_mut()
        .simulate_save_failure_for_test("disk full");
    harness.key_press(eframe::egui::Key::Enter);
    for _ in 0..6 {
        harness.run_steps(8);
        std::thread::sleep(Duration::from_millis(5));
    }
    assert_eq!(harness.state().selected_note_id(), Some("a"));
    assert!(harness
        .query_by_role_and_label(Role::TextInput, "Quick Open")
        .is_none());
    harness.get_by_label("Retry save");
}
