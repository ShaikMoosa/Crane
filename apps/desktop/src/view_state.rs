use crane_domain::{Collection, NoteFilter};

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
    pub collection: Collection,
    pub tag: Option<String>,
    pub query: String,
    pub selected_id: Option<String>,
    pub preview: bool,
    pub status: Option<String>,
}

impl Default for ViewState {
    fn default() -> Self {
        Self {
            collection: Collection::Notes,
            tag: None,
            query: String::new(),
            selected_id: None,
            preview: false,
            status: None,
        }
    }
}

impl ViewState {
    pub fn filter(&self) -> NoteFilter {
        NoteFilter {
            collection: self.collection,
            tag: self.tag.clone(),
            query: self.query.clone(),
        }
    }
}
