use serde_json::{Value, json};
use std::sync::{Arc, RwLock};
use std::path::PathBuf;
use gpui::*;

pub struct SettingsStore {
    global_settings: Value,
    workspace_settings: Value,
}

impl SettingsStore {
    pub fn new() -> Self {
        let mut store = Self {
            global_settings: json!({}),
            workspace_settings: json!({}),
        };
        store.load_global();
        store.load_workspace();
        store
    }

    fn global_settings_path() -> Option<PathBuf> {
        dirs::config_dir().map(|mut p| {
            p.push("nucleus");
            p.push("settings.json");
            p
        })
    }

    fn workspace_settings_path() -> PathBuf {
        PathBuf::from(".nucleus").join("workspace.json")
    }

    fn load_global(&mut self) {
        if let Some(path) = Self::global_settings_path() {
            if let Ok(content) = std::fs::read_to_string(path) {
                if let Ok(val) = serde_json::from_str(&content) {
                    self.global_settings = val;
                }
            }
        }
    }

    fn load_workspace(&mut self) {
        let path = Self::workspace_settings_path();
        if let Ok(content) = std::fs::read_to_string(path) {
            if let Ok(val) = serde_json::from_str(&content) {
                self.workspace_settings = val;
            }
        }
    }

    pub fn save_workspace(&self) {
        let path = Self::workspace_settings_path();
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Ok(json_str) = serde_json::to_string_pretty(&self.workspace_settings) {
            let _ = std::fs::write(path, json_str);
        }
    }

    pub fn get(&self, key: &str) -> Option<Value> {
        // Try workspace settings first
        let parts: Vec<&str> = key.split('.').collect();
        if let Some(val) = Self::get_from_value(&self.workspace_settings, &parts) {
            return Some(val);
        }
        // Fallback to global
        Self::get_from_value(&self.global_settings, &parts)
    }

    fn get_from_value(value: &Value, parts: &[&str]) -> Option<Value> {
        let mut current = value;
        for part in parts {
            if let Some(obj) = current.as_object() {
                if let Some(next) = obj.get(*part) {
                    current = next;
                } else {
                    return None;
                }
            } else {
                return None;
            }
        }
        Some(current.clone())
    }

    pub fn set(&mut self, key: &str, value: Value) {
        let parts: Vec<&str> = key.split('.').collect();
        if parts.is_empty() { return; }
        
        let mut current = &mut self.workspace_settings;
        for i in 0..parts.len() - 1 {
            let part = parts[i];
            if !current.is_object() {
                *current = json!({});
            }
            current = current.as_object_mut().unwrap().entry(part.to_string()).or_insert(json!({}));
        }
        
        if let Some(obj) = current.as_object_mut() {
            obj.insert(parts.last().unwrap().to_string(), value);
        }
        
        self.save_workspace();
    }
}

pub struct SettingsGlobal(pub Arc<RwLock<SettingsStore>>);
impl Global for SettingsGlobal {}
