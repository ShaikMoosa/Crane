mod common;

use common::{app, note};
use crane_desktop::view_state::{
    editor_measure, layout_mode, LayoutMode, COLLAPSED_NOTE_LIST_WIDTH, COLLAPSED_RAIL_WIDTH,
    EDITOR_MEASURE_MAX,
};
use crane_desktop::views::note_list::{metadata_chips, plain_excerpt, relative_time};
use eframe::egui::accesskit::Role;
use egui_kittest::{
    kittest::{NodeT, Queryable},
    Harness,
};
use std::time::{Duration, Instant};

fn brightness(color: eframe::egui::Color32) -> u16 {
    u16::from(color.r()) + u16::from(color.g()) + u16::from(color.b())
}

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
    harness
        .get_by_role_and_label(Role::Button, "#work/process")
        .click();
    settle(&mut harness);
    harness.get_by_label("Alpha plan");
    assert!(harness.query_by_label("Beta reading").is_none());

    harness.get_by_label("Alpha plan").click();
    harness.run();
    harness.get_by_label("Note actions").click();
    harness.run();
    harness.get_by_label("Pin note").click();
    settle(&mut harness);
    harness.get_by_label("Pinned").click();
    settle(&mut harness);
    harness.get_by_label("Alpha plan");
    harness.get_by_label("Alpha plan").click();
    harness.run();

    harness.get_by_label("Note actions").click();
    harness.run();
    harness.get_by_label("Archive note").click();
    settle(&mut harness);
    harness.get_by_label("Archive").click();
    settle(&mut harness);
    harness.get_by_label("Alpha plan");
    harness.get_by_label("Alpha plan").click();
    harness.run();

    harness.get_by_label("Note actions").click();
    harness.run();
    harness.get_by_label("Move to Trash").click();
    settle(&mut harness);
    harness.get_by_label("Trash").click();
    settle(&mut harness);
    harness.get_by_label("Alpha plan");
    harness.get_by_label("Alpha plan").click();
    harness.run();
    harness.get_by_label("Note actions").click();
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

    assert_eq!(layout_mode(1200.0), LayoutMode::Wide);
    assert_eq!(layout_mode(1199.0), LayoutMode::CollapsedRail);
    assert_eq!(layout_mode(720.0), LayoutMode::CollapsedRail);
    assert_eq!(layout_mode(719.0), LayoutMode::SinglePane);
    assert_eq!(editor_measure(900.0), EDITOR_MEASURE_MAX);
    assert_eq!(editor_measure(640.0), 608.0);
}

#[test]
fn nordic_library_and_browser_present_hierarchy_and_bounded_metadata() {
    let mut alpha = note(
        "alpha",
        "Alpha plan",
        "# Project update\nThis **plain** excerpt stays readable. #work #urgent #third",
        9_000,
    );
    alpha.pinned = true;
    let chips = metadata_chips(&alpha);
    let (app, _) = app(vec![alpha, note("beta", "Beta", "Second note", 8_000)]);
    let harness = Harness::builder()
        .with_size([1440.0, 900.0])
        .build_ui_state(|ui, app| app.show(ui), app);

    harness.get_by_role_and_label(Role::Heading, "Crane");
    harness.get_by_role_and_label(Role::Heading, "Notes browser");
    harness.get_by_label("2 notes");
    harness.get_by_role_and_label(Role::TextInput, "Search notes");
    harness.get_by_role_and_label(Role::Button, "Quick Open");
    let create = harness.get_by_role_and_label(Role::Button, "New note");
    assert!(
        (create.rect().width() - create.rect().height()).abs() < 2.0,
        "New note must be the circular square accent control: {:?}",
        create.rect()
    );
    assert_eq!(
        harness
            .get_by_role_and_label(Role::Button, "Notes")
            .accesskit_node()
            .toggled(),
        Some(eframe::egui::accesskit::Toggled::True)
    );
    assert_eq!(
        harness
            .get_by_role_and_label(Role::Button, "Alpha plan")
            .accesskit_node()
            .toggled(),
        Some(eframe::egui::accesskit::Toggled::True)
    );

    assert_eq!(chips.len(), 2);
    assert_eq!(chips[0], "Pinned");
    assert!(
        plain_excerpt("# Hello **world** [site](https://example.com)").contains("Hello world site")
    );
    assert!(plain_excerpt(&"word ".repeat(100)).chars().count() <= 140);
}

