/// ホバー情報（Hover Tooltip）および型シグネチャ表示モジュール

use crate::lsp::protocol::Range;

/// ホバーポップアップに表示する詳細情報
#[derive(Clone, Debug, PartialEq)]
pub struct HoverInfo {
    pub range: Option<Range>,
    pub contents: String,
}

/// ホバー状態管理
#[derive(Clone, Debug, Default)]
pub struct HoverState {
    pub is_visible: bool,
    pub x: f32,
    pub y: f32,
    pub info: Option<HoverInfo>,
}

impl HoverState {
    pub fn new() -> Self {
        Self::default()
    }

    /// ホバー情報を表示
    pub fn show(&mut self, x: f32, y: f32, info: HoverInfo) {
        self.x = x;
        self.y = y;
        self.info = Some(info);
        self.is_visible = true;
    }

    /// ホバー情報を非表示
    pub fn hide(&mut self) {
        self.is_visible = false;
        self.info = None;
    }
}
