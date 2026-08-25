use serde_json::Value;
use std::sync::mpsc::SyncSender;
use super::action::PluginAction;

pub fn handle_invoke(
    plugin_id: &str, 
    request_json: &str, 
    action_tx: &SyncSender<PluginAction>,
    settings: &std::sync::RwLock<crate::settings::SettingsStore>
) -> String {
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
            "settings.get" => {
                let key = req["args"]["key"].as_str().unwrap_or("");
                let store = settings.read().unwrap();
                if let Some(val) = store.get(key) {
                    format!(r#"{{"status": "ok", "value": {}}}"#, val)
                } else {
                    r#"{"status": "error", "message": "Key not found"}"#.to_string()
                }
            }
            "settings.set" => {
                let key = req["args"]["key"].as_str().unwrap_or("").to_string();
                let value = req["args"]["value"].clone();
                if let Err(e) = action_tx.send(PluginAction::UpdateSetting { key, value }) {
                    return format!(r#"{{"status": "error", "message": "Channel send failed: {}"}}"#, e);
                }
                r#"{"status": "queued"}"#.to_string()
            }
            "process.spawn" => {
                let id = req["args"]["id"].as_str().unwrap_or("unknown").to_string();
                let command = req["args"]["command"].as_str().unwrap_or("").to_string();
                let args: Vec<String> = req["args"]["args"].as_array().unwrap_or(&vec![]).iter().map(|v| v.as_str().unwrap_or("").to_string()).collect();
                
                let tx = action_tx.clone();
                std::thread::spawn(move || {
                    let output = std::process::Command::new(command).args(args).output();
                    if let Ok(output) = output {
                        let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
                        let code = output.status.code().unwrap_or(0);
                        let _ = tx.send(PluginAction::InternalProcessOutput { id, stdout, code });
                    }
                });
                
                r#"{"status": "queued"}"#.to_string()
            }
            "terminal.write" => {
                let text = req["args"]["text"].as_str().unwrap_or("").to_string();
                if let Err(e) = action_tx.send(PluginAction::TerminalWrite { text }) {
                    return format!(r#"{{"status": "error", "message": "Channel send failed: {}"}}"#, e);
                }
                r#"{"status": "queued"}"#.to_string()
            }
            "terminal.clear" => {
                if let Err(e) = action_tx.send(PluginAction::TerminalClear) {
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
