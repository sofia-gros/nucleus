/// JSON-RPC over stdio LSP クライアントモジュール

use anyhow::{Context, Result};
use std::io::{BufRead, BufReader, Read, Write};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;

pub struct LspClient {
    _process: Child,
    stdin: Arc<Mutex<ChildStdin>>,
    next_id: AtomicU64,
}

impl LspClient {
    /// Language Server プロセスの起動とクライアントの初期化
    pub fn spawn<F>(command: &str, args: &[&str], on_notification: F) -> Result<Self>
    where
        F: Fn(String, serde_json::Value) + Send + Sync + 'static,
    {
        let mut child = Command::new(command)
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .context(format!("Failed to spawn LSP server: {}", command))?;

        let stdin = child.stdin.take().context("Failed to open child stdin")?;
        let stdout = child.stdout.take().context("Failed to open child stdout")?;

        let stdin_arc = Arc::new(Mutex::new(stdin));

        // バックグラウンドで LSP 出力を継続読み取り
        thread::spawn(move || {
            let mut reader = BufReader::new(stdout);
            loop {
                let mut line = String::new();
                // 1. ヘッダーの読み取り (Content-Length: xxx)
                let mut content_length: Option<usize> = None;
                loop {
                    line.clear();
                    if reader.read_line(&mut line).unwrap_or(0) == 0 {
                        return; // EOF
                    }
                    let trimmed = line.trim();
                    if trimmed.is_empty() {
                        break; // ヘッダー終了
                    }
                    if let Some(len_str) = trimmed.strip_prefix("Content-Length:") {
                        if let Ok(len) = len_str.trim().parse::<usize>() {
                            content_length = Some(len);
                        }
                    }
                }

                // 2. 本文 JSON の読み取り
                if let Some(len) = content_length {
                    let mut body_buf = vec![0u8; len];
                    if reader.read_exact(&mut body_buf).is_ok() {
                        if let Ok(val) = serde_json::from_slice::<serde_json::Value>(&body_buf) {
                            if let Some(method) = val.get("method").and_then(|m| m.as_str()) {
                                let params = val.get("params").cloned().unwrap_or(serde_json::Value::Null);
                                on_notification(method.to_string(), params);
                            }
                        }
                    }
                }
            }
        });

        Ok(Self {
            _process: child,
            stdin: stdin_arc,
            next_id: AtomicU64::new(1),
        })
    }

    /// JSON-RPC 通知 (Notification) の送信
    pub fn send_notification(&self, method: &str, params: serde_json::Value) -> Result<()> {
        let msg = serde_json::json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params,
        });
        self.send_payload(&msg)
    }

    /// JSON-RPC リクエスト (Request) の送信
    pub fn send_request(&self, method: &str, params: serde_json::Value) -> Result<u64> {
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        let msg = serde_json::json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params,
        });
        self.send_payload(&msg)?;
        Ok(id)
    }

    /// ホバー情報の取得リクエスト (textDocument/hover)
    pub fn request_hover(&self, uri: &str, line: u32, character: u32) -> Result<u64> {
        self.send_request("textDocument/hover", serde_json::json!({
            "textDocument": { "uri": uri },
            "position": { "line": line, "character": character }
        }))
    }

    /// 定義位置の取得リクエスト (textDocument/definition)
    pub fn request_definition(&self, uri: &str, line: u32, character: u32) -> Result<u64> {
        self.send_request("textDocument/definition", serde_json::json!({
            "textDocument": { "uri": uri },
            "position": { "line": line, "character": character }
        }))
    }

    /// コード補完候補の取得リクエスト (textDocument/completion)
    pub fn request_completion(&self, uri: &str, line: u32, character: u32) -> Result<u64> {
        self.send_request("textDocument/completion", serde_json::json!({
            "textDocument": { "uri": uri },
            "position": { "line": line, "character": character }
        }))
    }

    /// 参照箇所一覧の取得リクエスト (textDocument/references)
    pub fn request_references(&self, uri: &str, line: u32, character: u32) -> Result<u64> {
        self.send_request("textDocument/references", serde_json::json!({
            "textDocument": { "uri": uri },
            "position": { "line": line, "character": character },
            "context": { "includeDeclaration": true }
        }))
    }

    /// シンボルの一括リネームリクエスト (textDocument/rename)
    pub fn request_rename(&self, uri: &str, line: u32, character: u32, new_name: &str) -> Result<u64> {
        self.send_request("textDocument/rename", serde_json::json!({
            "textDocument": { "uri": uri },
            "position": { "line": line, "character": character },
            "newName": new_name
        }))
    }

    /// クイックフィックス・コードアクションの取得リクエスト (textDocument/codeAction)
    pub fn request_code_actions(&self, uri: &str, start_line: u32, start_col: u32, end_line: u32, end_col: u32) -> Result<u64> {
        self.send_request("textDocument/codeAction", serde_json::json!({
            "textDocument": { "uri": uri },
            "range": {
                "start": { "line": start_line, "character": start_col },
                "end": { "line": end_line, "character": end_col }
            },
            "context": {
                "diagnostics": []
            }
        }))
    }

    /// ドキュメント自動整形の実行リクエスト (textDocument/formatting)
    pub fn request_formatting(&self, uri: &str, tab_size: u32, insert_spaces: bool) -> Result<u64> {
        self.send_request("textDocument/formatting", serde_json::json!({
            "textDocument": { "uri": uri },
            "options": {
                "tabSize": tab_size,
                "insertSpaces": insert_spaces
            }
        }))
    }

    /// インレイヒントの取得リクエスト (textDocument/inlayHint)
    pub fn request_inlay_hints(&self, uri: &str, start_line: u32, end_line: u32) -> Result<u64> {
        self.send_request("textDocument/inlayHint", serde_json::json!({
            "textDocument": { "uri": uri },
            "range": {
                "start": { "line": start_line, "character": 0 },
                "end": { "line": end_line, "character": 0 }
            }
        }))
    }

    /// シグネチャヘルプの取得リクエスト (textDocument/signatureHelp)
    pub fn request_signature_help(&self, uri: &str, line: u32, character: u32) -> Result<u64> {
        self.send_request("textDocument/signatureHelp", serde_json::json!({
            "textDocument": { "uri": uri },
            "position": { "line": line, "character": character }
        }))
    }

    /// ドキュメント内シンボル一覧の取得リクエスト (textDocument/documentSymbol)
    pub fn request_document_symbols(&self, uri: &str) -> Result<u64> {
        self.send_request("textDocument/documentSymbol", serde_json::json!({
            "textDocument": { "uri": uri }
        }))
    }

    /// LSP 規格 (Content-Length: ...\r\n\r\n{...}) に基づくペイロード送信
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
