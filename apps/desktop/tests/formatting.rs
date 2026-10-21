mod common;

use common::{app, note};
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
    panic!("formatting did not settle");
}

fn select_all(harness: &mut Harness<'_, crane_desktop::app::CraneApp>, text: &str) {
    harness
        .get_by_role_and_label(Role::MultilineTextInput, "Note body")
        .focus();
    harness
        .get_by_role_and_label(Role::MultilineTextInput, "Note body")
        .set_text_value_accesskit(text);
    harness.run_steps(2);
    for _ in 0..text.chars().count() {
        harness.key_press_modifiers(eframe::egui::Modifiers::SHIFT, eframe::egui::Key::ArrowLeft);
    }
    harness.run_steps(4);
}

#[test]
fn formatting_module_exists() {
    let outcome = crane_desktop::formatting::apply_format(
        "hello",
        0..5,
        &crane_desktop::formatting::FormatAction::Bold,
    );
    assert_eq!(outcome.body, "**hello**");
}

#[test]
fn formatting_bold_wraps_selection_and_saves() {
    let (app, committed) = app(vec![note("a", "Alpha", "hello", 3)]);
    let mut harness = Harness::builder()
        .with_size([1240.0, 780.0])
        .build_ui_state(|ui, app| app.show(ui), app);
    settle(&mut harness);
    select_all(&mut harness, "hello");
    harness.get_by_label("Bold").click();
    harness.run_steps(4);
    assert_eq!(harness.state().selected_draft().unwrap().body, "**hello**");

    harness.get_by_label("Preview Markdown").click();
    harness.run_steps(2);
    assert!(harness.query_by_label("Bold").is_none());
    harness.get_by_label("Edit Markdown").click();
    settle(&mut harness);

    let store = committed.lock().unwrap();
    assert_eq!(
        store.notes.iter().find(|note| note.id == "a").unwrap().body,
        "**hello**"
    );
}

#[test]
fn formatting_undo_restores_body() {
    let (app, _) = app(vec![note("a", "Alpha", "hello", 3)]);
    let mut harness = Harness::builder()
        .with_size([1240.0, 780.0])
        .build_ui_state(|ui, app| app.show(ui), app);
    settle(&mut harness);
    select_all(&mut harness, "hello");
    harness.get_by_label("Bold").click();
    harness.run_steps(4);
    assert_eq!(harness.state().selected_draft().unwrap().body, "**hello**");

    harness.key_press_modifiers(
        eframe::egui::Modifiers::CTRL | eframe::egui::Modifiers::COMMAND,
        eframe::egui::Key::Z,
    );
    harness.run_steps(4);
    assert_eq!(harness.state().selected_draft().unwrap().body, "hello");
}

#[test]
fn formatting_passes_through_save_gate_on_switch() {
    let (app, committed) = app(vec![
        note("a", "Alpha", "hello", 3),
        note("b", "Beta", "second", 2),
    ]);
    let mut harness = Harness::builder()
        .with_size([1240.0, 780.0])
        .build_ui_state(|ui, app| app.show(ui), app);
    settle(&mut harness);
    select_all(&mut harness, "hello");
    harness.get_by_label("Bold").click();
    harness.run_steps(2);
    harness.get_by_label("Beta").click();
    settle(&mut harness);

    assert_eq!(harness.state().selected_note_id(), Some("b"));
    let store = committed.lock().unwrap();
    assert_eq!(
        store.notes.iter().find(|note| note.id == "a").unwrap().body,
        "**hello**"
    );
}
