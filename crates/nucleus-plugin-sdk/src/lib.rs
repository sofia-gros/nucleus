/// Nucleus IDE WASM プラグイン開発用公式 Rust SDK

pub mod ui;
pub mod fs;
pub mod process;
pub mod settings;
pub mod commands;

pub use serde_json::Value;

// Host ABI 低レベルインポート
#[allow(dead_code)]
#[link(wasm_import_module = "nucleus")]
extern "C" {
    fn host_invoke(ptr: *const u8, len: usize) -> u64;
}

/// ホスト環境へ JSON メッセージを送信し、結果を受信する汎用関数
pub fn invoke_host(method: &str, params: serde_json::Value) -> Result<serde_json::Value, String> {
    let req = serde_json::json!({
        "method": method,
        "params": params,
    });

    let json_bytes = serde_json::to_vec(&req).map_err(|e| e.to_string())?;

    #[cfg(target_arch = "wasm32")]
    unsafe {
        let res_packed = host_invoke(json_bytes.as_ptr(), json_bytes.len());
        let res_ptr = (res_packed >> 32) as *const u8;
        let res_len = (res_packed & 0xFFFFFFFF) as usize;
        let res_slice = std::slice::from_raw_parts(res_ptr, res_len);
        let val: serde_json::Value = serde_json::from_slice(res_slice).map_err(|e| e.to_string())?;
        Ok(val)
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        // 非 WASM テスト環境用モック
        let _ = json_bytes;
        Ok(serde_json::json!({ "status": "ok" }))
    }
}

/// WASM プラグインのエントリポイントを定義するマクロ
#[macro_export]
macro_rules! export_plugin {
    ($init_fn:expr, $event_fn:expr) => {
        #[no_mangle]
        pub extern "C" fn init() {
            $init_fn();
        }

        #[no_mangle]
        pub extern "C" fn on_event(ptr: *const u8, len: usize) {
            let slice = unsafe { std::slice::from_raw_parts(ptr, len) };
            if let Ok(event_json) = serde_json::from_slice::<serde_json::Value>(slice) {
                $event_fn(event_json);
            }
        }
    };
}