#[test]
fn note_browser_rows_do_not_overlap() {
    let (app, _) = app(vec![
        note(
            "a",
            "Alpha",
            "First excerpt with enough words for two lines in the browser.",
            3,
        ),
        note(
            "b",
            "Beta",
            "Second excerpt with enough words for two lines in the browser.",
            2,
        ),
        note(
            "c",
            "Gamma",
            "Third excerpt with enough words for two lines in the browser.",
            1,
        ),
    ]);
    let harness = Harness::builder()
        .with_size([1240.0, 780.0])
        .build_ui_state(|ui, app| app.show(ui), app);

    let alpha = harness.get_by_role_and_label(Role::Button, "Alpha").rect();
    let beta = harness.get_by_role_and_label(Role::Button, "Beta").rect();
    let gamma = harness.get_by_role_and_label(Role::Button, "Gamma").rect();
    assert!(
        beta.top() >= alpha.bottom(),
        "Alpha and Beta note cards overlapped: {alpha:?} / {beta:?}"
    );
    assert!(
        gamma.top() >= beta.bottom(),
        "Beta and Gamma note cards overlapped: {beta:?} / {gamma:?}"
    );
}

#[test]
fn relative_time_table_is_stable_and_future_safe() {
    const MINUTE: u64 = 60_000;
    const HOUR: u64 = 60 * MINUTE;
    const DAY: u64 = 24 * HOUR;
    let now = 1_700_000_000_000;
    assert_eq!(relative_time(now, now), "now");
    assert_eq!(relative_time(now - 5 * MINUTE, now), "5m");
    assert_eq!(relative_time(now - 3 * HOUR, now), "3h");
    assert_eq!(relative_time(now - 2 * DAY, now), "2d");
    assert_eq!(relative_time(0, now), "1970-01-01");
    assert_eq!(relative_time(now + DAY, now), "now");
}

#[test]
fn empty_search_names_filter_and_can_recover() {
    let (app, _) = app(vec![note("alpha", "Alpha", "Body", 1)]);
    let mut harness = Harness::builder()
        .with_size([1440.0, 780.0])
        .build_ui_state(|ui, app| app.show(ui), app);

    harness
        .get_by_label("Search notes")
        .set_text_value_accesskit("zzzzz");
    settle(&mut harness);
    harness.get_by_label("No notes match “zzzzz”");
    harness
        .get_by_role_and_label(Role::Button, "Clear search")
        .click();
    settle(&mut harness);
    harness.get_by_role_and_label(Role::Button, "Alpha");
}

#[test]
fn thousand_note_browser_only_builds_visible_rows_and_scrolls_to_late_note() {
    let notes = (0..1_000)
        .map(|index| {
            note(
                &format!("note-{index:04}"),
                &format!("Note {index:04}"),
                "A compact body excerpt",
                index,
            )
        })
        .collect();
    let (app, _) = app(notes);
    let mut harness = Harness::builder()
        .with_size([1440.0, 780.0])
        .build_ui_state(|ui, app| app.show(ui), app);

    let visible_rows = harness
        .query_all_by_role(Role::Button)
        .filter(|node| {
            node.accesskit_node()
                .label()
                .is_some_and(|label| label.starts_with("Note "))
        })
        .count();
    assert!(
        (1..30).contains(&visible_rows),
        "expected a virtualized window, found {visible_rows} note rows"
    );

    for _ in 0..70 {
        let first_visible = harness
            .query_all_by_role(Role::Button)
            .find(|node| {
                node.accesskit_node()
                    .label()
                    .is_some_and(|label| label.starts_with("Note "))
            })
            .expect("the virtualized list should keep a visible row");
        first_visible.scroll_down();
        harness.run();
    }
    let late = harness
        .query_all_by_role(Role::Button)
        .find(|node| {
            let center_y = node.rect().center().y;
            (180.0..700.0).contains(&center_y)
                && node.accesskit_node().label().is_some_and(|label| {
                    label
                        .strip_prefix("Note ")
                        .is_some_and(|number| number < "0950")
                })
        })
        .expect("scrolling should render a later virtualized row");
    let label = late.accesskit_node().label().unwrap().to_owned();
    late.click();
    settle(&mut harness);
    let expected_id = format!("note-{}", label.trim_start_matches("Note "));
    assert_eq!(
        harness.state().selected_note_id(),
        Some(expected_id.as_str())
    );
}

