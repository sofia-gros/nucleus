//! Nucleus 公式 Rust Analyzer LSP プラグイン

use std::mem;

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

pub fn invoke(api: &str, args: &str) -> String {
    let payload = format!(r#"{{"api": "{}", "args": {}}}"#, api, args);
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

pub fn log(msg: &str) {
    unsafe {
        host_log(msg.as_ptr(), msg.len() as i32);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn init() {
    log("Rust Analyzer LSP Plugin initializing...");

    // rust-analyzer のバージョン確認
    let check_res = invoke("process.exec", r#"{"command": "rust-analyzer", "args": ["--version"]}"#);
    let status_text = if check_res.contains("rust-analyzer") {
        "Rust Analyzer: Ready"
    } else {
        "Rust Analyzer: Idle"
    };

    let status_bar_args = format!(
        r#"{{"id": "rust_analyzer_lsp", "text": "{}", "align": "right"}}"#,
        status_text
    );
    invoke("ui.register_status_bar_item", &status_bar_args);

    log("Rust Analyzer LSP Plugin initialized.");
}

#[unsafe(no_mangle)]
pub extern "C" fn on_event(_ptr: i32, _len: i32) {
    // 将来のファイル変更イベント等をここでハンドリング
}
