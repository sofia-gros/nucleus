/// ファイルシステム API ラッパーモジュール

use crate::invoke_host;

/// ファイル内容の読み込み
pub fn read_file(path: &str) -> Result<String, String> {
    let res = invoke_host("fs.read_file", serde_json::json!({ "path": path }))?;
    if let Some(content) = res.get("content").and_then(|c| c.as_str()) {
        Ok(content.to_string())
    } else if let Some(msg) = res.get("message").and_then(|m| m.as_str()) {
        Err(msg.to_string())
    } else {
        Err("Failed to read file".to_string())
    }
}

/// ファイル内容の書き込み
pub fn write_file(path: &str, content: &str) -> Result<(), String> {
    let res = invoke_host("fs.write_file", serde_json::json!({
        "path": path,
        "content": content
    }))?;
    if res.get("status").and_then(|s| s.as_str()) == Some("ok") {
        Ok(())
    } else {
        Err("Failed to write file".to_string())
    }
}
