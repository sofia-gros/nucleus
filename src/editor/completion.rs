/// コード補完（Completion / Suggestions）およびスニペットサジェストモジュール

use crate::workspace::command_palette::fuzzy::fuzzy_match;

/// 補完候補の種類
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CompletionItemKind {
    Function,
    Struct,
    Variable,
    Keyword,
    Snippet,
    Module,
}

impl CompletionItemKind {
    pub fn icon_label(&self) -> &'static str {
        match self {
            Self::Function => "ƒ",
            Self::Struct => "S",
            Self::Variable => "v",
            Self::Keyword => "k",
            Self::Snippet => "⊞",
            Self::Module => "m",
        }
    }
}

/// 補完候補アイテム
#[derive(Clone, Debug, PartialEq)]
pub struct CompletionItem {
    pub label: String,
    pub kind: CompletionItemKind,
    pub detail: Option<String>,
    pub insert_text: String,
}

/// 補完サジェスト状態管理
#[derive(Clone, Debug, Default)]
pub struct CompletionState {
    pub is_open: bool,
    pub selected_index: usize,
    pub query: String,
    pub items: Vec<CompletionItem>,
}

impl CompletionState {
    pub fn new() -> Self {
        Self::default()
    }

    /// 補完候補の登録とポップアップの表示
    pub fn show(&mut self, items: Vec<CompletionItem>, query: String) {
        self.items = items;
        self.query = query;
        self.selected_index = 0;
        self.is_open = !self.items.is_empty();
    }

    /// ポップアップを閉じる
    pub fn hide(&mut self) {
        self.is_open = false;
        self.items.clear();
        self.query.clear();
        self.selected_index = 0;
    }

    /// 現在のクエリに基づく絞り込み
    pub fn filtered_items(&self) -> Vec<CompletionItem> {
        if self.query.is_empty() {
            return self.items.iter().take(15).cloned().collect();
        }

        let mut scored: Vec<(i32, CompletionItem)> = Vec::new();
        for item in &self.items {
            if let Some(m) = fuzzy_match(&self.query, &item.label) {
                scored.push((m.score, item.clone()));
            }
        }

        scored.sort_by(|a, b| b.0.cmp(&a.0));
        scored.into_iter().take(15).map(|(_, item)| item).collect()
    }

    pub fn select_next(&mut self) {
        let count = self.filtered_items().len();
        if count > 0 {
            self.selected_index = (self.selected_index + 1) % count;
        }
    }

    pub fn select_prev(&mut self) {
        let count = self.filtered_items().len();
        if count > 0 {
            self.selected_index = (self.selected_index + count - 1) % count;
        }
    }

    pub fn selected_item(&self) -> Option<CompletionItem> {
        let items = self.filtered_items();
        items.get(self.selected_index).cloned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_completion_filtering() {
        let mut state = CompletionState::new();
        let items = vec![
            CompletionItem { label: "fn".into(), kind: CompletionItemKind::Keyword, detail: None, insert_text: "fn".into() },
            CompletionItem { label: "format!".into(), kind: CompletionItemKind::Function, detail: None, insert_text: "format!()".into() },
            CompletionItem { label: "struct".into(), kind: CompletionItemKind::Keyword, detail: None, insert_text: "struct".into() },
        ];

        state.show(items, "fn".into());
        let filtered = state.filtered_items();
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].label, "fn");
    }
}
