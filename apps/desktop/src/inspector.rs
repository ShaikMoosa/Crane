use crate::icons::{icon_button, IconId};
use crane_application::knowledge::{KnowledgeView, LinkState};
use crane_domain::knowledge::{HeadingResolution, NoteFacts};
use crane_storage::ThemeChoice;
use eframe::egui;
use std::ops::Range;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum InspectorTab {
    #[default]
    Statistics,
    Outline,
    Links,
    Backlinks,
}

impl InspectorTab {
    fn label(self) -> &'static str {
        match self {
            InspectorTab::Statistics => "Statistics",
            InspectorTab::Outline => "Outline",
            InspectorTab::Links => "Links",
            InspectorTab::Backlinks => "Backlinks",
        }
    }
}

#[derive(Clone, Debug)]
pub struct InspectorState {
    pub open: bool,
    pub tab: InspectorTab,
}

impl Default for InspectorState {
    fn default() -> Self {
        Self {
            open: false,
            tab: InspectorTab::Statistics,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum InspectorIntent {
    OpenNote {
        note_id: String,
        heading: Option<HeadingResolution>,
    },
    JumpToHeading {
        range: Range<usize>,
    },
    None,
}

/// Render the inspector column. Statistics and Outline come from live draft
/// facts; Links and Backlinks come from the committed knowledge view only, so
/// unsaved text can never surface as a link or backlink.
#[allow(clippy::too_many_arguments)]
pub fn show(
    ui: &mut egui::Ui,
    state: &mut InspectorState,
    draft_facts: &NoteFacts,
    committed: Option<&KnowledgeView>,
    draft_dirty: bool,
    theme: ThemeChoice,
) -> InspectorIntent {
    crate::theme::paint_pane(ui, crate::theme::PaneKind::NoteList, theme);
    let mut intent = InspectorIntent::None;
    ui.horizontal(|ui| {
        for tab in [
            InspectorTab::Statistics,
            InspectorTab::Outline,
            InspectorTab::Links,
            InspectorTab::Backlinks,
        ] {
            if ui
                .add(egui::Button::new(tab.label()).selected(state.tab == tab))
                .clicked()
            {
                state.tab = tab;
            }
        }
        if icon_button(ui, IconId::X, "Close inspector", "Close inspector").clicked() {
            state.open = false;
        }
    });
    ui.separator();
    egui::ScrollArea::vertical()
        .id_salt("crane-inspector-scroll")
        .show(ui, |ui| match state.tab {
            InspectorTab::Statistics => show_statistics(ui, draft_facts),
            InspectorTab::Outline => {
                if let Some(jump) = show_outline(ui, draft_facts) {
                    intent = InspectorIntent::JumpToHeading { range: jump };
                }
            }
            InspectorTab::Links => {
                if let Some(open) = show_links(ui, committed, draft_dirty) {
                    intent = InspectorIntent::OpenNote {
                        note_id: open,
                        heading: None,
                    };
                }
            }
            InspectorTab::Backlinks => {
                if let Some(open) = show_backlinks(ui, committed, draft_dirty) {
                    intent = InspectorIntent::OpenNote {
                        note_id: open,
                        heading: None,
                    };
                }
            }
        });
    intent
}

fn show_statistics(ui: &mut egui::Ui, facts: &NoteFacts) {
    let stats = &facts.statistics;
    ui.label(format!("{} words", stats.words));
    ui.label(format!("{} characters", stats.characters));
    ui.label(format!("{} headings", stats.headings));
    ui.label(format!("{} tasks", stats.tasks));
    if stats.tasks > 0 {
        ui.label(format!("{} of {} done", stats.completed_tasks, stats.tasks));
    }
}

fn show_outline(ui: &mut egui::Ui, facts: &NoteFacts) -> Option<Range<usize>> {
    if facts.headings.is_empty() {
        ui.label("No headings yet");
        return None;
    }
    let mut jump = None;
    for heading in &facts.headings {
        ui.horizontal(|ui| {
            ui.add_space(f32::from(heading.level.saturating_sub(1)) * 12.0);
            if ui.button(&heading.text).clicked() {
                jump = Some(heading.range.clone());
            }
        });
    }
    jump
}

fn show_links(
    ui: &mut egui::Ui,
    committed: Option<&KnowledgeView>,
    draft_dirty: bool,
) -> Option<String> {
    let Some(view) = committed else {
        ui.spinner();
        return None;
    };
    if draft_dirty {
        ui.small("Save to update links");
    }
    if view.outgoing.is_empty() {
        ui.label("No outgoing links");
        return None;
    }
    let mut open = None;
    for outgoing in &view.outgoing {
        match &outgoing.state {
            LinkState::Resolved { note_id, .. } => {
                let name = format!("Open linked note: {}", outgoing.link.display_text);
                if ui
                    .button(&outgoing.link.display_text)
                    .accessible_name(&name)
                    .clicked()
                {
                    open = Some(note_id.clone());
                }
            }
            LinkState::Unresolved => {
                ui.label(&outgoing.link.display_text);
                ui.small("Link target not found");
            }
            LinkState::Ambiguous => {
                ui.label(&outgoing.link.display_text);
                ui.small("Multiple notes share this title");
            }
        }
    }
    open
}

fn show_backlinks(
    ui: &mut egui::Ui,
    committed: Option<&KnowledgeView>,
    draft_dirty: bool,
) -> Option<String> {
    let Some(view) = committed else {
        ui.spinner();
        return None;
    };
    if draft_dirty {
        ui.small("Save to update links");
    }
    if view.backlinks.is_empty() {
        ui.label("No backlinks yet");
        return None;
    }
    let mut open = None;
    for backlink in &view.backlinks {
        let name = format!("Open backlink: {}", backlink.source_title);
        if ui
            .button(&backlink.source_title)
            .accessible_name(&name)
            .clicked()
        {
            open = Some(backlink.source_note_id.clone());
        }
        ui.small(&backlink.context_excerpt);
    }
    open
}
