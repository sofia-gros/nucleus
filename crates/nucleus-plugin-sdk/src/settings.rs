/// 設定ストア API ラッパーモジュール

use crate::invoke_host;
use serde_json::Value;

/// 設定キーの取得
pub fn get(key: &str) -> Result<Option<Value>, String> {
    let res = invoke_host("settings.get", serde_json::json!({ "key": key }))?;
    Ok(res.get("value").cloned())
}

/// 設定キーの保存
pub fn set(key: &str, value: Value) -> Result<(), String> {
    let res = invoke_host("settings.set", serde_json::json!({
        "key": key,
        "value": value
    }))?;
    if res.get("status").and_then(|s| s.as_str()) == Some("ok") || res.get("status").and_then(|s| s.as_str()) == Some("queued") {
        Ok(())
    } else {
        Err("Failed to set settings".to_string())
    }
}
