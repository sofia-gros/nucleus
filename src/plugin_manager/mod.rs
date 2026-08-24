pub mod manifest;
pub mod runtime;

use anyhow::Result;
use std::path::Path;

pub struct PluginManager {
    runtime: runtime::PluginRuntime,
    plugins: Vec<runtime::PluginInstance>,
}

impl PluginManager {
    pub fn new() -> Result<Self> {
        Ok(Self {
            runtime: runtime::PluginRuntime::new()?,
            plugins: Vec::new(),
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
            let instance = self.runtime.load_plugin(&wasm_path, manifest.clone())?;
            self.plugins.push(instance);
            println!("Plugin loaded successfully: {} ({})", manifest.plugin.name, manifest.plugin.id);
        } else {
            println!("Warning: wasm file not found for plugin {}", manifest.plugin.name);
        }

        Ok(())
    }
}
