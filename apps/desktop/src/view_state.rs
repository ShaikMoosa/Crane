use crane_application::knowledge::CollectionView;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LayoutMode {
    Wide,
    CollapsedRail,
    SinglePane,
}

pub fn layout_mode(width_points: f32) -> LayoutMode {
    if width_points > 900.0 {
        LayoutMode::Wide
    } else if width_points > 680.0 {
        LayoutMode::CollapsedRail
    } else {
        LayoutMode::SinglePane
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
