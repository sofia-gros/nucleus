/// 外部プロセス実行 API ラッパーモジュール

use crate::invoke_host;

/// コマンド実行結果
#[derive(Clone, Debug)]
pub struct ProcessOutput {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
}

/// 外部コマンドの同期実行
pub fn exec(command: &str, args: &[&str], cwd: Option<&str>) -> Result<ProcessOutput, String> {
    let res = invoke_host("process.exec", serde_json::json!({
        "command": command,
        "args": args,
        "cwd": cwd
    }))?;

    let stdout = res.get("stdout").and_then(|s| s.as_str()).unwrap_or("").to_string();
    let stderr = res.get("stderr").and_then(|s| s.as_str()).unwrap_or("").to_string();
    let exit_code = res.get("exit_code").and_then(|c| c.as_i64()).unwrap_or(-1) as i32;

    Ok(ProcessOutput {
        stdout,
        stderr,
        exit_code,
    })
}
