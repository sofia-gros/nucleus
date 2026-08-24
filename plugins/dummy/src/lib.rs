use std::mem;

#[link(wasm_import_module = "env")]
extern "C" {
    fn host_log(ptr: *const u8, len: i32);
    fn host_invoke(ptr: *const u8, len: i32) -> i64;
}

#[no_mangle]
pub extern "C" fn alloc(size: usize) -> *mut u8 {
    let mut buf = Vec::with_capacity(size);
    let ptr = buf.as_mut_ptr();
    mem::forget(buf);
    ptr
}

#[no_mangle]
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
    
    // Deallocate the response buffer that was allocated by the host via our `alloc`
    dealloc(ptr, len);
    
    response
}

#[no_mangle]
pub extern "C" fn on_event(ptr: i32, len: i32) {
    let slice = unsafe { std::slice::from_raw_parts(ptr as *const u8, len as usize) };
    let json_str = String::from_utf8_lossy(slice);
    
    let msg = format!("WASM received event: {}", json_str);
    unsafe {
        host_log(msg.as_ptr(), msg.len() as i32);
    }
}

#[no_mangle]
pub extern "C" fn init() {
    let msg = "WASM dummy plugin initialized.";
    unsafe {
        host_log(msg.as_ptr(), msg.len() as i32);
    }
    
    let res = invoke("editor.open_tab", r#"{"title": "WASM Tab", "content": "Hello from plugin!"}"#);
    let msg = format!("open_tab response: {}", res);
    unsafe {
        host_log(msg.as_ptr(), msg.len() as i32);
    }

    let res = invoke("panel.open", r#"{"id": "plugin_side_panel"}"#);
    let msg = format!("open_panel response: {}", res);
    unsafe {
        host_log(msg.as_ptr(), msg.len() as i32);
    }

    // test large actions
    for i in 0..5 {
        invoke("editor.open_tab", &format!(r#"{{"title": "Bulk Tab {}", "content": ""}}"#, i));
    }
}
