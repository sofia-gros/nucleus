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
    pub action_tx: std::sync::mpsc::SyncSender<super::action::PluginAction>,
}

impl PluginRuntime {
    pub fn new() -> Result<Self> {
        let engine = Engine::default();
        Ok(Self { engine })
    }

    pub fn load_plugin(&self, wasm_path: &Path, manifest: PluginManifest, action_tx: std::sync::mpsc::SyncSender<super::action::PluginAction>) -> Result<PluginInstance> {
        let module = Module::from_file(&self.engine, wasm_path)
            .map_err(|e| anyhow::anyhow!("Failed to load wasm module: {}", e))?;

        let mut store = Store::new(
            &self.engine,
            HostState {
                plugin_id: manifest.plugin.id.clone(),
                action_tx,
            },
        );

        let mut linker = Linker::new(&self.engine);

        linker.func_wrap("env", "host_log", |mut caller: Caller<'_, HostState>, ptr: i32, len: i32| {
            let memory = caller.get_export("memory").unwrap().into_memory().unwrap();
            let data = memory.data(&caller);
            let start = ptr as usize;
            let end = start + len as usize;
            if end <= data.len() {
                let s = String::from_utf8_lossy(&data[start..end]);
                println!("[{}] {}", caller.data().plugin_id, s);
            }
        })?;

        linker.func_wrap("env", "host_invoke", |mut caller: Caller<'_, HostState>, ptr: i32, len: i32| -> i64 {
            let memory = caller.get_export("memory").unwrap().into_memory().unwrap();
            let request_str = {
                let data = memory.data(&caller);
                let start = ptr as usize;
                let end = start + len as usize;
                if end <= data.len() {
                    String::from_utf8_lossy(&data[start..end]).into_owned()
                } else {
                    String::new()
                }
            };
            
            let response = super::api_router::handle_invoke(&caller.data().plugin_id, &request_str, &caller.data().action_tx);
            let response_bytes = response.into_bytes();
            let response_len = response_bytes.len() as i32;
            
            let alloc_func = caller.get_export("alloc").unwrap().into_func().unwrap();
            let alloc_typed = alloc_func.typed::<i32, i32>(&caller).unwrap();
            let result_ptr = alloc_typed.call(&mut caller, response_len).unwrap();
            
            let memory = caller.get_export("memory").unwrap().into_memory().unwrap();
            memory.write(&mut caller, result_ptr as usize, &response_bytes).unwrap();
            
            ((result_ptr as i64) << 32) | ((response_len as u32) as i64)
        })?;

        let instance = linker.instantiate(&mut store, &module)
            .map_err(|e| anyhow::anyhow!("Failed to instantiate wasm module: {}", e))?;

        if let Ok(init_func) = instance.get_typed_func::<(), ()>(&mut store, "init") {
            if let Err(e) = init_func.call(&mut store, ()) {
                eprintln!("Plugin init failed: {}", e);
            }
        }

        Ok(PluginInstance {
            store,
            instance,
            manifest,
        })
    }

    pub fn dispatch_event(&self, instance: &mut PluginInstance, event_json: &str) -> Result<()> {
        let event_bytes = event_json.as_bytes();
        let len = event_bytes.len() as i32;
        
        let alloc_func = instance.instance.get_func(&mut instance.store, "alloc")
            .context("alloc func not found")?
            .typed::<i32, i32>(&instance.store)?;
        let ptr = alloc_func.call(&mut instance.store, len)?;
        
        let memory = instance.instance.get_memory(&mut instance.store, "memory").unwrap();
        memory.write(&mut instance.store, ptr as usize, event_bytes)?;
        
        if let Some(on_event) = instance.instance.get_func(&mut instance.store, "on_event") {
            if let Ok(func) = on_event.typed::<(i32, i32), ()>(&instance.store) {
                let _ = func.call(&mut instance.store, (ptr, len));
            }
        }
        
        if let Some(dealloc) = instance.instance.get_func(&mut instance.store, "dealloc") {
            if let Ok(func) = dealloc.typed::<(i32, i32), ()>(&instance.store) {
                let _ = func.call(&mut instance.store, (ptr, len));
            }
        }
        
        Ok(())
    }
}
