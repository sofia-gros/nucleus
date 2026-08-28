/// プラグインによる UI 拡張レジストリおよび UI AST 定義モジュール

use serde_json::Value;

/// Activity Bar（左端の縦アイコンバー）の拡張アイテム
#[derive(Clone, Debug)]
pub struct ActivityBarItem {
    pub id: String,
    pub plugin_id: String,
    pub icon: String,
    pub tooltip: String,
    pub command: String,
}

/// Status Bar（下部ステータスバー）の拡張アイテム
#[derive(Clone, Debug)]
pub struct StatusBarItem {
    pub id: String,
    pub plugin_id: String,
    pub text: String,
    pub icon: Option<String>,
    pub command: Option<String>,
    pub alignment: StatusBarAlignment,
}

/// Status Bar の配置位置
#[derive(Clone, Debug, PartialEq)]
pub enum StatusBarAlignment {
    Left,
    Right,
}

/// サイドバーのカスタムビュー拡張アイテム
#[derive(Clone, Debug)]
pub struct SidebarItem {
    pub id: String,
    pub plugin_id: String,
    pub title: String,
    pub ui_ast: Value,
}

/// ボトムパネルのカスタムビュー拡張アイテム
#[derive(Clone, Debug)]
pub struct PanelItem {
    pub id: String,
    pub plugin_id: String,
    pub title: String,
    pub ui_ast: Value,
}

/// プラグインからの UI 拡張登録を保持するレジストリ
#[derive(Default)]
pub struct UIExtensionRegistry {
    pub activity_bar_items: Vec<ActivityBarItem>,
    pub status_bar_items: Vec<StatusBarItem>,
    pub sidebar_items: Vec<SidebarItem>,
    pub panel_items: Vec<PanelItem>,
}

impl UIExtensionRegistry {
    /// Activity Bar アイテムを登録
    pub fn register_activity_bar_item(&mut self, item: ActivityBarItem) {
        self.activity_bar_items.retain(|i| i.id != item.id);
        self.activity_bar_items.push(item);
    }

    /// Status Bar アイテムを登録（既存IDは上書き更新）
    pub fn register_status_bar_item(&mut self, item: StatusBarItem) {
        self.status_bar_items.retain(|i| i.id != item.id);
        self.status_bar_items.push(item);
    }

    /// サイドバーアイテムを登録
    pub fn register_sidebar_item(&mut self, item: SidebarItem) {
        self.sidebar_items.retain(|i| i.id != item.id);
        self.sidebar_items.push(item);
    }

    /// 既存のサイドバーアイテムを動的更新
    pub fn update_sidebar_item(&mut self, plugin_id: &str, id: &str, title: Option<String>, ui_ast: Value) {
        if let Some(item) = self.sidebar_items.iter_mut().find(|i| i.id == id) {
            if let Some(t) = title {
                item.title = t;
            }
            item.ui_ast = ui_ast;
        } else {
            self.sidebar_items.push(SidebarItem {
                id: id.to_string(),
                plugin_id: plugin_id.to_string(),
                title: title.unwrap_or_else(|| "Sidebar".to_string()),
                ui_ast,
            });
        }
    }

    /// パネルアイテムを登録
    pub fn register_panel_item(&mut self, item: PanelItem) {
        self.panel_items.retain(|i| i.id != item.id);
        self.panel_items.push(item);
    }
}
