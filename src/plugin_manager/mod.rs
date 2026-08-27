pub mod manifest;
pub mod runtime;
pub mod api_router;
pub mod action;
pub mod event;
pub mod ui;

use anyhow::Result;
use std::path::Path;
use std::sync::mpsc::SyncSender;
use std::sync::{Arc, RwLock};
use std::collections::HashMap;
use action::PluginAction;
use gpui::{Global, Entity};
use crate::settings::SettingsStore;

pub struct PluginManagerGlobal(pub Entity<PluginManager>);
impl Global for PluginManagerGlobal {}

pub struct PluginManager {
    runtime: runtime::PluginRuntime,
    plugins: Vec<runtime::PluginInstance>,
    action_tx: SyncSender<PluginAction>,
    settings: Arc<RwLock<SettingsStore>>,
    pub commands: HashMap<String, String>, // command_name -> plugin_id
    pub ui_registry: ui::UIExtensionRegistry,
}

impl PluginManager {
    pub fn new(action_tx: SyncSender<PluginAction>, settings: Arc<RwLock<SettingsStore>>) -> Result<Self> {
        Ok(Self {
            runtime: runtime::PluginRuntime::new()?,
            plugins: Vec::new(),
            action_tx,
            settings,
            commands: HashMap::new(),
            ui_registry: ui::UIExtensionRegistry::default(),
        })
    }

    pub fn discover_and_load(&mut self, plugins_dir: &Path) -> Result<()> {
        if !plugins_dir.exists() {
            return Ok(()); // directory doesn't exist yet, which is fine
        }

        for entry in std::fs::read_dir(plugins_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                if let Err(e) = self.try_load_plugin_dir(&path) {
                    eprintln!("Failed to load plugin from {}: {}", path.display(), e);
                }
            }
        }
        Ok(())
    }

    fn try_load_plugin_dir(&mut self, dir: &Path) -> Result<()> {
        let manifest_path = dir.join("plugin.toml");
        if !manifest_path.exists() {
            return Ok(());
        }

        let content = std::fs::read_to_string(&manifest_path)?;
        let manifest = manifest::PluginManifest::parse(&content)?;

        if let Some(wasm_file) = &manifest.runtime.wasm {
            let wasm_path = dir.join(wasm_file);
            if wasm_path.exists() {
                let instance = self.runtime.load_plugin(&wasm_path, manifest.clone(), self.action_tx.clone(), self.settings.clone())?;
                self.plugins.push(instance);
                println!("Plugin loaded successfully: {} ({})", manifest.plugin.name, manifest.plugin.id);
            } else {
                eprintln!("WASM file not found for plugin {}: {}", manifest.plugin.id, wasm_path.display());
            }
        } else {
            // Optional: fallback to plugin.wasm if it exists
            let wasm_path = dir.join("plugin.wasm");
            if wasm_path.exists() {
                if let Ok(instance) = self.runtime.load_plugin(&wasm_path, manifest.clone(), self.action_tx.clone(), self.settings.clone()) {
                    self.plugins.push(instance);
                    println!("Plugin loaded successfully: {} ({})", manifest.plugin.name, manifest.plugin.id);
                }
            }
        }

        let syntaxes_dir = dir.join("syntaxes");
        if syntaxes_dir.exists() && syntaxes_dir.is_dir() {
            if let Err(e) = crate::workspace::editor_area::highlighter::load_syntaxes_from_folder(&syntaxes_dir) {
                eprintln!("Failed to load syntaxes for plugin {}: {}", manifest.plugin.id, e);
            } else {
                println!("Loaded syntaxes for plugin: {}", manifest.plugin.id);
            }
        }
        
        Ok(())
    }

    pub fn dispatch_action(&self, action: PluginAction) {
        let _ = self.action_tx.send(action);
    }

    pub fn dispatch_event(&mut self, event: event::PluginEvent) {
        let event_json = match event {
            event::PluginEvent::FileOpened { path } => {
                format!(r#"{{"event": "file_opened", "path": "{}"}}"#, path)
            }
            event::PluginEvent::ProcessOutput { id, stdout } => {
                let stdout_escaped = stdout.replace("\"", "\\\"").replace("\n", "\\n");
                format!(r#"{{"event": "process_output", "id": "{}", "stdout": "{}"}}"#, id, stdout_escaped)
            }
            event::PluginEvent::ProcessExited { id, code } => {
                format!(r#"{{"event": "process_exited", "id": "{}", "code": {}}}"#, id, code)
            }
            event::PluginEvent::FileSystemReadComplete { req_id, content, error } => {
                let content_json = match content {
                    Some(c) => format!(r#""{}""#, c.replace("\"", "\\\"").replace("\n", "\\n")),
                    None => "null".to_string()
                };
                let error_json = match error {
                    Some(e) => format!(r#""{}""#, e.replace("\"", "\\\"").replace("\n", "\\n")),
                    None => "null".to_string()
                };
                format!(r#"{{"event": "fs_read_complete", "req_id": "{}", "content": {}, "error": {}}}"#, req_id, content_json, error_json)
            }
            event::PluginEvent::FileSystemWriteComplete { req_id, error } => {
                let error_json = match error {
                    Some(e) => format!(r#""{}""#, e.replace("\"", "\\\"").replace("\n", "\\n")),
                    None => "null".to_string()
                };
                format!(r#"{{"event": "fs_write_complete", "req_id": "{}", "error": {}}}"#, req_id, error_json)
            }
            event::PluginEvent::CommandExecuted { command } => {
                let command_escaped = command.replace("\"", "\\\"");
                format!(r#"{{"event": "command_execute", "command": "{}"}}"#, command_escaped)
            }
            _ => "{}".to_string()
        };
        
        for plugin in &mut self.plugins {
            if let Err(e) = self.runtime.dispatch_event(plugin, &event_json) {
                eprintln!("Failed to dispatch event to plugin: {}", e);
            }
        }
    }

    pub fn register_command(&mut self, plugin_id: String, command: String) {
        self.commands.insert(command.clone(), plugin_id.clone());
        println!("Registered command '{}' for plugin '{}'", command, plugin_id);
    }
}
