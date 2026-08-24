use serde_json::Value;
use std::sync::mpsc::SyncSender;
use super::action::PluginAction;

pub fn handle_invoke(plugin_id: &str, request_json: &str, action_tx: &SyncSender<PluginAction>) -> String {
    if let Ok(req) = serde_json::from_str::<Value>(request_json) {
        let api = req["api"].as_str().unwrap_or("");
        
        match api {
            "system.ping" => {
                r#"{"status": "ok", "result": "pong"}"#.to_string()
            }
            "editor.open_tab" => {
                let title = req["args"]["title"].as_str().unwrap_or("Untitled").to_string();
                let content = req["args"]["content"].as_str().unwrap_or("").to_string();
                
                if let Err(e) = action_tx.send(PluginAction::OpenTab { title, content }) {
                    return format!(r#"{{"status": "error", "message": "Channel send failed: {}"}}"#, e);
                }
                r#"{"status": "queued"}"#.to_string()
            }
            "panel.open" => {
                let id = req["args"]["id"].as_str().unwrap_or("unknown").to_string();
                if let Err(e) = action_tx.send(PluginAction::OpenPanel { id }) {
                    return format!(r#"{{"status": "error", "message": "Channel send failed: {}"}}"#, e);
                }
                r#"{"status": "queued"}"#.to_string()
            }
            _ => {
                format!(r#"{{"status": "error", "message": "Unknown API: {}"}}"#, api)
            }
        }
    } else {
        r#"{"status": "error", "message": "Invalid JSON"}"#.to_string()
    }
}
