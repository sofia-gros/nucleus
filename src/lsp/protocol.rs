/// Language Server Protocol (LSP) 基本型定義モジュール

use serde::{Deserialize, Serialize};

/// JSON-RPC 2.0 リクエストメッセージ
#[derive(Debug, Serialize, Deserialize)]
pub struct JsonRpcRequest<T> {
    pub jsonrpc: String,
    pub id: u64,
    pub method: String,
    pub params: T,
}

/// JSON-RPC 2.0 通知メッセージ（IDなし）
#[derive(Debug, Serialize, Deserialize)]
pub struct JsonRpcNotification<T> {
    pub jsonrpc: String,
    pub method: String,
    pub params: T,
}

/// LSP 2次元座標位置（0-indexed）
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Position {
    pub line: u32,
    pub character: u32,
}

/// LSP 範囲
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Range {
    pub start: Position,
    pub end: Position,
}

/// LSP 診断情報（エラー、警告、情報、ヒント）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Diagnostic {
    pub range: Range,
    pub severity: Option<DiagnosticSeverity>,
    pub code: Option<serde_json::Value>,
    pub source: Option<String>,
    pub message: String,
}

/// 診断情報の重要度
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(from = "u32", into = "u32")]
pub enum DiagnosticSeverity {
    Error = 1,
    Warning = 2,
    Information = 3,
    Hint = 4,
}

impl From<u32> for DiagnosticSeverity {
    fn from(val: u32) -> Self {
        match val {
            1 => Self::Error,
            2 => Self::Warning,
            3 => Self::Information,
            _ => Self::Hint,
        }
    }
}

impl From<DiagnosticSeverity> for u32 {
    fn from(sev: DiagnosticSeverity) -> Self {
        sev as u32
    }
}

/// textDocument/publishDiagnostics のパラメータ
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublishDiagnosticsParams {
    pub uri: String,
    pub version: Option<i32>,
    pub diagnostics: Vec<Diagnostic>,
}
