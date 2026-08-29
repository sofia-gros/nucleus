/// Debug Adapter Protocol (DAP) クライアント実装モジュール

use anyhow::{Context, Result};
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;

/// DAP クライアント
pub struct DapClient {
    #[allow(dead_code)]
    child: Arc<Mutex<Child>>,
    stdin: Arc<Mutex<ChildStdin>>,
    seq: AtomicU64,
}

impl DapClient {
    /// デバッグアダプタープロセスの起動と初期化
    pub fn spawn(program: &str, args: &[&str]) -> Result<Self> {
        let mut child = Command::new(program)
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .with_context(|| format!("Failed to spawn DAP process: {}", program))?;

        let stdin = child.stdin.take().context("Failed to open DAP stdin")?;
        let stdout = child.stdout.take().context("Failed to open DAP stdout")?;

        let stdin = Arc::new(Mutex::new(stdin));
        let child = Arc::new(Mutex::new(child));

        // stdout バックグラウンドリーダースレッド
        thread::spawn(move || {
            let mut reader = BufReader::new(stdout);
            let mut line = String::new();

            while let Ok(n) = reader.read_line(&mut line) {
                if n == 0 {
                    break;
                }
                if line.starts_with("Content-Length:") {
                    let parts: Vec<&str> = line.trim().split(':').collect();
                    if parts.len() == 2 {
                        if let Ok(len) = parts[1].trim().parse::<usize>() {
                            let mut empty_line = String::new();
                            let _ = reader.read_line(&mut empty_line);

                            let mut body_buf = vec![0u8; len];
                            if std::io::Read::read_exact(&mut reader, &mut body_buf).is_ok() {
                                if let Ok(val) = serde_json::from_slice::<serde_json::Value>(&body_buf) {
                                    let _ = val;
                                }
                            }
                        }
                    }
                }
                line.clear();
            }
        });

        Ok(Self {
            child,
            stdin,
            seq: AtomicU64::new(1),
        })
    }

    /// リクエスト送信 (JSON-RPC)
    pub fn send_request(&self, command: &str, arguments: Option<serde_json::Value>) -> Result<u64> {
        let seq = self.seq.fetch_add(1, Ordering::SeqCst);
        let mut payload = serde_json::json!({
            "seq": seq,
            "type": "request",
            "command": command
        });

        if let Some(args) = arguments {
            payload["arguments"] = args;
        }

        self.send_payload(&payload)?;
        Ok(seq)
    }

    /// 初期化リクエスト (initialize)
    pub fn initialize(&self, client_id: &str) -> Result<u64> {
        self.send_request("initialize", Some(serde_json::json!({
            "clientID": client_id,
            "clientName": "Nucleus IDE",
            "adapterID": "lldb-dap",
            "linesStartAt1": true,
            "columnsStartAt1": true,
            "supportsVariableType": true
        })))
    }

    /// プログラム起動リクエスト (launch)
    pub fn launch(&self, program_path: &str, cwd: Option<&str>) -> Result<u64> {
        self.send_request("launch", Some(serde_json::json!({
            "program": program_path,
            "cwd": cwd,
            "stopOnEntry": false
        })))
    }

    /// ブレークポイント設定 (setBreakpoints)
    pub fn set_breakpoints(&self, file_path: &str, lines: &[usize]) -> Result<u64> {
        let bps: Vec<serde_json::Value> = lines.iter().map(|l| serde_json::json!({ "line": l })).collect();
        self.send_request("setBreakpoints", Some(serde_json::json!({
            "source": { "path": file_path },
            "breakpoints": bps
        })))
    }

    /// 続行 (continue)
    pub fn continue_exec(&self, thread_id: usize) -> Result<u64> {
        self.send_request("continue", Some(serde_json::json!({ "threadId": thread_id })))
    }

    /// ステップオーバー (next)
    pub fn next(&self, thread_id: usize) -> Result<u64> {
        self.send_request("next", Some(serde_json::json!({ "threadId": thread_id })))
    }

    /// ステップイン (stepIn)
    pub fn step_in(&self, thread_id: usize) -> Result<u64> {
        self.send_request("stepIn", Some(serde_json::json!({ "threadId": thread_id })))
    }

    /// ステップアウト (stepOut)
    pub fn step_out(&self, thread_id: usize) -> Result<u64> {
        self.send_request("stepOut", Some(serde_json::json!({ "threadId": thread_id })))
    }

    /// 切断・停止 (disconnect)
    pub fn disconnect(&self) -> Result<u64> {
        self.send_request("disconnect", Some(serde_json::json!({ "restart": false })))
    }

    fn send_payload(&self, value: &serde_json::Value) -> Result<()> {
        let body = serde_json::to_string(value)?;
        let header = format!("Content-Length: {}\r\n\r\n", body.len());

        let mut stdin = self.stdin.lock().unwrap();
        stdin.write_all(header.as_bytes())?;
        stdin.write_all(body.as_bytes())?;
        stdin.flush()?;
        Ok(())
    }
}
