use crane_application::knowledge::CollectionView;

pub const RAIL_WIDTH: f32 = 208.0;
pub const COLLAPSED_RAIL_WIDTH: f32 = 64.0;
pub const NOTE_LIST_WIDTH: f32 = 350.0;
pub const COLLAPSED_NOTE_LIST_WIDTH: f32 = 320.0;
pub const EDITOR_MEASURE_MAX: f32 = 700.0;
pub const EDITOR_SIDE_INSET: f32 = 16.0;
pub const INSPECTOR_WIDTH: f32 = 320.0;
const EDITOR_MEASURE_MIN_WITH_INSPECTOR: f32 = 660.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LayoutMode {
    Wide,
    CollapsedRail,
    SinglePane,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InspectorPresentation {
    Docked,
    Sheet,
}

pub fn layout_mode(width_points: f32) -> LayoutMode {
    if width_points >= 1200.0 {
        LayoutMode::Wide
    } else if width_points >= 720.0 {
        LayoutMode::CollapsedRail
    } else {
        LayoutMode::SinglePane
    }
}

pub fn editor_measure(available_points: f32) -> f32 {
    (available_points - EDITOR_SIDE_INSET * 2.0).clamp(0.0, EDITOR_MEASURE_MAX)
}

pub fn inspector_presentation(editor_pane_points: f32) -> InspectorPresentation {
    let remaining = (editor_pane_points - INSPECTOR_WIDTH).max(0.0);
    if editor_measure(remaining) >= EDITOR_MEASURE_MIN_WITH_INSPECTOR {
        InspectorPresentation::Docked
    } else {
        InspectorPresentation::Sheet
    }
}

pub fn focus_transition_seconds(reduced_motion: bool) -> f32 {
    if reduced_motion {
        0.0
    } else {
        0.18
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ViewState {
    pub collection: CollectionView,
    pub tag: Option<String>,
    pub query: String,
    pub selected_id: Option<String>,
    pub preview: bool,
    pub status: Option<String>,
    pub focus_mode: bool,
}

impl Default for ViewState {
    fn default() -> Self {
        Self {
            collection: CollectionView::Notes,
            tag: None,
            query: String::new(),
            selected_id: None,
            preview: false,
            status: None,
            focus_mode: false,
        }
    }
}
