use crate::icons::{icon_button, IconId};
use crane_application::knowledge::{CollectionView, SidebarCounts};
use crane_domain::knowledge::TagNode;
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
    } = view;
    let mut intent = None;
    ui.heading(if collapsed { "C" } else { "Crane" });
    if collapsed {
        if icon_button(ui, IconId::Plus, "New note", "New note").clicked() && mutations_enabled {
            intent = Some(SidebarIntent::Create);
        }
    } else if ui
        .add_enabled(
            mutations_enabled,
            egui::Button::new("+  New note")
                .min_size(egui::vec2(0.0, crate::icons::target_size(ui))),
        )
        .accessible_name("New note")
        .clicked()
    {
        intent = Some(SidebarIntent::Create);
    }
    ui.separator();
    egui::ScrollArea::vertical()
        .id_salt("crane-library-tags")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            if let Some(clicked) =
                show_collections(ui, counts, active, active_tag, collapsed, mutations_enabled)
            {
                intent = Some(clicked);
            }
            if !tag_tree.is_empty() {
                ui.separator();
                if !collapsed {
                    ui.small("TAGS");
                }
                for node in tag_tree {
                    if let Some(clicked) =
                        show_tag(ui, node, state, active_tag, collapsed, mutations_enabled)
                    {
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
        let expanded = format!("{label}  {count}");
        let text = if collapsed { short } else { &expanded };
        let response = ui.add_enabled(
            mutations_enabled,
            egui::Button::selectable(active == collection && active_tag.is_none(), text),
        );
        let response = if collapsed {
            response.on_hover_text(label)
        } else {
            response
        };
        response.widget_info(|| {
            egui::WidgetInfo::selected(
                egui::Role::Button,
                true,
                active == collection && active_tag.is_none(),
                label,
            )
        });
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
        let response = ui
            .add_enabled(
                mutations_enabled,
                egui::Button::selectable(
                    active_tag == Some(node.path.as_str()),
                    if collapsed { "#" } else { &label },
                ),
            )
            .accessible_name(&label);
        if response.clicked() {
            intent = Some(SidebarIntent::Tag(Some(node.path.clone())));
        }
    });
    if !is_collapsed {
        for child in &node.children {
            if let Some(clicked) =
                show_tag(ui, child, state, active_tag, collapsed, mutations_enabled)
            {
                intent = Some(clicked);
            }
        }
    }
    intent
}
