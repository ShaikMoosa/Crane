mod common;

use common::{app, note};
use crane_desktop::knowledge_navigation::wiki_fragment_at;
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
    panic!("knowledge navigation did not settle");
}

fn set_body_end(harness: &mut Harness<'_, crane_desktop::app::CraneApp>, text: &str) {
    harness
        .get_by_role_and_label(Role::MultilineTextInput, "Note body")
        .focus();
    harness
        .get_by_role_and_label(Role::MultilineTextInput, "Note body")
        .set_text_value_accesskit(text);
    harness.key_press(eframe::egui::Key::End);
    settle(harness);
}

fn completion_count(harness: &Harness<'_, crane_desktop::app::CraneApp>) -> usize {
    harness
        .query_all_by_label_contains("Complete link: ")
        .count()
}

#[test]
fn wiki_fragment_detection_rules() {
    let fragment = wiki_fragment_at("See [[Fie", 9).expect("fragment");
    assert_eq!(fragment.raw, "Fie");
    assert_eq!(fragment.replace_range, 4..9);

    assert!(wiki_fragment_at("See [[Fie]]", 11).is_none());
    assert!(wiki_fragment_at("See [[A|B", 10).is_none());
    assert!(wiki_fragment_at("See \\[[Fie", 10).is_none());
    assert!(wiki_fragment_at("```\n[[Fie", 9).is_none());
    assert!(wiki_fragment_at("```\ncode\n```\n[[Fie", 18).is_some());
    assert!(wiki_fragment_at("a `[[Fie", 8).is_none());
    assert!(wiki_fragment_at("plain", 5).is_none());
    assert!(wiki_fragment_at("héllo [[wör", 13).is_some());
}

#[test]
fn completion_inserts_title_and_undo_restores() {
    let (app, _) = app(vec![
        note("a", "Alpha", "", 3),
        note("f", "Field Notes", "about fields", 2),
    ]);
    let mut harness = Harness::builder()
        .with_size([1240.0, 780.0])
        .build_ui_state(|ui, app| app.show(ui), app);
    settle(&mut harness);
    set_body_end(&mut harness, "[[Fie");

    harness.get_by_label("Complete link: Field Notes");
    assert_eq!(completion_count(&harness), 1);
    harness.key_press(eframe::egui::Key::Enter);
    settle(&mut harness);
    assert_eq!(
        harness.state().selected_draft().unwrap().body,
        "[[Field Notes]]"
    );
    assert_eq!(completion_count(&harness), 0);

    harness.key_press_modifiers(
        eframe::egui::Modifiers::CTRL | eframe::egui::Modifiers::COMMAND,
        eframe::egui::Key::Z,
    );
    harness.run_steps(4);
    assert_eq!(harness.state().selected_draft().unwrap().body, "[[Fie");
}

#[test]
fn completion_heading_fragment_and_pipe_suppression() {
    let (app, _) = app(vec![
        note("a", "Alpha", "", 3),
        note("f", "Field Notes", "## Onboarding\n## Overview", 2),
    ]);
    let mut harness = Harness::builder()
        .with_size([1240.0, 780.0])
        .build_ui_state(|ui, app| app.show(ui), app);
    settle(&mut harness);
    set_body_end(&mut harness, "[[Field Notes#On");

    harness.get_by_label("Complete link: Onboarding");
    assert_eq!(completion_count(&harness), 1);
    harness.key_press(eframe::egui::Key::Enter);
    settle(&mut harness);
    assert_eq!(
        harness.state().selected_draft().unwrap().body,
        "[[Field Notes#Onboarding]]"
    );

    set_body_end(&mut harness, "[[Field|A");
    assert_eq!(completion_count(&harness), 0);
}

#[test]
fn completion_escape_and_code_fence_suppress() {
    let (app, _) = app(vec![
        note("a", "Alpha", "", 3),
        note("f", "Field Notes", "about fields", 2),
    ]);
    let mut harness = Harness::builder()
        .with_size([1240.0, 780.0])
        .build_ui_state(|ui, app| app.show(ui), app);
    settle(&mut harness);
    set_body_end(&mut harness, "[[Fie");
    assert_eq!(completion_count(&harness), 1);
    harness.key_press(eframe::egui::Key::Escape);
    harness.run_steps(4);
    assert_eq!(harness.state().selected_draft().unwrap().body, "[[Fie");
    assert_eq!(completion_count(&harness), 0);

    set_body_end(&mut harness, "```\n[[Fie");
    assert_eq!(completion_count(&harness), 0);
}

#[test]
fn preview_wiki_link_opens_resolved_note() {
    let (app, _) = app(vec![
        note("a", "Alpha", "See [[Beta]]!", 3),
        note("b", "Beta", "second", 2),
    ]);
    let mut harness = Harness::builder()
        .with_size([1240.0, 780.0])
        .build_ui_state(|ui, app| app.show(ui), app);
    settle(&mut harness);
    harness.get_by_label("Preview Markdown").click();
    settle(&mut harness);
    harness.get_by_label("Open linked note: Beta").click();
    settle(&mut harness);
    assert_eq!(harness.state().selected_note_id(), Some("b"));
}

#[test]
fn heading_navigation_keeps_the_target_markdown_byte_exact() {
    let target = "Intro\r\n\r\n## नमस्ते 🕊️\r\nExact body";
    let (app, _) = app(vec![
        note("a", "Alpha", "See [[Beta#नमस्ते 🕊️]]!", 3),
        note("b", "Beta", target, 2),
    ]);
    let mut harness = Harness::builder()
        .with_size([1240.0, 780.0])
        .allow_missing_glyphs()
        .build_ui_state(|ui, app| app.show(ui), app);
    settle(&mut harness);
    harness.get_by_label("Preview Markdown").click();
    settle(&mut harness);
    harness.get_by_label("Open linked note: Beta").click();
    settle(&mut harness);

    assert_eq!(harness.state().selected_note_id(), Some("b"));
    assert_eq!(harness.state().selected_draft().unwrap().body, target);
    harness.get_by_label("Edit Markdown").click();
    harness.run_steps(2);
    harness.get_by_role_and_label(Role::MultilineTextInput, "Note body");
}

