use serde_json::json;
use std::ffi::CString;

extern "C" {
    fn host_dispatch_action(ptr: *const u8, len: usize);
    fn host_log(ptr: *const u8, len: usize);
}

fn log(msg: &str) {
    unsafe {
        host_log(msg.as_ptr(), msg.len());
    }
}

fn dispatch(action: serde_json::Value) {
    let s = action.to_string();
    unsafe {
        host_dispatch_action(s.as_ptr(), s.len());
    }
}

#[no_mangle]
pub extern "C" fn plugin_init() {
    log("Git Plugin Initialized");
    
    // Register Activity Bar icon
    dispatch(json!({
        "type": "RegisterActivityBarItem",
        "args": {
            "plugin_id": "nucleus_git",
            "id": "git.activity",
            "icon": "lucide-git-branch",
            "tooltip": "Source Control",
            "command": "git.openSidebar"
        }
    }));
    
    // Register Sidebar view
    dispatch(json!({
        "type": "RegisterSidebarItem",
        "args": {
            "plugin_id": "nucleus_git",
            "id": "git.sidebar",
            "title": "SOURCE CONTROL",
            "ui_ast": {
                "type": "tree",
                "nodes": [
                    { "label": "src/main.rs (M)", "icon": "file" },
                    { "label": "src/workspace/title_bar/mod.rs (M)", "icon": "file" },
                    { "label": "src/workspace/editor_area/mod.rs (M)", "icon": "file" },
                    { "label": "plugins/git/Cargo.toml (U)", "icon": "plus" },
                    { "label": "plugins/git/src/lib.rs (U)", "icon": "plus" }
                ]
            }
        }
    }));
}

#[no_mangle]
pub extern "C" fn plugin_on_event(ptr: *const u8, len: usize) {
    let slice = unsafe { std::slice::from_raw_parts(ptr, len) };
    if let Ok(event_str) = std::str::from_utf8(slice) {
        if let Ok(event) = serde_json::from_str::<serde_json::Value>(event_str) {
            if let Some(t) = event.get("type").and_then(|t| t.as_str()) {
                if t == "CommandExecuted" {
                    if let Some(cmd) = event.get("command").and_then(|c| c.as_str()) {
                        if cmd == "git.openSidebar" {
                            dispatch(json!({
                                "type": "OpenPanel",
                                "args": {
                                    "id": "git.sidebar"
                                }
                            }));
                        }
                    }
                }
            }
        }
    }
}
