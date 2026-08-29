/// コマンド登録 API ラッパーモジュール

use crate::invoke_host;

/// コマンドパレットへコマンドを登録
pub fn register(id: &str, title: &str) -> Result<(), String> {
    let res = invoke_host("command.register", serde_json::json!({
        "id": id,
        "title": title
    }))?;
    if res.get("status").and_then(|s| s.as_str()) == Some("ok") || res.get("status").and_then(|s| s.as_str()) == Some("queued") {
        Ok(())
    } else {
        Err("Failed to register command".to_string())
    }
}