#[test]
fn ambiguous_link_opens_chooser_and_navigates_to_choice() {
    let (app, _) = app(vec![
        note("a", "Alpha", "See [[Dupe]]!", 4),
        note("d1", "Dupe", "first body", 3),
        note("d2", "Dupe", "second body", 2),
    ]);
    let mut harness = Harness::builder()
        .with_size([1240.0, 780.0])
        .build_ui_state(|ui, app| app.show(ui), app);
    settle(&mut harness);
    harness.get_by_label("Preview Markdown").click();
    settle(&mut harness);
    harness.get_by_label("Open linked note: Dupe").click();
    settle(&mut harness);
    let choices: Vec<_> = harness
        .query_all_by_label("Choose linked note: Dupe")
        .collect();
    assert_eq!(choices.len(), 2);
    assert_eq!(harness.query_all_by_label_contains("first body").count(), 2);
    choices.into_iter().nth(1).unwrap().click();
    settle(&mut harness);
    assert_eq!(harness.state().selected_note_id(), Some("d2"));
}

#[test]
fn ambiguous_link_chooser_traps_tab_focus_inside_the_dialog() {
    let (app, _) = app(vec![
        note("a", "Alpha", "See [[Dupe]]!", 4),
        note("d1", "Dupe", "first body", 3),
        note("d2", "Dupe", "second body", 2),
    ]);
    let mut harness = Harness::builder()
        .with_size([1240.0, 780.0])
        .build_ui_state(|ui, app| app.show(ui), app);
    settle(&mut harness);
    harness.get_by_label("Preview Markdown").click();
    settle(&mut harness);
    harness.get_by_label("Open linked note: Dupe").click();
    settle(&mut harness);

    for _ in 0..12 {
        harness.key_press(eframe::egui::Key::Tab);
        harness.run_steps(2);
        let choice_focused = harness
            .query_all_by_label("Choose linked note: Dupe")
            .any(|choice| choice.is_focused());
        assert!(
            harness.get_by_label("Close link chooser").is_focused()
                || choice_focused
                || harness.get_by_label("Cancel").is_focused(),
            "Tab focus escaped the link chooser"
        );
    }
}

#[test]
fn ambiguous_link_chooser_blocks_background_editor_actions() {
    let (app, _) = app(vec![
        note("a", "Alpha", "See [[Dupe]]!", 4),
        note("d1", "Dupe", "first body", 3),
        note("d2", "Dupe", "second body", 2),
    ]);
    let mut harness = Harness::builder()
        .with_size([1240.0, 780.0])
        .build_ui_state(|ui, app| app.show(ui), app);
    settle(&mut harness);
    harness.get_by_label("Preview Markdown").click();
    settle(&mut harness);
    harness.get_by_label("Open linked note: Dupe").click();
    settle(&mut harness);

    harness.get_by_label("Edit Markdown").click();
    harness.run_steps(2);

    harness.get_by_label("Edit Markdown");
    assert!(harness.query_by_label("Note body").is_none());
}

#[test]
fn unresolved_link_offers_create_through_save_gate() {
    let (app, committed) = app(vec![note("a", "Alpha", "See [[Brand New]]!", 3)]);
    let mut harness = Harness::builder()
        .with_size([1240.0, 780.0])
        .build_ui_state(|ui, app| app.show(ui), app);
    settle(&mut harness);
    harness.get_by_label("Preview Markdown").click();
    settle(&mut harness);
    harness.get_by_label("Edit Markdown").click();
    settle(&mut harness);
    harness
        .get_by_role_and_label(Role::MultilineTextInput, "Note body")
        .focus();
    harness
        .get_by_role_and_label(Role::MultilineTextInput, "Note body")
        .set_text_value_accesskit("See [[Brand New]]! plus unsaved words");
    harness.run_steps(2);
    harness.get_by_label("Preview Markdown").click();
    settle(&mut harness);
    harness.get_by_label("Open linked note: Brand New").click();
    settle(&mut harness);
    harness.get_by_label("Create note \"Brand New\"").click();
    settle(&mut harness);

    let created_id = harness
        .state()
        .selected_note_id()
        .expect("created note selected")
        .to_owned();
    assert_ne!(created_id, "a");
    assert_eq!(harness.state().selected_draft().unwrap().title, "Brand New");
    let store = committed.lock().unwrap();
    assert_eq!(
        store.notes.iter().find(|note| note.id == "a").unwrap().body,
        "See [[Brand New]]! plus unsaved words"
    );
}

#[test]
fn missing_heading_opens_top_with_status() {
    let (app, _) = app(vec![
        note("a", "Alpha", "See [[Beta#Nope]]!", 3),
        note("b", "Beta", "## Real\nbody", 2),
    ]);
    let mut harness = Harness::builder()
        .with_size([1240.0, 780.0])
        .build_ui_state(|ui, app| app.show(ui), app);
    settle(&mut harness);
    harness.get_by_label("Preview Markdown").click();
    settle(&mut harness);
    harness.get_by_label("Open linked note: Beta").click();
    settle(&mut harness);
    assert_eq!(harness.state().selected_note_id(), Some("b"));
    harness.get_by_label("Heading not found");
}
