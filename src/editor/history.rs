/// テキストバッファの編集履歴（Undo/Redo）を管理するモジュール

use crate::editor::buffer::point::Point;

/// 1つの編集操作
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EditOperation {
    /// テキストの挿入 (位置, 挿入された文字列)
    Insert {
        offset: usize,
        point: Point,
        text: String,
    },
    /// テキストの削除 (位置, 削除された文字列)
    Delete {
        offset: usize,
        point: Point,
        deleted_text: String,
    },
}

/// 1回のアクション（一括置換やトランザクション）で発生した複数の編集操作
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ChangeGroup {
    /// 編集操作のリスト
    pub operations: Vec<EditOperation>,
}

impl ChangeGroup {
    /// 空のチェンジグループを作成
    pub fn new() -> Self {
        Self {
            operations: Vec::new(),
        }
    }

    /// 操作を追加
    pub fn push(&mut self, op: EditOperation) {
        self.operations.push(op);
    }

    /// 操作が空かどうか
    pub fn is_empty(&self) -> bool {
        self.operations.is_empty()
    }
}

/// バッファ全体のUndo/Redoスタック
#[derive(Clone, Debug, Default)]
pub struct History {
    /// Undoスタック
    undo_stack: Vec<ChangeGroup>,
    /// Redoスタック
    redo_stack: Vec<ChangeGroup>,
    /// 現在オープン中のトランザクション
    current_transaction: Option<ChangeGroup>,
    /// 最大履歴保持件数
    max_entries: usize,
}

impl History {
    /// 新しい履歴スタックを作成
    pub fn new() -> Self {
        Self {
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            current_transaction: None,
            max_entries: 1000,
        }
    }

    /// トランザクションを開始する
    pub fn begin_transaction(&mut self) {
        if self.current_transaction.is_none() {
            self.current_transaction = Some(ChangeGroup::new());
        }
    }

    /// トランザクションをコミットしてUndoスタックに追加する
    pub fn end_transaction(&mut self) {
        if let Some(group) = self.current_transaction.take() {
            if !group.is_empty() {
                self.push_group(group);
            }
        }
    }

    /// 1つの操作を記録（トランザクション中であればその中に追加）
    pub fn record(&mut self, op: EditOperation) {
        if let Some(group) = self.current_transaction.as_mut() {
            group.push(op);
        } else {
            let mut group = ChangeGroup::new();
            group.push(op);
            self.push_group(group);
        }
    }

    /// チェンジグループをUndoスタックに追加し、Redoスタックをクリアする
    pub fn push_group(&mut self, group: ChangeGroup) {
        self.undo_stack.push(group);
        self.redo_stack.clear();
        if self.undo_stack.len() > self.max_entries {
            self.undo_stack.remove(0);
        }
    }

    /// Undo操作を取り出す
    pub fn pop_undo(&mut self) -> Option<ChangeGroup> {
        self.end_transaction();
        let group = self.undo_stack.pop()?;
        self.redo_stack.push(group.clone());
        Some(group)
    }

    /// Redo操作を取り出す
    pub fn pop_redo(&mut self) -> Option<ChangeGroup> {
        self.end_transaction();
        let group = self.redo_stack.pop()?;
        self.undo_stack.push(group.clone());
        Some(group)
    }

    /// Undo可能かどうか
    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty() || self.current_transaction.as_ref().is_some_and(|g| !g.is_empty())
    }

    /// Redo可能かどうか
    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }

    /// 履歴をクリア
    pub fn clear(&mut self) {
        self.undo_stack.clear();
        self.redo_stack.clear();
        self.current_transaction = None;
    }
}
