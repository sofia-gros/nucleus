/// プラグインからのホスト API 呼び出しをディスパッチするルーターモジュール

use serde_json::Value;
use std::sync::mpsc::SyncSender;
use super::action::PluginAction;

/// プラグインからの JSON API リクエストを処理し、レスポンス JSON を返す
pub fn handle_invoke(
    plugin_id: &str, 
    request_json: &str, 
    action_tx: &SyncSender<PluginAction>,
    settings: &std::sync::RwLock<crate::settings::SettingsStore>,
    permissions: &crate::plugin_manager::manifest::PluginPermissions,
) -> String {
    if let Ok(req) = serde_json::from_str::<Value>(request_json) {
        let api = req["api"].as_str().unwrap_or("");
        
        match api {
            "system.ping" => {
                r#"{"status": "ok", "result": "pong"}"#.to_string()
            }
            "workspace.get_root_path" => {
                let store = settings.read().unwrap();
                let root_opt = store.get("last_opened_workspace")
                    .and_then(|v| v.as_str().map(|s| s.to_string()))
                    .or_else(|| std::env::current_dir().ok().map(|p| p.to_string_lossy().to_string()));

                if let Some(path) = root_opt {
                    format!(r#"{{"status": "ok", "path": "{}"}}"#, path.replace("\\", "/"))
                } else {
                    r#"{"status": "ok", "path": null}"#.to_string()
                }
            }
            "editor.open_tab" | "workspace.open_tab" => {
                let title = req["args"]["title"].as_str().unwrap_or("Untitled").to_string();
                let path = req["args"]["path"].as_str().unwrap_or(&title).to_string();
                let content = req["args"]["content"].as_str().unwrap_or("").to_string();
                
                if let Err(e) = action_tx.send(PluginAction::OpenTab { path, title, content }) {
                    return format!(r#"{{"status": "error", "message": "Channel send failed: {}"}}"#, e);
                }
                r#"{"status": "queued"}"#.to_string()
            }
            "workspace.show_notification" => {
                let message = req["args"]["message"].as_str().unwrap_or("").to_string();
                if let Err(e) = action_tx.send(PluginAction::ShowNotification { message }) {
                    return format!(r#"{{"status": "error", "message": "Channel send failed: {}"}}"#, e);
                }
                r#"{"status": "ok"}"#.to_string()
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
            "process.exec" => {
                // 同期プロセス実行（結果を即座に返す）
                if permissions.process.is_empty() {
                    return r#"{"status": "error", "message": "Permission denied: process access not granted"}"#.to_string();
                }
                let command = req["args"]["command"].as_str().unwrap_or("");
                let args: Vec<String> = req["args"]["args"].as_array().unwrap_or(&vec![]).iter().map(|v| v.as_str().unwrap_or("").to_string()).collect();
                let cwd = req["args"]["cwd"].as_str();

                let mut cmd = std::process::Command::new(command);
                cmd.args(&args);
                if let Some(dir) = cwd {
                    cmd.current_dir(dir);
                }

                match cmd.output() {
                    Ok(output) => {
                        let stdout = String::from_utf8_lossy(&output.stdout);
                        let stderr = String::from_utf8_lossy(&output.stderr);
                        let code = output.status.code().unwrap_or(-1);

                        let json_val = serde_json::json!({
                            "status": "ok",
                            "code": code,
                            "stdout": stdout,
                            "stderr": stderr,
                        });
                        serde_json::to_string(&json_val).unwrap_or_else(|_| r#"{"status": "error"}"#.to_string())
                    }
                    Err(e) => {
                        format!(r#"{{"status": "error", "message": "{}"}}"#, e)
                    }
                }
            }
            "process.spawn" => {
                if permissions.process.is_empty() {
                    return r#"{"status": "error", "message": "Permission denied: process access not granted"}"#.to_string();
                }
                let id = req["args"]["id"].as_str().unwrap_or("unknown").to_string();
                let command = req["args"]["command"].as_str().unwrap_or("").to_string();
                let args: Vec<String> = req["args"]["args"].as_array().unwrap_or(&vec![]).iter().map(|v| v.as_str().unwrap_or("").to_string()).collect();
                let cwd = req["args"]["cwd"].as_str().map(|s| s.to_string());
                
                let tx = action_tx.clone();
                std::thread::spawn(move || {
                    let mut cmd = std::process::Command::new(command);
                    cmd.args(args);
                    if let Some(dir) = cwd {
                        cmd.current_dir(dir);
                    }
                    let output = cmd.output();
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
            "fs.read_file" => {
                let path = req["args"]["path"].as_str().unwrap_or("").to_string();
                let req_id = req["args"]["req_id"].as_str().unwrap_or("").to_string();
                
                if permissions.filesystem.is_empty() {
                    return r#"{"status": "error", "message": "Permission denied: filesystem access not granted"}"#.to_string();
                }

                if let Err(e) = action_tx.send(PluginAction::FileSystemRead { plugin_id: plugin_id.to_string(), req_id, path }) {
                    return format!(r#"{{"status": "error", "message": "Channel send failed: {}"}}"#, e);
                }
                r#"{"status": "queued"}"#.to_string()
            }
            "fs.write_file" => {
                let path = req["args"]["path"].as_str().unwrap_or("").to_string();
                let content = req["args"]["content"].as_str().unwrap_or("").to_string();
                let req_id = req["args"]["req_id"].as_str().unwrap_or("").to_string();
                
                if permissions.filesystem.is_empty() {
                    return r#"{"status": "error", "message": "Permission denied: filesystem access not granted"}"#.to_string();
                }

                if let Err(e) = action_tx.send(PluginAction::FileSystemWrite { plugin_id: plugin_id.to_string(), req_id, path, content }) {
                    return format!(r#"{{"status": "error", "message": "Channel send failed: {}"}}"#, e);
                }
                r#"{"status": "queued"}"#.to_string()
            }
            "command.register" => {
                let command = req["args"]["command"].as_str().unwrap_or("").to_string();
                
                if let Err(e) = action_tx.send(PluginAction::RegisterCommand { plugin_id: plugin_id.to_string(), command }) {
                    return format!(r#"{{"status": "error", "message": "Channel send failed: {}"}}"#, e);
                }
                r#"{"status": "queued"}"#.to_string()
            }
            "ui.register_status_bar_item" => {
                let id = req["args"]["id"].as_str().unwrap_or("").to_string();
                let text = req["args"]["text"].as_str().unwrap_or("").to_string();
                let icon = req["args"]["icon"].as_str().map(|s| s.to_string());
                let command = req["args"]["command"].as_str().map(|s| s.to_string());
                let align = req["args"]["align"].as_str().unwrap_or("left").to_string();

                if let Err(e) = action_tx.send(PluginAction::RegisterStatusBarItem { plugin_id: plugin_id.to_string(), id, text, icon, command, align }) {
                    return format!(r#"{{"status": "error", "message": "Channel send failed: {}"}}"#, e);
                }
                r#"{"status": "queued"}"#.to_string()
            }
            "ui.register_activity_bar_item" => {
                let id = req["args"]["id"].as_str().unwrap_or("").to_string();
                let icon = req["args"]["icon"].as_str().unwrap_or("").to_string();
                let tooltip = req["args"]["tooltip"].as_str().unwrap_or("").to_string();
                let command = req["args"]["command"].as_str().unwrap_or("").to_string();

                if let Err(e) = action_tx.send(PluginAction::RegisterActivityBarItem { plugin_id: plugin_id.to_string(), id, icon, tooltip, command }) {
                    return format!(r#"{{"status": "error", "message": "Channel send failed: {}"}}"#, e);
                }
                r#"{"status": "queued"}"#.to_string()
            }
            "ui.register_sidebar" => {
                let id = req["args"]["id"].as_str().unwrap_or("").to_string();
                let title = req["args"]["title"].as_str().unwrap_or("Sidebar").to_string();
                let ui_ast = req["args"]["ui"].clone();

                if let Err(e) = action_tx.send(PluginAction::RegisterSidebarItem { plugin_id: plugin_id.to_string(), id, title, ui_ast }) {
                    return format!(r#"{{"status": "error", "message": "Channel send failed: {}"}}"#, e);
                }
                r#"{"status": "queued"}"#.to_string()
            }
            "ui.update_sidebar" => {
                let id = req["args"]["id"].as_str().unwrap_or("").to_string();
                let title = req["args"]["title"].as_str().map(|s| s.to_string());
                let ui_ast = req["args"]["ui"].clone();

                if let Err(e) = action_tx.send(PluginAction::UpdateSidebarItem { plugin_id: plugin_id.to_string(), id, title, ui_ast }) {
                    return format!(r#"{{"status": "error", "message": "Channel send failed: {}"}}"#, e);
                }
                r#"{"status": "queued"}"#.to_string()
            }
            "ui.register_panel" => {
                let id = req["args"]["id"].as_str().unwrap_or("").to_string();
                let title = req["args"]["title"].as_str().unwrap_or("Panel").to_string();
                let ui_ast = req["args"]["ui"].clone();

                if let Err(e) = action_tx.send(PluginAction::RegisterPanelItem { plugin_id: plugin_id.to_string(), id, title, ui_ast }) {
                    return format!(r#"{{"status": "error", "message": "Channel send failed: {}"}}"#, e);
                }
                r#"{"status": "queued"}"#.to_string()
            }
            "ui.register_icon_rules" => {
                let mut rules = std::collections::HashMap::new();
                if let Some(obj) = req["args"]["rules"].as_object() {
                    for (k, v) in obj {
                        if let (Some(icon), Some(color)) = (v["icon"].as_str(), v["color"].as_str()) {
                            rules.insert(k.clone(), (icon.to_string(), color.to_string()));
                        }
                    }
                }
                if let Err(e) = action_tx.send(PluginAction::RegisterIconRules { rules }) {
                    return format!(r#"{{"status": "error", "message": "Channel send failed: {}"}}"#, e);
                }
                r#"{"status": "ok"}"#.to_string()
            }
            "ui.register_translations" => {
                let mut dict = std::collections::HashMap::new();
                if let Some(obj) = req["args"]["dict"].as_object() {
                    for (k, v) in obj {
                        if let Some(val_str) = v.as_str() {
                            dict.insert(k.clone(), val_str.to_string());
                        }
                    }
                }
                if let Err(e) = action_tx.send(PluginAction::RegisterTranslations { dict }) {
                    return format!(r#"{{"status": "error", "message": "Channel send failed: {}"}}"#, e);
                }
                r#"{"status": "ok"}"#.to_string()
            }
            _ => {
                format!(r#"{{"status": "error", "message": "Unknown API: {}"}}"#, api)
            }
        }
    } else {
        r#"{"status": "error", "message": "Invalid JSON"}"#.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{mpsc, Arc, RwLock};
    use crate::settings::SettingsStore;
    use crate::plugin_manager::manifest::PluginPermissions;

    #[test]
    fn test_api_router_ping() {
        let (tx, _rx) = mpsc::sync_channel(10);
        let settings = Arc::new(RwLock::new(SettingsStore::new()));
        let permissions = PluginPermissions::default();

        let res = handle_invoke(
            "test_plugin",
            r#"{"api": "system.ping"}"#,
            &tx,
            &settings,
            &permissions,
        );

        assert_eq!(res, r#"{"status": "ok", "result": "pong"}"#);
    }

    #[test]
    fn test_api_router_settings() {
        let (tx, rx) = mpsc::sync_channel(10);
        let settings = Arc::new(RwLock::new(SettingsStore::new()));
        let permissions = PluginPermissions::default();

        // set
        let res_set = handle_invoke(
            "test_plugin",
            r#"{"api": "settings.set", "args": {"key": "test_key", "value": "test_val"}}"#,
            &tx,
            &settings,
            &permissions,
        );
        assert_eq!(res_set, r#"{"status": "queued"}"#);

        // receive action
        let action = rx.try_recv().unwrap();
        if let PluginAction::UpdateSetting { key, value } = action {
            assert_eq!(key, "test_key");
            assert_eq!(value, serde_json::json!("test_val"));
            settings.write().unwrap().set(&key, value);
        } else {
            panic!("Expected UpdateSetting action");
        }

        // get
        let res_get = handle_invoke(
            "test_plugin",
            r#"{"api": "settings.get", "args": {"key": "test_key"}}"#,
            &tx,
            &settings,
            &permissions,
        );
        assert_eq!(res_get, r#"{"status": "ok", "value": "test_val"}"#);
    }
}
