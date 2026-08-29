/// グローバル設定（User）およびワークスペース設定（Workspace）の階層化管理モジュール

use serde_json::{Value, json};
use std::sync::{Arc, RwLock};
use std::path::PathBuf;
use gpui::*;

/// 設定のスコープ対象
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SettingsTarget {
    User,
    Workspace,
}

/// 階層化設定ストア
pub struct SettingsStore {
    pub global_settings: Value,
    pub workspace_settings: Value,
    pub workspace_root: Option<PathBuf>,
}

impl SettingsStore {
    /// 新規作成と設定の読み込み
    pub fn new() -> Self {
        let default_global = serde_json::from_str(r#"{
            "theme": "dark",
            "editor": {
                "font_size": 14,
                "tab_size": 4,
                "soft_wrap": false
            },
            "files": {
                "auto_save_interval": 30
            }
        }"#).unwrap_or(Value::Object(serde_json::Map::new()));

        let mut store = Self {
            global_settings: default_global,
            workspace_settings: Value::Object(serde_json::Map::new()),
            workspace_root: None,
        };
        store.load_global();
        store.load_workspace();
        store
    }

    /// ワークスペースルートパスの設定
    pub fn set_workspace_root(&mut self, root: Option<PathBuf>) {
        self.workspace_root = root;
        self.load_workspace();
    }

    fn global_settings_path() -> Option<PathBuf> {
        dirs::config_dir().map(|mut p| {
            p.push("nucleus");
            p.push("settings.json");
            p
        })
    }

    fn workspace_settings_path(&self) -> Option<PathBuf> {
        self.workspace_root.as_ref().map(|root| root.join(".nucleus").join("workspace.json"))
            .or_else(|| Some(PathBuf::from(".nucleus").join("workspace.json")))
    }

    /// グローバル設定の読み込み
    pub fn load_global(&mut self) {
        if let Some(path) = Self::global_settings_path() {
            if let Ok(content) = std::fs::read_to_string(path) {
                if let Ok(val) = serde_json::from_str(&content) {
                    self.global_settings = val;
                }
            }
        }
    }

    /// ワークスペース設定の読み込み
    pub fn load_workspace(&mut self) {
        if let Some(path) = self.workspace_settings_path() {
            if let Ok(content) = std::fs::read_to_string(path) {
                if let Ok(val) = serde_json::from_str(&content) {
                    self.workspace_settings = val;
                }
            }
        }
    }

    /// ワークスペース設定のファイル保存
    pub fn save_workspace(&self) {
        if let Some(path) = self.workspace_settings_path() {
            if let Some(parent) = path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            if let Ok(json_str) = serde_json::to_string_pretty(&self.workspace_settings) {
                let _ = std::fs::write(path, json_str);
            }
        }
    }

    /// グローバル設定のファイル保存
    pub fn save_global(&self) {
        if let Some(path) = Self::global_settings_path() {
            if let Some(parent) = path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            if let Ok(json_str) = serde_json::to_string_pretty(&self.global_settings) {
                let _ = std::fs::write(path, json_str);
            }
        }
    }

    /// 階層マージされた設定値の取得（Workspace 優先、なければ Global）
    pub fn get(&self, key: &str) -> Option<Value> {
        let parts: Vec<&str> = key.split('.').collect();
        if let Some(val) = Self::get_from_value(&self.workspace_settings, &parts) {
            return Some(val);
        }
        Self::get_from_value(&self.global_settings, &parts)
    }

    /// User (Global) 設定の個別取得
    pub fn get_user(&self, key: &str) -> Option<Value> {
        let parts: Vec<&str> = key.split('.').collect();
        Self::get_from_value(&self.global_settings, &parts)
    }

    /// Workspace 設定の個別取得
    pub fn get_workspace(&self, key: &str) -> Option<Value> {
        let parts: Vec<&str> = key.split('.').collect();
        Self::get_from_value(&self.workspace_settings, &parts)
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

    /// 設定値の保存（デフォルトは Workspace があれば Workspace、なければ Global）
    pub fn set(&mut self, key: &str, value: Value) {
        self.set_target(SettingsTarget::Workspace, key, value);
    }

    /// 対象スコープを指定して設定値を保存
    pub fn set_target(&mut self, target: SettingsTarget, key: &str, value: Value) {
        let parts: Vec<&str> = key.split('.').collect();
        match target {
            SettingsTarget::User => {
                Self::set_to_value(&mut self.global_settings, &parts, value);
                self.save_global();
            }
            SettingsTarget::Workspace => {
                Self::set_to_value(&mut self.workspace_settings, &parts, value);
                self.save_workspace();
            }
        }
    }

    fn set_to_value(root: &mut Value, parts: &[&str], value: Value) {
        if !root.is_object() {
            *root = json!({});
        }
        let mut current = root;
        for (i, part) in parts.iter().enumerate() {
            if i == parts.len() - 1 {
                if let Some(obj) = current.as_object_mut() {
                    obj.insert(part.to_string(), value);
                    return;
                }
            } else {
                let obj = current.as_object_mut().unwrap();
                if !obj.contains_key(*part) || !obj.get(*part).unwrap().is_object() {
                    obj.insert(part.to_string(), Value::Object(serde_json::Map::new()));
                }
                current = obj.get_mut(*part).unwrap();
            }
        }
    }
}

pub struct SettingsGlobal(pub Arc<RwLock<SettingsStore>>);

impl Global for SettingsGlobal {}
