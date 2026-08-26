use serde_json::Value;

#[derive(Clone, Debug)]
pub struct ActivityBarItem {
    pub id: String,
    pub plugin_id: String,
    pub icon: String,
    pub tooltip: String,
    pub command: String,
}

#[derive(Clone, Debug)]
pub struct StatusBarItem {
    pub id: String,
    pub plugin_id: String,
    pub text: String,
    pub icon: Option<String>,
    pub command: Option<String>,
    pub alignment: StatusBarAlignment,
}

#[derive(Clone, Debug, PartialEq)]
pub enum StatusBarAlignment {
    Left,
    Right,
}

#[derive(Clone, Debug)]
pub struct SidebarItem {
    pub id: String,
    pub plugin_id: String,
    pub title: String,
    pub ui_ast: Value,
}

#[derive(Clone, Debug)]
pub struct PanelItem {
    pub id: String,
    pub plugin_id: String,
    pub title: String,
    pub ui_ast: Value,
}

#[derive(Default)]
pub struct UIExtensionRegistry {
    pub activity_bar_items: Vec<ActivityBarItem>,
    pub status_bar_items: Vec<StatusBarItem>,
    pub sidebar_items: Vec<SidebarItem>,
    pub panel_items: Vec<PanelItem>,
}

impl UIExtensionRegistry {
    pub fn register_activity_bar_item(&mut self, item: ActivityBarItem) {
        self.activity_bar_items.retain(|i| i.id != item.id);
        self.activity_bar_items.push(item);
    }

    pub fn register_status_bar_item(&mut self, item: StatusBarItem) {
        self.status_bar_items.retain(|i| i.id != item.id);
        self.status_bar_items.push(item);
    }

    pub fn register_sidebar_item(&mut self, item: SidebarItem) {
        self.sidebar_items.retain(|i| i.id != item.id);
        self.sidebar_items.push(item);
    }

    pub fn register_panel_item(&mut self, item: PanelItem) {
        self.panel_items.retain(|i| i.id != item.id);
        self.panel_items.push(item);
    }
}
