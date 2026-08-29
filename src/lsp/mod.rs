/// Language Server Protocol (LSP) 統合管理モジュール

pub mod protocol;
pub mod client;
pub mod inlay_hints;

use client::LspClient;
use protocol::Diagnostic;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// ワークスペース内の LSP クライアントおよび診断情報の一元管理
pub struct LspStore {
    clients: HashMap<String, Arc<LspClient>>,
    pub diagnostics: Arc<RwLock<HashMap<String, Vec<Diagnostic>>>>,
}

impl LspStore {
    pub fn new() -> Self {
        Self {
            clients: HashMap::new(),
            diagnostics: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// 特定言語用の LSP サーバー起動
    pub fn start_server(&mut self, language: &str, command: &str, args: &[&str]) -> anyhow::Result<()> {
        let diag_store = self.diagnostics.clone();

        let client = LspClient::spawn(command, args, move |method, params| {
            if method == "textDocument/publishDiagnostics" {
                if let Ok(diag_params) = serde_json::from_value::<protocol::PublishDiagnosticsParams>(params) {
                    if let Ok(mut store) = diag_store.write() {
                        store.insert(diag_params.uri, diag_params.diagnostics);
                    }
                }
            }
        })?;

        // LSP 初期化リクエスト (initialize)
        let _ = client.send_request("initialize", serde_json::json!({
            "processId": std::process::id(),
            "capabilities": {
                "textDocument": {
                    "publishDiagnostics": { "relatedInformation": true }
                }
            }
        }));

        self.clients.insert(language.to_string(), Arc::new(client));
        Ok(())
    }

    /// ファイルの診断情報を取得
    pub fn get_diagnostics(&self, uri: &str) -> Vec<Diagnostic> {
        if let Ok(store) = self.diagnostics.read() {
            store.get(uri).cloned().unwrap_or_default()
        } else {
            Vec::new()
        }
    }
}
