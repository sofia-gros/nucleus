/// エディタ領域およびタブ管理コンポーネント

pub mod highlighter;

use std::path::PathBuf;
use gpui::*;
use gpui_component::{Icon, IconName};
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

/// タブの右クリックコンテキストメニュー情報
#[derive(Clone, Debug)]
pub struct TabContextMenu {
    /// 対象のタブインデックス
    pub tab_index: usize,
    /// クリックされた画面座標
    pub position: Point<Pixels>,
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
    /// 表示中のコンテキストメニュー
    pub context_menu: Option<TabContextMenu>,
}

impl EditorArea {
    /// 新しい EditorArea を作成
    pub fn new() -> Self {
        Self {
            tabs: Vec::new(),
            pending_contents: std::collections::HashMap::new(),
            active_tab: 0,
            pending_close_tab: None,
            context_menu: None,
        }
    }

    /// 指定したインデックスのタブを閉じる
    pub fn close_tab(&mut self, idx: usize, cx: &mut Context<Self>) {
        if idx < self.tabs.len() {
            self.tabs.remove(idx);
            if self.active_tab >= self.tabs.len() && self.active_tab > 0 {
                self.active_tab = self.tabs.len() - 1;
            }
        }
        self.context_menu = None;
        cx.notify();
    }

    /// 指定したタブ以外の他のすべてのタブを閉じる (Close Others)
    pub fn close_others(&mut self, target_idx: usize, cx: &mut Context<Self>) {
        if target_idx < self.tabs.len() {
            let target_tab = self.tabs.remove(target_idx);
            self.tabs.clear();
            self.tabs.push(target_tab);
            self.active_tab = 0;
        }
        self.context_menu = None;
        cx.notify();
    }

    /// 指定したタブより右側のすべてのタブを閉じる (Close to the Right)
    pub fn close_to_right(&mut self, target_idx: usize, cx: &mut Context<Self>) {
        if target_idx < self.tabs.len() {
            self.tabs.truncate(target_idx + 1);
            if self.active_tab > target_idx {
                self.active_tab = target_idx;
            }
        }
        self.context_menu = None;
        cx.notify();
    }

    /// 保存済み（未変更）のタブをすべて閉じる (Close Saved)
    pub fn close_saved(&mut self, cx: &mut Context<Self>) {
        let mut idx = 0;
        while idx < self.tabs.len() {
            let is_dirty = self.tabs[idx]
                .editor
                .as_ref()
                .map(|e| e.read(cx).is_dirty(cx))
                .unwrap_or(false);

            if !is_dirty {
                self.tabs.remove(idx);
                if self.active_tab >= self.tabs.len() && self.active_tab > 0 {
                    self.active_tab = self.tabs.len() - 1;
                }
            } else {
                idx += 1;
            }
        }
        self.context_menu = None;
        cx.notify();
    }

    /// すべてのタブを閉じる (Close All)
    pub fn close_all(&mut self, cx: &mut Context<Self>) {
        self.tabs.clear();
        self.active_tab = 0;
        self.context_menu = None;
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

    /// ファイルパスまたは拡張子から言語名を判定
    fn detect_language(path: &str) -> &'static str {
        let p = std::path::Path::new(path);
        let file_name = p.file_name().and_then(|s| s.to_str()).unwrap_or("");
        
        // ファイル名による判定 (Cargo.lock, Pipfile など)
        if file_name == "Cargo.lock" || file_name == "Gopkg.lock" || file_name == "Pipfile" || file_name == "pdm.lock" || file_name == "poetry.lock" || file_name == "uv.lock" || file_name == "mise.lock" {
            return "toml";
        }
        if file_name == "Dockerfile" || file_name.starts_with("Dockerfile.") {
            return "dockerfile";
        }
        if file_name == ".gitignore" || file_name == ".gitmodules" || file_name == ".gitattributes" {
            return "ini";
        }

        let ext = p.extension().and_then(|s| s.to_str()).unwrap_or("");
        match ext {
            "rs" => "rust",
            "js" | "jsx" | "mjs" | "cjs" => "javascript",
            "ts" | "tsx" | "mts" | "cts" => "typescript",
            "html" | "htm" => "html",
            "css" | "scss" | "sass" | "less" => "css",
            "json" | "jsonc" => "json",
            "toml" | "tml" | "lock" => "toml",
            "md" | "markdown" => "markdown",
            "py" | "pyw" | "pyi" => "python",
            "go" => "go",
            "c" | "h" => "c",
            "cpp" | "cc" | "cxx" | "hpp" | "hxx" => "cpp",
            "yaml" | "yml" => "yaml",
            "sh" | "bash" | "zsh" => "shell",
            "sql" => "sql",
            "xml" | "svg" => "xml",
            _ => "plaintext",
        }
    }

