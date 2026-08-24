pub mod manifest;
pub mod runtime;
pub mod api_router;
pub mod action;
pub mod event;

use anyhow::Result;
use std::path::Path;
use std::sync::mpsc::SyncSender;
use action::PluginAction;
use gpui::{Global, Entity};

pub struct PluginManagerGlobal(pub Entity<PluginManager>);
impl Global for PluginManagerGlobal {}

pub struct PluginManager {
    runtime: runtime::PluginRuntime,
    plugins: Vec<runtime::PluginInstance>,
    action_tx: SyncSender<PluginAction>,
}

impl PluginManager {
    pub fn new(action_tx: SyncSender<PluginAction>) -> Result<Self> {
        Ok(Self {
            runtime: runtime::PluginRuntime::new()?,
            plugins: Vec::new(),
            action_tx,
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

        let wasm_file = manifest.runtime.wasm.as_deref().unwrap_or("plugin.wasm");
        let wasm_path = dir.join(wasm_file);

        if wasm_path.exists() {
            let instance = self.runtime.load_plugin(&wasm_path, manifest.clone(), self.action_tx.clone())?;
            self.plugins.push(instance);
            println!("Plugin loaded successfully: {} ({})", manifest.plugin.name, manifest.plugin.id);
        } else {
            eprintln!("WASM file not found for plugin {}: {}", manifest.plugin.id, wasm_path.display());
        }
        
        Ok(())
    }

    pub fn dispatch_event(&mut self, event: event::PluginEvent) {
        let event_json = match event {
            event::PluginEvent::FileOpened { path } => {
                format!(r#"{{"event": "file_opened", "path": "{}"}}"#, path)
            }
            _ => "{}".to_string()
        };
        
        for plugin in &mut self.plugins {
            if let Err(e) = self.runtime.dispatch_event(plugin, &event_json) {
                eprintln!("Failed to dispatch event to plugin: {}", e);
            }
        }
    }
}
