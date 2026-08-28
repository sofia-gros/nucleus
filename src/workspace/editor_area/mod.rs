/// エディタ領域およびタブ管理コンポーネント

pub mod highlighter;

use std::path::PathBuf;
use gpui::*;
use gpui_component::tab::TabBar;
use gpui_component::theme::ActiveTheme;
use crate::editor::Editor;

/// 単一のエディタタブ情報
pub struct EditorTab {
    /// ファイルパス
    pub path: String,
    /// タブの表示タイトル
    pub title: String,
    /// エディタエンティティ（初期化前は None）
    pub editor: Option<Entity<Editor>>,
}

/// エディタ領域全体の管理構造体
pub struct EditorArea {
    /// 開かれているタブのリスト
    pub tabs: Vec<EditorTab>,
    /// 初期化待ちのタブコンテンツ (path -> content)
    pub pending_contents: std::collections::HashMap<String, String>,
    /// アクティブなタブのインデックス
    pub active_tab: usize,
    /// 閉じる予約が入ったタブのインデックス
    pub pending_close_tab: Option<usize>,
}

impl EditorArea {
    /// 新しい EditorArea を作成
    pub fn new() -> Self {
        Self {
            tabs: Vec::new(),
            pending_contents: std::collections::HashMap::new(),
            active_tab: 0,
            pending_close_tab: None,
        }
    }

    /// タブを閉じる
    pub fn close_tab(&mut self, idx: usize, cx: &mut Context<Self>) {
        if idx < self.tabs.len() {
            self.tabs.remove(idx);
            if self.active_tab >= self.tabs.len() && self.active_tab > 0 {
                self.active_tab = self.tabs.len() - 1;
            }
        }
        cx.notify();
    }

    /// ファイルまたは新規バッファをタブとして開く
    pub fn open_tab(&mut self, path: String, title: String, content: String, cx: &mut Context<Self>) {
        if let Some(idx) = self.tabs.iter().position(|t| t.path == path) {
            self.active_tab = idx;
            self.pending_contents.insert(path, content);
        } else {
            self.pending_contents.insert(path.clone(), content);
            self.tabs.push(EditorTab {
                path,
                title,
                editor: None,
            });
            self.active_tab = self.tabs.len() - 1;
        }
        cx.notify();
    }

    /// ファイル拡張子から言語名を判定
    fn detect_language(path: &str) -> &'static str {
        let ext = std::path::Path::new(path)
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("");
        match ext {
            "rs" => "rust",
            "js" | "jsx" => "javascript",
            "ts" | "tsx" => "typescript",
            "html" => "html",
            "css" => "css",
            "json" => "json",
            "toml" => "toml",
            "md" => "markdown",
            "py" => "python",
            "go" => "go",
            "c" => "c",
            "cpp" | "cc" | "cxx" => "cpp",
            _ => "plaintext",
        }
    }
}

impl Render for EditorArea {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if let Some(idx) = self.pending_close_tab.take() {
            self.close_tab(idx, cx);
        }

        // 未初期化のタブを初期化
        for tab in self.tabs.iter_mut() {
            if tab.editor.is_none() {
                if let Some(content) = self.pending_contents.remove(&tab.path) {
                    let path_buf = if tab.path.is_empty() {
                        None
                    } else {
                        Some(PathBuf::from(&tab.path))
                    };
                    let lang = Self::detect_language(&tab.path);
                    let editor_entity = cx.new(|cx| {
                        Editor::new(window, &content, path_buf, lang, cx)
                    });
                    tab.editor = Some(editor_entity);
                }
            }
        }

        if self.tabs.is_empty() {
            return div()
                .size_full()
                .bg(cx.theme().background)
                .flex()
                .items_center()
                .justify_center()
                .child(
                    div().flex().flex_col().items_center().child(
                        div().text_3xl().font_weight(FontWeight::BOLD).text_color(cx.theme().accent)
                            .child("Nucleus")
                    )
                    .child(
                        div().mt_4().text_sm().text_color(cx.theme().muted_foreground)
                            .child("No files opened. Open a file from the Explorer.")
                    )
                    .child(
                        div().mt_8().flex().flex_col().items_center().text_sm().text_color(cx.theme().muted_foreground)
                            .child(div().child("Press Ctrl+B to toggle sidebar."))
                    )
                )
                .into_any_element();
        }

        let mut tab_bar = TabBar::new("editor-tabs")
            .w_full()
            .selected_index(self.active_tab)
            .on_click(cx.listener(|this, selected: &usize, _, cx| {
                this.active_tab = *selected;
                cx.notify();
            }));

        for tab in &self.tabs {
            let is_dirty = tab.editor.as_ref().map(|e| e.read(cx).is_dirty(cx)).unwrap_or(false);
            let title_text = if is_dirty {
                format!("● {}", tab.title)
            } else {
                tab.title.clone()
            };

            tab_bar = tab_bar.child(title_text);
        }

        let active_editor = if self.active_tab < self.tabs.len() {
            if let Some(editor_entity) = &self.tabs[self.active_tab].editor {
                editor_entity.clone().into_any_element()
            } else {
                div().p_4().text_sm().text_color(cx.theme().muted_foreground).child("Loading...").into_any_element()
            }
        } else {
            div().into_any_element()
        };

        div()
            .size_full()
            .flex()
            .flex_col()
            .bg(cx.theme().background)
            .child(
                div().w_full().flex().items_center().justify_between().bg(cx.theme().background)
                    .border_b_1()
                    .border_color(cx.theme().border)
                    .child(div().flex_grow(1.).overflow_hidden().child(tab_bar))
                    .child(
                        div()
                            .id("close-tab-btn")
                            .px_3()
                            .h_full()
                            .flex()
                            .items_center()
                            .text_color(cx.theme().muted_foreground)
                            .hover(|s| s.bg(gpui::rgb(0xe81123)).text_color(gpui::rgb(0xffffff)))
                            .cursor_pointer()
                            .on_mouse_down(MouseButton::Left, cx.listener(|this, _, _, cx| {
                                if !this.tabs.is_empty() {
                                    this.pending_close_tab = Some(this.active_tab);
                                    cx.notify();
                                }
                            }))
                            .child("✕")
                    )
            )
            .child(
                div().flex_grow(1.).w_full().child(active_editor)
            )
            .into_any_element()
    }
}
