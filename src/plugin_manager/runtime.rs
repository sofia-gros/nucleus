use anyhow::{Context, Result};
use wasmtime::*;
use super::manifest::PluginManifest;
use std::path::Path;

pub struct PluginRuntime {
    engine: Engine,
}

pub struct PluginInstance {
    store: Store<HostState>,
    instance: Instance,
    pub manifest: PluginManifest,
}

pub struct HostState {
    pub plugin_id: String,
}

impl PluginRuntime {
    pub fn new() -> Result<Self> {
        let engine = Engine::default();
        Ok(Self { engine })
    }

    pub fn load_plugin(&self, wasm_path: &Path, manifest: PluginManifest) -> Result<PluginInstance> {
        let module = Module::from_file(&self.engine, wasm_path)
            .context("Failed to load wasm module")?;

        let mut store = Store::new(
            &self.engine,
            HostState {
                plugin_id: manifest.plugin.id.clone(),
            },
        );

        let mut linker = Linker::new(&self.engine);

        // Phase 1: Minimal Host API -> host_log
        linker.func_wrap("env", "host_log", |caller: Caller<'_, HostState>, _ptr: i32, _len: i32| {
            // Note: Reading strings from wasm memory is complex without wit-bindgen.
            // For now, we just print that the function was called.
            println!("[{}] host_log called by plugin", caller.data().plugin_id);
        })?;

        let instance = linker.instantiate(&mut store, &module)
            .context("Failed to instantiate wasm module")?;

        Ok(PluginInstance {
            store,
            instance,
            manifest,
        })
    }
}
