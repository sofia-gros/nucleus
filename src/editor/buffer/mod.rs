/// Ropeデータ構造に基づいた高性能テキストバッファ

pub mod point;
#[cfg(test)]
mod tests;

use std::path::{Path, PathBuf};
use ropey::{Rope, RopeSlice};
use crate::editor::buffer::point::Point;
use crate::editor::history::{EditOperation, History};

/// 改行コードの種別
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum LineEnding {
    /// UNIXスタイル (\n)
    #[default]
    LF,
    /// Windowsスタイル (\r\n)
    CRLF,
}

/// テキストバッファの実装（RopeyによるO(log n)編集とUndo/Redo履歴）
#[derive(Clone, Debug)]
pub struct TextBuffer {
    /// Ropeテキストデータ
    rope: Rope,
    /// 編集履歴（Undo/Redo）
    pub history: History,
    /// バッファ変更バージョン（編集ごとにインクリメント）
    pub version: usize,
    /// 最後に保存してから変更があったか
    pub is_dirty: bool,
    /// 関連付けられたファイルパス
    pub file_path: Option<PathBuf>,
    /// 改行コード種別
    pub line_ending: LineEnding,
}

impl TextBuffer {
    /// 新しいテキストバッファを作成
    pub fn new(initial_content: &str) -> Self {
        let rope = Rope::from_str(initial_content);
        let line_ending = if initial_content.contains("\r\n") {
            LineEnding::CRLF
        } else {
            LineEnding::LF
        };

        Self {
            rope,
            history: History::new(),
            version: 0,
            is_dirty: false,
            file_path: None,
            line_ending,
        }
    }

    /// ファイルパスを設定して作成
    pub fn from_file(path: impl Into<PathBuf>, content: &str) -> Self {
        let mut buffer = Self::new(content);
        buffer.file_path = Some(path.into());
        buffer
    }

    /// バッファ全体の文字数を取得
    pub fn len_chars(&self) -> usize {
        self.rope.len_chars()
    }

    /// バッファ全体の行数を取得（空でも最低1行）
    pub fn len_lines(&self) -> usize {
        let lines = self.rope.len_lines();
        if lines == 0 {
            1
        } else {
            lines
        }
    }

