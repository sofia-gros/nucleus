/// デバッグ機能 (Debug Adapter Protocol - DAP) 統括モジュール

pub mod protocol;
pub mod client;
pub mod profiler;

use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use protocol::{StackFrame, Variable};

/// ブレークポイント情報
#[derive(Clone, Debug, PartialEq)]
pub struct BreakpointItem {
    pub file_path: String,
    pub line: usize,
    pub enabled: bool,
}

/// デバッグセッションの状態
#[derive(Clone, Debug, PartialEq)]
pub enum DebugState {
    Stopped,
    Running,
    Paused { thread_id: usize, line: usize, file_path: String },
}

/// デバッグマネージャー
pub struct DebugManager {
    pub breakpoints: HashMap<String, Vec<usize>>,
    pub state: DebugState,
    pub stack_frames: Vec<StackFrame>,
    pub variables: Vec<Variable>,
    pub watch_expressions: Vec<String>,
}

impl Default for DebugManager {
    fn default() -> Self {
        Self::new()
    }
}

impl DebugManager {
    /// 新規作成
    pub fn new() -> Self {
        Self {
            breakpoints: HashMap::new(),
            state: DebugState::Stopped,
            stack_frames: Vec::new(),
            variables: Vec::new(),
            watch_expressions: Vec::new(),
        }
    }

    /// ブレークポイントのトグル切り替え
    pub fn toggle_breakpoint(&mut self, file_path: &str, line: usize) -> bool {
        let lines = self.breakpoints.entry(file_path.to_string()).or_default();
        if let Some(pos) = lines.iter().position(|l| *l == line) {
            lines.remove(pos);
            false
        } else {
            lines.push(line);
            lines.sort();
            true
        }
    }

    /// 特定行にブレークポイントが存在するか確認
    pub fn has_breakpoint(&self, file_path: &str, line: usize) -> bool {
        self.breakpoints.get(file_path).map(|lines| lines.contains(&line)).unwrap_or(false)
    }

    /// 全ブレークポイント一覧の取得
    pub fn list_all_breakpoints(&self) -> Vec<BreakpointItem> {
        let mut list = Vec::new();
        for (path, lines) in &self.breakpoints {
            for &l in lines {
                list.push(BreakpointItem {
                    file_path: path.clone(),
                    line: l,
                    enabled: true,
                });
            }
        }
        list
    }
}

pub struct DebugGlobal(pub Arc<RwLock<DebugManager>>);

impl gpui::Global for DebugGlobal {}