    /// コンテキストメニュー（右クリックメニュー）の描画
    fn render_context_menu(&self, cx: &mut Context<Self>) -> Option<impl IntoElement> {
        let menu = self.context_menu.as_ref()?;
        let target_idx = menu.tab_index;
        let pos = menu.position;
        let theme = cx.theme().clone();

        Some(
            gpui::deferred(
                div()
                    .absolute()
                    .occlude()
                    .top(pos.y)
                    .left(pos.x)
                    .w(gpui::px(180.0))
                    .bg(theme.background)
                    .border_1()
                    .border_color(theme.border)
                    .rounded_md()
                    .shadow_lg()
                    .p_1()
                    .flex()
                    .flex_col()
                    .gap_0p5()
                    .on_mouse_down(MouseButton::Left, cx.listener(|_, _, _, cx| {
                        cx.stop_propagation();
                    }))
                    .on_mouse_down(MouseButton::Right, cx.listener(|_, _, _, cx| {
                        cx.stop_propagation();
                    }))
                    .child(Self::render_menu_button(
                        "Close",
                        Some("Ctrl+W"),
                        cx.listener(move |this, _, _, cx| {
                            cx.stop_propagation();
                            this.close_tab(target_idx, cx);
                        }),
                    ))
                    .child(Self::render_menu_button(
                        "Close Others",
                        None,
                        cx.listener(move |this, _, _, cx| {
                            cx.stop_propagation();
                            this.close_others(target_idx, cx);
                        }),
                    ))
                    .child(Self::render_menu_button(
                        "Close to the Right",
                        None,
                        cx.listener(move |this, _, _, cx| {
                            cx.stop_propagation();
                            this.close_to_right(target_idx, cx);
                        }),
                    ))
                    .child(Self::render_menu_button(
                        "Close Saved",
                        None,
                        cx.listener(|this, _, _, cx| {
                            cx.stop_propagation();
                            this.close_saved(cx);
                        }),
                    ))
                    .child(div().h(gpui::px(1.0)).bg(theme.border).my_1())
                    .child(Self::render_menu_button(
                        "Close All",
                        None,
                        cx.listener(|this, _, _, cx| {
                            cx.stop_propagation();
                            this.close_all(cx);
                        }),
                    ))
            )
        )
    }