    /// 指定行のスライスを取得
    pub fn line(&self, line_idx: usize) -> Option<RopeSlice<'_>> {
        if line_idx < self.rope.len_lines() {
            Some(self.rope.line(line_idx))
        } else if line_idx == 0 && self.rope.len_chars() == 0 {
            Some(self.rope.slice(..))
        } else {
            None
        }
    }

    /// 指定行の文字列を取得（末尾の改行コードを除外）
    pub fn line_to_string(&self, line_idx: usize) -> Option<String> {
        let slice = self.line(line_idx)?;
        let mut s = slice.to_string();
        if s.ends_with("\r\n") {
            s.pop();
            s.pop();
        } else if s.ends_with('\n') || s.ends_with('\r') {
            s.pop();
        }
        Some(s)
    }

    /// 指定行の文字数を取得（末尾の改行コードを除く）
    pub fn line_len(&self, line_idx: usize) -> usize {
        if let Some(slice) = self.line(line_idx) {
            let mut len = slice.len_chars();
            let s = slice.to_string();
            if s.ends_with("\r\n") {
                len = len.saturating_sub(2);
            } else if s.ends_with('\n') || s.ends_with('\r') {
                len = len.saturating_sub(1);
            }
            len
        } else {
            0
        }
    }

    /// `Point` (row, column) を文字オフセットに変換
    pub fn point_to_offset(&self, point: Point) -> usize {
        let total_lines = self.len_lines();
        if point.row >= total_lines {
            return self.len_chars();
        }

        let line_char_idx = self.rope.line_to_char(point.row);
        let line_len = self.line_len(point.row);
        let col = point.column.min(line_len);
        (line_char_idx + col).min(self.len_chars())
    }

    /// 文字オフセットを `Point` (row, column) に変換
    pub fn offset_to_point(&self, offset: usize) -> Point {
        let offset = offset.min(self.len_chars());
        let row = self.rope.char_to_line(offset);
        let line_start = self.rope.line_to_char(row);
        let col = offset - line_start;
        Point::new(row, col)
    }

    /// 座標 `Point` をバッファの有効範囲内にクリップする
    pub fn clip_point(&self, point: Point) -> Point {
        let max_row = self.len_lines().saturating_sub(1);
        let row = point.row.min(max_row);
        let max_col = self.line_len(row);
        let column = point.column.min(max_col);
        Point::new(row, column)
    }

    /// 指定座標にテキストを挿入し、挿入後のカーソル位置を返す
    pub fn insert(&mut self, point: Point, text: &str) -> Point {
        if text.is_empty() {
            return self.clip_point(point);
        }

        let point = self.clip_point(point);
        let offset = self.point_to_offset(point);

        // Ropeへの挿入
        self.rope.insert(offset, text);
        self.version += 1;
        self.is_dirty = true;

        // 履歴の記録
        self.history.record(EditOperation::Insert {
            offset,
            point,
            text: text.to_string(),
        });

        // 挿入後の文字位置
        let new_offset = offset + text.chars().count();
        self.offset_to_point(new_offset)
    }

    /// 指定範囲のテキストを削除し、削除されたテキストを返す
    pub fn delete(&mut self, start: Point, end: Point) -> String {
        let start = self.clip_point(start);
        let end = self.clip_point(end);
        let (min, max) = if start <= end { (start, end) } else { (end, start) };

        if min == max {
            return String::new();
        }

        let start_offset = self.point_to_offset(min);
        let end_offset = self.point_to_offset(max);

        if start_offset >= end_offset {
            return String::new();
        }

        let deleted_text = self.rope.slice(start_offset..end_offset).to_string();
        self.rope.remove(start_offset..end_offset);
        self.version += 1;
        self.is_dirty = true;

        self.history.record(EditOperation::Delete {
            offset: start_offset,
            point: min,
            deleted_text: deleted_text.clone(),
        });

        deleted_text
    }

    /// 指定範囲のテキストを取得
    pub fn slice(&self, start: Point, end: Point) -> String {
        let start = self.clip_point(start);
        let end = self.clip_point(end);
        let (min, max) = if start <= end { (start, end) } else { (end, start) };

        let start_offset = self.point_to_offset(min);
        let end_offset = self.point_to_offset(max);

        if start_offset >= end_offset {
            return String::new();
        }

        self.rope.slice(start_offset..end_offset).to_string()
    }

    /// Undoを実行
    pub fn undo(&mut self) -> bool {
        if let Some(group) = self.history.pop_undo() {
            // 操作を逆順に巻き戻す
            for op in group.operations.into_iter().rev() {
                match op {
                    EditOperation::Insert { offset, text, .. } => {
                        let len = text.chars().count();
                        self.rope.remove(offset..offset + len);
                    }
                    EditOperation::Delete { offset, deleted_text, .. } => {
                        self.rope.insert(offset, &deleted_text);
                    }
                }
            }
            self.version += 1;
            self.is_dirty = true;
            true
        } else {
            false
        }
    }

    /// Redoを実行
    pub fn redo(&mut self) -> bool {
        if let Some(group) = self.history.pop_redo() {
            for op in group.operations.into_iter() {
                match op {
                    EditOperation::Insert { offset, text, .. } => {
                        self.rope.insert(offset, &text);
                    }
                    EditOperation::Delete { offset, deleted_text, .. } => {
                        let len = deleted_text.chars().count();
                        self.rope.remove(offset..offset + len);
                    }
                }
            }
            self.version += 1;
            self.is_dirty = true;
            true
        } else {
            false
        }
    }

    /// バッファ全体を文字列として取得
    pub fn to_string(&self) -> String {
        self.rope.to_string()
    }

    /// ファイルに書き出し、dirtyフラグをクリア
    pub fn save_to_file(&mut self, path: &Path) -> std::io::Result<()> {
        let content = self.to_string();
        std::fs::write(path, content)?;
        self.file_path = Some(path.to_path_buf());
        self.is_dirty = false;
        Ok(())
    }

    /// dirtyフラグをクリア
    pub fn mark_clean(&mut self) {
        self.is_dirty = false;
    }
}
