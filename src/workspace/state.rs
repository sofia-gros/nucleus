use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceState {
    pub left_sidebar_width: f32,
    pub right_sidebar_width: f32,
    pub bottom_panel_height: f32,
    pub left_sidebar_open: bool,
    pub right_sidebar_open: bool,
    pub bottom_panel_open: bool,
    pub open_tabs: Vec<String>,
    pub active_tab_index: usize,
}

impl Default for WorkspaceState {
    fn default() -> Self {
        Self {
            left_sidebar_width: 250.0,
            right_sidebar_width: 250.0,
            bottom_panel_height: 200.0,
            left_sidebar_open: true,
            right_sidebar_open: false,
            bottom_panel_open: false,
            open_tabs: Vec::new(),
            active_tab_index: 0,
        }
    }
}