#[test]
fn exact_720_point_viewport_uses_collapsed_shell() {
    let (app, _) = app(vec![note("alpha", "Alpha", "Body", 1)]);
    let harness = Harness::builder()
        .with_size([720.0, 560.0])
        .build_ui_state(|ui, app| app.show(ui), app);

    harness.get_by_label("New note");
    assert!(harness.query_by_label("Back to notes").is_none());
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
fn sub_720_layout_has_explicit_editor_list_and_library_navigation() {
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
    let reduce_motion =
        harness.get_by_role_and_label(eframe::egui::accesskit::Role::CheckBox, "Reduce motion");
    assert!(
        reduce_motion.rect().max.y <= 560.0,
        "reduced-motion preference was outside the viewport: {:?}",
        reduce_motion.rect()
    );
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
    harness.get_by_role_and_label(Role::Button, "Notes").click();
    settle(&mut harness);
    harness.get_by_label("#project").click();
    settle(&mut harness);
    harness.get_by_label("Alpha");
}

#[test]
fn focus_mode_hides_chrome_and_restores_selection() {
    let (app, _) = app(vec![
        note("a", "Alpha", "first", 3),
        note("b", "Beta", "second", 2),
    ]);
    let mut harness = Harness::builder()
        .with_size([1240.0, 780.0])
        .build_ui_state(|ui, app| app.show(ui), app);
    settle(&mut harness);
    let before = harness.state().selected_note_id().map(str::to_owned);
    harness.get_by_role_and_label(Role::Button, "Notes");

    harness.get_by_label("Toggle focus mode").click();
    harness.run_steps(4);
    assert!(harness
        .query_by_role_and_label(Role::Button, "Notes")
        .is_none());
    harness.get_by_role_and_label(Role::MultilineTextInput, "Note body");

    harness.get_by_label("Toggle focus mode").click();
    harness.run_steps(4);
    harness.get_by_role_and_label(Role::Button, "Notes");
    assert_eq!(
        harness.state().selected_note_id().map(str::to_owned),
        before
    );
}

#[test]
fn focus_mode_restores_inspector_tab_and_body_focus() {
    let (app, _) = app(vec![note("a", "Alpha", "See [[Alpha]]", 3)]);
    let mut harness = Harness::builder()
        .with_size([1440.0, 900.0])
        .build_ui_state(|ui, app| app.show(ui), app);
    settle(&mut harness);
    harness.get_by_label("Note information").click();
    harness.run_steps(3);
    harness.get_by_label("Links").click();
    harness.run_steps(2);
    harness.get_by_label("Toggle focus mode").click();
    harness.run_steps(16);
    assert!(harness.query_by_label("Inspector").is_none());

    harness.get_by_label("Toggle focus mode").click();
    harness.run_steps(16);
    harness.get_by_role_and_label(Role::Window, "Inspector");
    assert_eq!(
        harness.get_by_label("Links").accesskit_node().toggled(),
        Some(eframe::egui::accesskit::Toggled::True)
    );
    assert!(harness.get_by_label("Note body").is_focused());
}

#[test]
fn focus_transition_respects_reduced_motion() {
    assert_eq!(
        crane_desktop::view_state::focus_transition_seconds(false),
        0.18
    );
    assert_eq!(
        crane_desktop::view_state::focus_transition_seconds(true),
        0.0
    );

    let (app, _) = app(vec![note("a", "Alpha", "body", 1)]);
    let mut harness = Harness::builder()
        .with_size([1000.0, 780.0])
        .build_ui_state(|ui, app| app.show(ui), app);
    harness.get_by_label("Reduce motion").click();
    harness.run_steps(2);
    harness.get_by_label("Toggle focus mode").click();
    harness.run();
    assert!(harness
        .query_by_role_and_label(Role::Button, "Notes")
        .is_none());
}

#[test]
fn escape_after_overlays_close_exits_focus_mode() {
    let (app, _) = app(vec![note("a", "Alpha", "first", 3)]);
    let mut harness = Harness::builder()
        .with_size([1240.0, 780.0])
        .build_ui_state(|ui, app| app.show(ui), app);
    settle(&mut harness);
    harness.get_by_label("Toggle focus mode").click();
    harness.run_steps(2);
    assert!(harness
        .query_by_role_and_label(Role::Button, "Notes")
        .is_none());

    harness.key_press_modifiers(eframe::egui::Modifiers::CTRL, eframe::egui::Key::O);
    harness.run_steps(4);
    harness.get_by_role_and_label(Role::TextInput, "Quick Open");

    harness.key_press(eframe::egui::Key::Escape);
    harness.run_steps(4);
    assert!(harness
        .query_by_role_and_label(Role::TextInput, "Quick Open")
        .is_none());
    assert!(harness
        .query_by_role_and_label(Role::Button, "Notes")
        .is_none());

    harness.key_press(eframe::egui::Key::Escape);
    harness.run_steps(4);
    harness.get_by_role_and_label(Role::Button, "Notes");
}

fn editor_body_rect(width: f32) -> eframe::egui::Rect {
    let (app, _) = app(vec![note("a", "Alpha", "body", 1)]);
    let harness = Harness::builder()
        .with_size([width, 780.0])
        .build_ui_state(|ui, app| app.show(ui), app);
    harness
        .get_by_role_and_label(Role::MultilineTextInput, "Note body")
        .rect()
}

#[test]
fn pane_widths_and_centered_editor_measure_follow_layout_contract() {
    let wide = editor_body_rect(1440.0);
    let wider = editor_body_rect(1640.0);
    assert!(
        wide.width() <= EDITOR_MEASURE_MAX + 1.0 && wide.width() >= 660.0,
        "wide editor measure was {:?}",
        wide
    );
    assert!(
        (wider.width() - wide.width()).abs() < 1.0
            && ((wider.min.x - wide.min.x) - 100.0).abs() < 5.0,
        "capped editor did not stay centered as its pane grew: {wide:?} -> {wider:?}"
    );

    let collapsed = editor_body_rect(1000.0);
    let collapsed_available = 1000.0 - COLLAPSED_RAIL_WIDTH - COLLAPSED_NOTE_LIST_WIDTH;
    assert!(
        collapsed.width() <= editor_measure(collapsed_available) + 1.0,
        "collapsed editor exceeded its measure: {collapsed:?}"
    );

    let compact = editor_body_rect(640.0);
    assert!(
        compact.width() <= editor_measure(640.0) + 1.0,
        "compact editor ignored side insets: {compact:?}"
    );
}

#[test]
fn rendering_each_responsive_mode_preserves_the_selected_note() {
    for width in [1440.0, 1000.0, 640.0] {
        let (app, _) = app(vec![note("a", "Alpha", "body", 1)]);
        let harness = Harness::builder()
            .with_size([width, 780.0])
            .build_ui_state(|ui, app| app.show(ui), app);
        assert_eq!(harness.state().selected_note_id(), Some("a"));
    }
}

fn assert_icon_target(
    harness: &Harness<'_, crane_desktop::app::CraneApp>,
    label: &str,
    minimum: f32,
) {
    let node = harness.get_by_label(label);
    let rect = node.rect();
    assert!(
        rect.width() >= minimum && rect.height() >= minimum,
        "{label} was {rect:?}, expected at least {minimum} points"
    );
}

#[test]
fn responsive_layouts_keep_controls_findable_sized_and_rectangular() {
    for width in [1240.0, 800.0] {
        let (app, _) = app(vec![
            note("a", "Alpha", "Work #project/crane", 3),
            note("b", "Beta", "second", 2),
        ]);
        let mut harness = Harness::builder()
            .with_size([width, 780.0])
            .build_ui_state(|ui, app| app.show(ui), app);
        settle(&mut harness);
        harness.get_by_label("Collapse project").click();
        harness.run_steps(2);

        for label in [
            "New note",
            "Quick Open",
            "Note information",
            "Toggle focus mode",
            "Note actions",
            "Heading",
            "Task list",
            "Bulleted list",
            "Numbered list",
            "Quote",
            "Bold",
            "Italic",
            "Inline code",
            "Code block",
            "Insert link",
            "Expand project",
        ] {
            assert_icon_target(&harness, label, 40.0);
        }

        let row = harness.get_by_label("Alpha").rect();
        assert!(
            row.width() > row.height(),
            "note row was not rectangular: {row:?}"
        );
    }
}

#[test]
fn compact_mode_targets_are_at_least_44_points() {
    let (app, _) = app(vec![note("a", "Alpha", "Work #project/crane", 3)]);
    let mut harness = Harness::builder()
        .with_size([640.0, 780.0])
        .build_ui_state(|ui, app| app.show(ui), app);
    settle(&mut harness);

    for label in [
        "Note information",
        "Toggle focus mode",
        "Note actions",
        "Bold",
    ] {
        assert_icon_target(&harness, label, 44.0);
    }

    harness.get_by_label("Back to notes").click();
    harness.run_steps(2);
    assert_icon_target(&harness, "Quick Open", 44.0);
    assert_icon_target(&harness, "New note", 44.0);

    harness.get_by_label("Open library").click();
    settle(&mut harness);
    harness.get_by_label("Collapse project").click();
    harness.run_steps(2);
    assert_icon_target(&harness, "Expand project", 44.0);
}

#[test]
fn collapsed_reduce_motion_control_keeps_a_full_touch_target() {
    let (app, _) = app(vec![note("a", "Alpha", "Body", 1)]);
    let harness = Harness::builder()
        .with_size([1000.0, 780.0])
        .build_ui_state(|ui, app| app.show(ui), app);

    let rect = harness
        .get_by_role_and_label(Role::CheckBox, "Reduce motion")
        .rect();
    assert!(rect.height() >= 40.0, "reduced motion target was {rect:?}");
}

#[test]
fn minimum_window_keeps_formatting_controls_inside_the_editor_viewport() {
    let (app, _) = app(vec![note("a", "Alpha", "Body", 1)]);
    let harness = Harness::builder()
        .with_size([720.0, 560.0])
        .build_ui_state(|ui, app| app.show(ui), app);

    let body = harness
        .get_by_role_and_label(Role::MultilineTextInput, "Note body")
        .rect();
    let bold = harness.get_by_role_and_label(Role::Button, "Bold").rect();
    assert!(
        bold.max.y <= 560.0,
        "formatting controls were outside the minimum viewport: {bold:?}"
    );
    assert!(
        body.max.y <= bold.min.y,
        "editor body overlapped the formatting controls: {body:?} vs {bold:?}"
    );
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
        assert!(brightness(colors.editor) > brightness(colors.rail));
        assert!(brightness(colors.rail) > brightness(colors.note_list));

        let overlay = crane_desktop::theme::overlay_frame(choice);
        let menu = crane_desktop::theme::menu_frame(choice);
        let tokens = crane_desktop::theme::tokens(choice);
        assert_eq!(overlay.fill, tokens.editor_background);
        assert_eq!(menu.fill, tokens.surface_selected);
        assert_eq!(overlay.stroke.color, tokens.rule_strong);
        assert!(overlay.corner_radius.nw >= 8);
        assert!(overlay.shadow.blur > 0);
        assert!(menu.shadow.blur > 0);
    }
}
