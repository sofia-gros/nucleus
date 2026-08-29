/// Debug Adapter Protocol (DAP) プロトコル型定義モジュール

use serde::{Deserialize, Serialize};

/// ブレークポイント情報
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SourceBreakpoint {
    pub line: usize,
    pub column: Option<usize>,
    pub condition: Option<String>,
}

/// スタックフレーム情報
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct StackFrame {
    pub id: usize,
    pub name: String,
    pub source: Option<Source>,
    pub line: usize,
    pub column: usize,
}

/// ソースファイル参照
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Source {
    pub name: Option<String>,
    pub path: Option<String>,
}

/// 変数情報
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Variable {
    pub name: String,
    pub value: String,
    pub type_name: Option<String>,
    pub variables_reference: usize,
}

/// スレッド情報
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ThreadInfo {
    pub id: usize,
    pub name: String,
}