    /// メニューアイテムボタンの共通描画
    fn render_menu_button(
        label: &'static str,
        shortcut: Option<&'static str>,
        on_click: impl Fn(&MouseDownEvent, &mut Window, &mut App) + 'static,
    ) -> impl IntoElement {
        div()
            .w_full()
            .px_2()
            .py_1()
            .rounded_sm()
            .flex()
            .items_center()
            .justify_between()
            .hover(|s| s.bg(gpui::rgb(0x38bdf8).opacity(0.15)))
            .cursor_pointer()
            .on_mouse_down(MouseButton::Left, on_click)
            .child(div().text_xs().child(label))
            .children(shortcut.map(|sc| {
                div().text_xs().text_color(gpui::rgb(0x888888)).child(sc)
            }))
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

        // カスタムファイルタブバーの構築
        let mut tabs_row = div()
            .flex()
            .flex_row()
            .items_center()
            .h_full()
            .overflow_x_hidden();

        let active_tab_idx = self.active_tab;

        for (idx, tab) in self.tabs.iter().enumerate() {
            let is_active = idx == active_tab_idx;
            let is_dirty = tab.editor.as_ref().map(|e| e.read(cx).is_dirty(cx)).unwrap_or(false);

            let tab_element = div()
                .relative()
                .id(format!("custom-tab-{}", idx))
                .h_full()
                .px_3()
                .flex()
                .items_center()
                .gap_2()
                .border_r_1()
                .border_color(cx.theme().border)
                .bg(if is_active {
                    cx.theme().background
                } else {
                    cx.theme().muted.opacity(0.3)
                })
                .hover(|s| {
                    if !is_active {
                        s.bg(cx.theme().secondary)
                    } else {
                        s
                    }
                })
                .cursor_pointer()
                .on_mouse_down(MouseButton::Left, cx.listener(move |this, _, _, cx| {
                    this.active_tab = idx;
                    this.context_menu = None;
                    cx.notify();
                }))
                .on_mouse_down(MouseButton::Right, cx.listener(move |this, event: &MouseDownEvent, _, cx| {
                    this.active_tab = idx;
                    this.context_menu = Some(TabContextMenu {
                        tab_index: idx,
                        position: event.position,
                    });
                    cx.notify();
                }))
                // 上部のアクティブインジケータ線 (VSCode スタイル)
                .children(if is_active {
                    Some(div().absolute().left_0().right_0().top_0().h(gpui::px(2.0)).bg(gpui::rgb(0x007acc)))
                } else {
                    None
                })
                // ファイルアイコン
                .child(
                    Icon::new(IconName::File)
                        .text_color(if is_active {
                            cx.theme().foreground
                        } else {
                            cx.theme().muted_foreground
                        })
                )
                // ファイル名
                .child(
                    div()
                        .text_sm()
                        .text_color(if is_active {
                            cx.theme().foreground
                        } else {
                            cx.theme().muted_foreground
                        })
                        .child(tab.title.clone())
                )
                // ダーティインジケータ（変更ありの場合）
                .children(if is_dirty {
                    Some(
                        div()
                            .w_2()
                            .h_2()
                            .rounded_full()
                            .bg(cx.theme().foreground)
                    )
                } else {
                    None
                })
                // タブ閉じるボタン (✕)
                .child(
                    div()
                        .w(gpui::px(18.0))
                        .h(gpui::px(18.0))
                        .rounded_sm()
                        .flex()
                        .items_center()
                        .justify_center()
                        .text_xs()
                        .text_color(cx.theme().muted_foreground)
                        .hover(|s| s.bg(cx.theme().secondary).text_color(cx.theme().foreground))
                        .on_mouse_down(MouseButton::Left, cx.listener(move |this, _, _, cx| {
                            cx.stop_propagation();
                            this.close_tab(idx, cx);
                        }))
                        .child("✕")
                );

            tabs_row = tabs_row.child(tab_element);
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
            // 外側クリックでコンテキストメニューを閉じる
            .on_mouse_down(MouseButton::Left, cx.listener(|this, _, _, cx| {
                if this.context_menu.is_some() {
                    this.context_menu = None;
                    cx.notify();
                }
            }))
            .child(
                div()
                    .w_full()
                    .h(gpui::px(36.0))
                    .flex()
                    .items_center()
                    .bg(cx.theme().muted.opacity(0.2))
                    .border_b_1()
                    .border_color(cx.theme().border)
                    .child(tabs_row)
            )
            .child(
                div().flex_grow(1.).w_full().child(active_editor)
            )
            .children(self.render_context_menu(cx))
            .into_any_element()
    }
}
