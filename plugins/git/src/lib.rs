use std::mem;
use serde_json::json;

#[link(wasm_import_module = "env")]
unsafe extern "C" {
    fn host_log(ptr: *const u8, len: i32);
    fn host_invoke(ptr: *const u8, len: i32) -> i64;
}

#[unsafe(no_mangle)]
pub extern "C" fn alloc(size: usize) -> *mut u8 {
    let mut buf = Vec::with_capacity(size);
    let ptr = buf.as_mut_ptr();
    mem::forget(buf);
    ptr
}

#[unsafe(no_mangle)]
pub extern "C" fn dealloc(ptr: *mut u8, size: usize) {
    unsafe {
        let _ = Vec::from_raw_parts(ptr, 0, size);
    }
}

pub fn invoke(api: &str, args: serde_json::Value) -> String {
    let payload = format!(r#"{{"api": "{}", "args": {}}}"#, api, args.to_string());
    let result = unsafe { host_invoke(payload.as_ptr(), payload.len() as i32) };
    
    let ptr = (result >> 32) as *mut u8;
    let len = (result & 0xFFFFFFFF) as usize;
    
    let response = unsafe {
        let slice = std::slice::from_raw_parts(ptr, len);
        String::from_utf8_lossy(slice).into_owned()
    };
    
    dealloc(ptr, len);
    response
}

fn log(msg: &str) {
    unsafe { host_log(msg.as_ptr(), msg.len() as i32); }
}

#[unsafe(no_mangle)]
pub extern "C" fn init() {
    log("Git Plugin Initialized");
    
    invoke("ui.register_activity_bar_item", json!({
        "plugin_id": "nucleus_git",
        "id": "git.activity",
        "icon": "lucide-git-branch",
        "tooltip": "Source Control",
        "command": "git.openSidebar"
    }));
    
    invoke("process.spawn", json!({
        "id": "git_status_init",
        "command": "git",
        "args": ["status", "--porcelain"]
    }));
    
    invoke("process.spawn", json!({
        "id": "git_branch",
        "command": "git",
        "args": ["rev-parse", "--abbrev-ref", "HEAD"]
    }));
}

fn handle_git_status(stdout: &str) {
    let mut nodes = Vec::new();
    let mut stats = std::collections::HashMap::new();
    
    for line in stdout.lines() {
        if line.len() < 4 { continue; }
        let status = &line[0..2];
        let file = &line[3..];
        
        let icon = match status {
            "?? " => "plus",
            " M " | "M " | "MM" => "edit-2",
            " D " | "D " => "minus",
            _ => "file",
        };
        
        nodes.push(json!({
            "label": format!("{} ({})", file, status.trim()),
            "icon": icon
        }));
        
        stats.insert(file.to_string(), status.trim().to_string());
    }
    
    invoke("ui.register_sidebar_item", json!({
        "plugin_id": "nucleus_git",
        "id": "git.sidebar",
        "title": "SOURCE CONTROL",
        "ui_ast": {
            "type": "source_control",
            "nodes": nodes
        }
    }));
    
    invoke("settings.set", json!({
        "key": "git.status",
        "value": stats
    }));
}

fn handle_git_branch(stdout: &str) {
    let branch = stdout.trim();
    invoke("settings.set", json!({
        "key": "git.branch",
        "value": branch
    }));
}

#[unsafe(no_mangle)]
pub extern "C" fn on_event(ptr: i32, len: i32) {
    let slice = unsafe { std::slice::from_raw_parts(ptr as *const u8, len as usize) };
    if let Ok(event_str) = std::str::from_utf8(slice) {
        if let Ok(event) = serde_json::from_str::<serde_json::Value>(event_str) {
            if let Some(t) = event.get("type").and_then(|t| t.as_str()) {
                if t == "CommandExecuted" {
                    if let Some(cmd) = event.get("command").and_then(|c| c.as_str()) {
                        if cmd == "git.openSidebar" {
                            invoke("panel.open", json!({
                                "id": "git.sidebar"
                            }));
                        }
                    }
                } else if t == "ProcessOutput" {
                    let id = event.get("id").and_then(|i| i.as_str()).unwrap_or("");
                    let stdout = event.get("stdout").and_then(|s| s.as_str()).unwrap_or("");
                    
                    if id == "git_status_init" || id == "git_status_update" {
                        handle_git_status(stdout);
                    } else if id == "git_branch" {
                        handle_git_branch(stdout);
                    }
                } else if t == "file_saved" || t == "file_opened" {
                    invoke("process.spawn", json!({
                        "id": "git_status_update",
                        "command": "git",
                        "args": ["status", "--porcelain"]
                    }));
                }
            }
        }
    }
}
