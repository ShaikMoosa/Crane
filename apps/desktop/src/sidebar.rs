use crane_application::knowledge::{CollectionView, SidebarCounts};
use crane_domain::knowledge::TagNode;
use crane_storage::ThemeChoice;
use eframe::egui;
use std::collections::HashSet;

#[derive(Clone, Debug, Default)]
pub struct SidebarState {
    pub collapsed_tags: HashSet<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SidebarIntent {
    Create,
    Collection(CollectionView),
    Tag(Option<String>),
}

pub struct SidebarView<'a> {
    pub counts: &'a SidebarCounts,
    pub tag_tree: &'a [TagNode],
    pub active: CollectionView,
    pub active_tag: Option<&'a str>,
    pub collapsed: bool,
    pub mutations_enabled: bool,
    pub theme: ThemeChoice,
}

pub fn show(
    ui: &mut egui::Ui,
    state: &mut SidebarState,
    view: SidebarView<'_>,
) -> Option<SidebarIntent> {
    let SidebarView {
        counts,
        tag_tree,
        active,
        active_tag,
        collapsed,
        mutations_enabled,
        theme,
    } = view;
    let mut intent = None;
    let identity = ui.heading(if collapsed { "C" } else { "Crane" });
    identity.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Heading, true, "Crane"));
    ui.separator();
    egui::ScrollArea::vertical()
        .id_salt("crane-library-tags")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            if let Some(clicked) = show_collections(
                ui,
                counts,
                active,
                active_tag,
                collapsed,
                mutations_enabled,
                theme,
            ) {
                intent = Some(clicked);
            }
            if !tag_tree.is_empty() {
                ui.separator();
                if !collapsed {
                    ui.small("TAGS");
                }
                for node in tag_tree {
                    if let Some(clicked) = show_tag(
                        ui,
                        node,
                        state,
                        active_tag,
                        collapsed,
                        mutations_enabled,
                        theme,
                    ) {
                        intent = Some(clicked);
                    }
                }
            }
        });
    ui.allocate_space(ui.available_size());
    intent
}

fn show_collections(
    ui: &mut egui::Ui,
    counts: &SidebarCounts,
    active: CollectionView,
    active_tag: Option<&str>,
    collapsed: bool,
    mutations_enabled: bool,
    theme: ThemeChoice,
) -> Option<SidebarIntent> {
    let mut intent = None;
    for (collection, label, short, count) in [
        (CollectionView::Notes, "Notes", "N", counts.notes),
        (CollectionView::Pinned, "Pinned", "P", counts.pinned),
        (CollectionView::Today, "Today", "T", counts.today),
        (CollectionView::Todos, "Todos", "D", counts.todos),
        (CollectionView::Untagged, "Untagged", "U", counts.untagged),
        (CollectionView::Archive, "Archive", "A", counts.archive),
        (CollectionView::Trash, "Trash", "R", counts.trash),
    ] {
        let response = destination_row(
            ui,
            label,
            short,
            Some(count),
            active == collection && active_tag.is_none(),
            collapsed,
            mutations_enabled,
            theme,
        );
        if response.clicked() {
            intent = Some(SidebarIntent::Collection(collection));
        }
    }
    intent
}

fn show_tag(
    ui: &mut egui::Ui,
    node: &TagNode,
    state: &mut SidebarState,
    active_tag: Option<&str>,
    collapsed: bool,
    mutations_enabled: bool,
    theme: ThemeChoice,
) -> Option<SidebarIntent> {
    let mut intent = None;
    let is_collapsed = state.collapsed_tags.contains(&node.path);
    ui.horizontal(|ui| {
        ui.add_space(node.depth as f32 * 14.0);
        if !node.children.is_empty() {
            let (glyph, name) = if is_collapsed {
                (">", format!("Expand {}", node.path))
            } else {
                ("v", format!("Collapse {}", node.path))
            };
            let edge = crate::icons::target_size(ui);
            let toggle = ui
                .add_enabled(
                    mutations_enabled,
                    egui::Button::new(glyph).min_size(egui::vec2(edge, edge)),
                )
                .accessible_name(&name)
                .on_hover_text(&name);
            if toggle.clicked() {
                if is_collapsed {
                    state.collapsed_tags.remove(&node.path);
                } else {
                    state.collapsed_tags.insert(node.path.clone());
                }
            }
        }
        let label = format!("#{}", node.path);
        let response = destination_row(
            ui,
            &label,
            "#",
            Some(node.note_count),
            active_tag == Some(node.path.as_str()),
            collapsed,
            mutations_enabled,
            theme,
        );
        if response.clicked() {
            intent = Some(SidebarIntent::Tag(Some(node.path.clone())));
        }
    });
    if !is_collapsed {
        for child in &node.children {
            if let Some(clicked) = show_tag(
                ui,
                child,
                state,
                active_tag,
                collapsed,
                mutations_enabled,
                theme,
            ) {
                intent = Some(clicked);
            }
        }
    }
    intent
}

#[allow(clippy::too_many_arguments)]
fn destination_row(
    ui: &mut egui::Ui,
    label: &str,
    collapsed_label: &str,
    count: Option<usize>,
    selected: bool,
    collapsed: bool,
    enabled: bool,
    theme_choice: ThemeChoice,
) -> egui::Response {
    let palette = crate::theme::tokens(theme_choice);
    let edge = crate::icons::target_size(ui);
    let (rect, response) =
        ui.allocate_exact_size(egui::vec2(ui.available_width(), edge), egui::Sense::click());
    let fill = if selected {
        palette.surface_selected
    } else if response.hovered() {
        palette.surface_hover
    } else {
        egui::Color32::TRANSPARENT
    };
    ui.painter().rect_filled(rect.shrink(2.0), 6, fill);
    if selected {
        ui.painter().rect_filled(
            egui::Rect::from_min_max(
                egui::pos2(rect.left() + 2.0, rect.top() + 7.0),
                egui::pos2(rect.left() + 5.0, rect.bottom() - 7.0),
            ),
            2,
            palette.accent,
        );
    }
    let color = if enabled {
        palette.text_primary
    } else {
        palette.text_disabled
    };
    ui.painter().text(
        egui::pos2(
            if collapsed {
                rect.center().x
            } else {
                rect.left() + 13.0
            },
            rect.center().y,
        ),
        if collapsed {
            egui::Align2::CENTER_CENTER
        } else {
            egui::Align2::LEFT_CENTER
        },
        if collapsed { collapsed_label } else { label },
        egui::TextStyle::Button.resolve(ui.style()),
        color,
    );
    if !collapsed {
        if let Some(count) = count {
            ui.painter().text(
                egui::pos2(rect.right() - 10.0, rect.center().y),
                egui::Align2::RIGHT_CENTER,
                count.to_string(),
                egui::TextStyle::Small.resolve(ui.style()),
                palette.text_tertiary,
            );
        }
    }
    let response = response.on_hover_text(match count {
        Some(count) => format!("{label} · {count}"),
        None => label.to_owned(),
    });
    response
        .widget_info(|| egui::WidgetInfo::selected(egui::Role::Button, enabled, selected, label));
    response
}
