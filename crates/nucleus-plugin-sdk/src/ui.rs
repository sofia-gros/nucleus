/// UI Extension Points API ラッパーモジュール

use crate::invoke_host;
use serde_json::Value;

/// アクティビティバーアイテムの登録
pub fn register_activity_bar_item(id: &str, icon: &str, tooltip: &str, command: &str) -> Result<Value, String> {
    invoke_host("ui.register_activity_bar_item", serde_json::json!({
        "id": id,
        "icon": icon,
        "tooltip": tooltip,
        "command": command
    }))
}

/// ステータスバーアイテムの登録
pub fn register_status_bar_item(id: &str, text: &str, alignment: &str, command: Option<&str>) -> Result<Value, String> {
    invoke_host("ui.register_status_bar_item", serde_json::json!({
        "id": id,
        "text": text,
        "alignment": alignment,
        "command": command
    }))
}

/// サイドバービューの更新
pub fn update_sidebar(id: &str, title: &str, ui_ast: Value) -> Result<Value, String> {
    invoke_host("ui.update_sidebar", serde_json::json!({
        "id": id,
        "title": title,
        "ui_ast": ui_ast
    }))
}

/// 通知バナーの表示
pub fn show_notification(message: &str) -> Result<Value, String> {
    invoke_host("workspace.show_notification", serde_json::json!({
        "message": message
    }))
}
