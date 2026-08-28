/// エディタカーソルの形状や表示状態、マルチカーソル制御の管理モジュール

use crate::editor::buffer::point::Point;

/// カーソルの表示スタイル
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum CursorStyle {
    /// 通常の縦線バー（デフォルト）
    #[default]
    Bar,
    /// 矩形ブロック（Vimノーマルモード等）
    Block,
    /// 下線（アンダーライン）
    Underline,
}

/// 単一カーソル情報
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Cursor {
    /// バッファ上の位置
    pub point: Point,
    /// カーソルのスタイル
    pub style: CursorStyle,
    /// カーソル点滅の可視フラグ
    pub is_visible: bool,
}

impl Cursor {
    /// 新しいカーソルを作成
    pub fn new(point: Point) -> Self {
        Self {
            point,
            style: CursorStyle::Bar,
            is_visible: true,
        }
    }
}
