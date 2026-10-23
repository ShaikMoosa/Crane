use crate::theme;
use crane_domain::{Note, Store};
use crane_storage::ThemeChoice;
use eframe::egui;
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

const ROW_HEIGHT: f32 = 106.0;
const EXCERPT_LIMIT: usize = 140;

pub struct NoteListAction {
    pub selected: Option<String>,
    pub quick_open: bool,
    pub create: bool,
    pub clear_search: bool,
}

#[allow(clippy::too_many_arguments)]
pub fn show(
    ui: &mut egui::Ui,
    store: &Store,
    visible_ids: &[String],
    selected_id: Option<&str>,
    query: &mut String,
    request_search_focus: bool,
    heading: &str,
    theme_choice: ThemeChoice,
    mutations_enabled: bool,
) -> NoteListAction {
    let mut action = NoteListAction {
        selected: None,
        quick_open: false,
        create: false,
        clear_search: false,
    };
    let palette = theme::tokens(theme_choice);

    ui.add_space(8.0);
    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            let heading_response = ui.heading(heading);
            let accessible_heading = format!("{heading} browser");
            heading_response.widget_info(|| {
                egui::WidgetInfo::labeled(egui::Role::Heading, true, accessible_heading.clone())
            });
            ui.label(
                egui::RichText::new(format!(
                    "{} {}",
                    visible_ids.len(),
                    if visible_ids.len() == 1 {
                        "note"
                    } else {
                        "notes"
                    }
                ))
                .small()
                .color(palette.text_secondary),
            );
        });
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui
                .add_enabled_ui(mutations_enabled, |ui| {
                    crate::icons::accent_icon_button(
                        ui,
                        crate::icons::IconId::Plus,
                        "New note",
                        "New note",
                        palette.accent,
                        palette.on_accent,
                    )
                })
                .inner
                .clicked()
            {
                action.create = true;
            }
            if crate::icons::icon_button(
                ui,
                crate::icons::IconId::Ellipsis,
                "Quick Open",
                "Quick Open",
            )
            .clicked()
            {
                action.quick_open = true;
            }
        });
    });
    ui.add_space(4.0);
    let search = ui
        .add(
            egui::TextEdit::singleline(query)
                .hint_text("Search notes")
                .desired_width(f32::INFINITY),
        )
        .accessible_name("Search notes");
    if request_search_focus {
        search.request_focus();
    }
    ui.add_space(4.0);
    ui.separator();

    if visible_ids.is_empty() {
        ui.add_space(32.0);
        ui.vertical_centered(|ui| {
            if query.trim().is_empty() {
                ui.label(format!("No notes in {heading}"));
                if ui.button("Create a note").clicked() {
                    action.create = true;
                }
            } else {
                ui.label(format!("No notes match “{}”", query.trim()));
                if ui.button("Clear search").clicked() {
                    action.clear_search = true;
                }
            }
        });
        return action;
    }

    let notes_by_id: HashMap<&str, &Note> = store
        .notes
        .iter()
        .map(|note| (note.id.as_str(), note))
        .collect();
    let now = wall_clock_millis();
    egui::ScrollArea::vertical()
        .id_salt("crane-note-browser")
        .auto_shrink([false, false])
        .show_rows(ui, ROW_HEIGHT, visible_ids.len(), |ui, range| {
            for index in range {
                let id = &visible_ids[index];
                let Some(note) = notes_by_id.get(id.as_str()).copied() else {
                    continue;
                };
                if note_row(
                    ui,
                    note,
                    selected_id == Some(id.as_str()),
                    now,
                    theme_choice,
                )
                .clicked()
                {
                    action.selected = Some(id.clone());
                }
            }
        });
    action
}

fn note_row(
    ui: &mut egui::Ui,
    note: &Note,
    selected: bool,
    now: u64,
    theme_choice: ThemeChoice,
) -> egui::Response {
    let palette = theme::tokens(theme_choice);
    let title = if note.title.trim().is_empty() {
        "Untitled note"
    } else {
        note.title.trim()
    };
    let (rect, response) = ui.allocate_exact_size(
        egui::vec2(ui.available_width(), ROW_HEIGHT),
        egui::Sense::click(),
    );
    let fill = if selected {
        palette.surface_selected
    } else if response.hovered() {
        palette.surface_hover
    } else {
        egui::Color32::TRANSPARENT
    };
    ui.painter()
        .rect_filled(rect.shrink2(egui::vec2(3.0, 3.0)), 7, fill);
    if selected {
        let marker = egui::Rect::from_min_max(
            egui::pos2(rect.left() + 3.0, rect.top() + 10.0),
            egui::pos2(rect.left() + 6.0, rect.bottom() - 10.0),
        );
        ui.painter().rect_filled(marker, 2, palette.accent);
    }

    let left = rect.left() + 14.0;
    let right = rect.right() - 12.0;
    ui.painter().text(
        egui::pos2(left, rect.top() + 13.0),
        egui::Align2::LEFT_TOP,
        title,
        egui::TextStyle::Button.resolve(ui.style()),
        palette.text_primary,
    );
    ui.painter().text(
        egui::pos2(right, rect.top() + 14.0),
        egui::Align2::RIGHT_TOP,
        relative_time(note.updated_at, now),
        egui::TextStyle::Small.resolve(ui.style()),
        palette.text_tertiary,
    );

    let excerpt = plain_excerpt(&note.body);
    let excerpt_text = excerpt_lines(&excerpt).join("\n");
    let excerpt_response = ui.put(
        egui::Rect::from_min_size(
            egui::pos2(left, rect.top() + 35.0),
            egui::vec2((right - left).max(0.0), 32.0),
        ),
        egui::Label::new(
            egui::RichText::new(excerpt_text)
                .small()
                .color(palette.text_secondary),
        ),
    );
    let response = response.union(excerpt_response);

    let mut chip_x = left;
    for chip in metadata_chips(note) {
        let galley = ui.painter().layout_no_wrap(
            chip.clone(),
            egui::TextStyle::Small.resolve(ui.style()),
            palette.text_secondary,
        );
        let chip_rect = egui::Rect::from_min_size(
            egui::pos2(chip_x, rect.bottom() - 24.0),
            egui::vec2(galley.size().x + 12.0, 18.0),
        );
        ui.painter().rect_filled(chip_rect, 9, palette.accent_soft);
        ui.painter().galley(
            chip_rect.left_center() + egui::vec2(6.0, -galley.size().y * 0.5),
            galley,
            palette.text_secondary,
        );
        chip_x = chip_rect.right() + 6.0;
    }
    ui.painter().line_segment(
        [
            egui::pos2(rect.left() + 10.0, rect.bottom()),
            egui::pos2(rect.right() - 10.0, rect.bottom()),
        ],
        egui::Stroke::new(1.0, palette.rule),
    );

    response.widget_info(|| egui::WidgetInfo::selected(egui::Role::Button, true, selected, title));
    ui.ctx().accesskit_node_builder(response.id, |node| {
        node.set_description(format!(
            "{}; updated {}; {}",
            excerpt,
            relative_time(note.updated_at, now),
            metadata_chips(note).join(", ")
        ));
    });
    response
}

pub fn metadata_chips(note: &Note) -> Vec<String> {
    let mut chips = Vec::with_capacity(2);
    if note.pinned {
        chips.push("Pinned".to_owned());
    } else if note.archived {
        chips.push("Archived".to_owned());
    } else if note.trashed {
        chips.push("Trash".to_owned());
    }
    for tag in &note.tags {
        if chips.len() == 2 {
            break;
        }
        chips.push(format!("#{tag}"));
    }
    chips.truncate(2);
    chips
}

pub fn plain_excerpt(markdown: &str) -> String {
    let mut plain = String::with_capacity(markdown.len().min(EXCERPT_LIMIT));
    let mut chars = markdown.chars().peekable();
    while let Some(ch) = chars.next() {
        if plain.chars().count() >= EXCERPT_LIMIT {
            break;
        }
        match ch {
            ']' if chars.peek() == Some(&'(') => {
                chars.next();
                for url in chars.by_ref() {
                    if url == ')' {
                        break;
                    }
                }
            }
            '#' | '*' | '_' | '~' | '`' | '[' | ']' => {}
            '\r' | '\n' | '\t' => plain.push(' '),
            other => plain.push(other),
        }
    }
    let collapsed = plain.split_whitespace().collect::<Vec<_>>().join(" ");
    if collapsed.chars().count() <= EXCERPT_LIMIT {
        collapsed
    } else {
        collapsed.chars().take(EXCERPT_LIMIT).collect()
    }
}

fn excerpt_lines(excerpt: &str) -> Vec<String> {
    const LINE_CHARS: usize = 58;
    let mut lines = Vec::with_capacity(2);
    let mut words = excerpt.split_whitespace().peekable();
    for _ in 0..2 {
        let mut line = String::new();
        while let Some(word) = words.peek().copied() {
            if !line.is_empty() && line.chars().count() + word.chars().count() + 1 > LINE_CHARS {
                break;
            }
            words.next();
            if !line.is_empty() {
                line.push(' ');
            }
            line.push_str(word);
        }
        if line.is_empty() {
            break;
        }
        lines.push(line);
    }
    lines
}

pub fn relative_time(updated_at: u64, now: u64) -> String {
    const MINUTE: u64 = 60_000;
    const HOUR: u64 = 60 * MINUTE;
    const DAY: u64 = 24 * HOUR;
    let age = now.saturating_sub(updated_at);
    if updated_at >= now || age < MINUTE {
        "now".to_owned()
    } else if age < HOUR {
        format!("{}m", age / MINUTE)
    } else if age < DAY {
        format!("{}h", age / HOUR)
    } else if age < 7 * DAY {
        format!("{}d", age / DAY)
    } else {
        crate::views::restore::format_timestamp_millis(updated_at)
            .split_whitespace()
            .next()
            .unwrap_or("1970-01-01")
            .to_owned()
    }
}

fn wall_clock_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .min(u64::MAX as u128) as u64
}
