/// エディタ領域およびタブ管理コンポーネント (コード補完・ホバー・リネーム・Quick Fix・SettingsView統合)

pub mod highlighter;
pub mod settings_view;
pub mod keybindings_view;

use std::path::PathBuf;
use gpui::*;
use gpui_component::{Icon, IconName};
use gpui_component::theme::ActiveTheme;
use crate::editor::Editor;
use crate::editor::completion::CompletionState;
use crate::editor::hover::HoverState;
use crate::editor::find_replace::FindReplaceState;
use crate::lsp::inlay_hints::SignatureHelpState;
use self::settings_view::SettingsView;
use self::keybindings_view::KeybindingsView;

/// 単一のエディタタブ情報
pub struct EditorTab {
    /// ファイルパス
    pub path: String,
    /// タブの表示タイトル
    pub title: String,
    /// エディタエンティティ（初期化前は None）
    pub editor: Option<Entity<Editor>>,
    /// 設定画面エンティティ（設定タブの場合）
    pub settings_view: Option<Entity<SettingsView>>,
    /// キーバインド設定画面エンティティ（ショートカットタブの場合）
    pub keybindings_view: Option<Entity<KeybindingsView>>,
}

/// タブの右クリックコンテキストメニュー情報
#[derive(Clone, Debug)]
pub struct TabContextMenu {
    /// 対象のタブインデックス
    pub tab_index: usize,
    /// クリックされた画面座標
    pub position: Point<Pixels>,
}

/// シンボルリネームの状態
#[derive(Clone, Debug)]
pub struct RenameState {
    pub current_name: String,
    pub new_name: String,
}

/// クイックフィックスアクション候補
#[derive(Clone, Debug)]
pub struct QuickFixItem {
    pub title: String,
    pub action_id: String,
}

/// クイックフィックスポップアップの状態
#[derive(Clone, Debug, Default)]
pub struct QuickFixState {
    pub is_open: bool,
    pub x: f32,
    pub y: f32,
    pub items: Vec<QuickFixItem>,
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
    /// コード補完状態
    pub completion_state: CompletionState,
    /// ホバーツールチップ状態
    pub hover_state: HoverState,
    /// インライン検索・置換状態
    pub find_replace: FindReplaceState,
    /// シグネチャヘルプ状態
    pub signature_help: SignatureHelpState,
    /// シンボルリネームモーダル状態
    pub rename_state: Option<RenameState>,
    /// クイックフィックスポップアップ状態
    pub quick_fix_state: QuickFixState,
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
            completion_state: CompletionState::new(),
            hover_state: HoverState::new(),
            find_replace: FindReplaceState::new(),
            signature_help: SignatureHelpState::new(),
            rename_state: None,
            quick_fix_state: QuickFixState::default(),
        }
    }

    /// 設定画面タブを開く
    pub fn open_settings(&mut self, cx: &mut Context<Self>) {
        let settings_path = "nucleus://settings";
        if let Some(pos) = self.tabs.iter().position(|t| t.path == settings_path) {
            self.active_tab = pos;
        } else {
            let sv = cx.new(|cx| SettingsView::new(cx));
            self.tabs.push(EditorTab {
                path: settings_path.to_string(),
                title: "Settings".to_string(),
                editor: None,
                settings_view: Some(sv),
                keybindings_view: None,
            });
            self.active_tab = self.tabs.len() - 1;
        }
        cx.notify();
    }

    /// キーバインド設定画面タブを開く
    pub fn open_keybindings(&mut self, cx: &mut Context<Self>) {
        let kb_path = "nucleus://keybindings";
        if let Some(pos) = self.tabs.iter().position(|t| t.path == kb_path) {
            self.active_tab = pos;
        } else {
            let kb = cx.new(|_| KeybindingsView::new());
            self.tabs.push(EditorTab {
                path: kb_path.to_string(),
                title: "Keyboard Shortcuts".to_string(),
                editor: None,
                settings_view: None,
                keybindings_view: Some(kb),
            });
            self.active_tab = self.tabs.len() - 1;
        }
        cx.notify();
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

    /// 指定したタブより右側のタブをすべて閉じる (Close to the Right)
    pub fn close_to_right(&mut self, target_idx: usize, cx: &mut Context<Self>) {
        if target_idx + 1 < self.tabs.len() {
            self.tabs.truncate(target_idx + 1);
            if self.active_tab > target_idx {
                self.active_tab = target_idx;
            }
        }
        self.context_menu = None;
        cx.notify();
    }

    /// 保存済みのタブをすべて閉じる (Close Saved)
    pub fn close_saved(&mut self, cx: &mut Context<Self>) {
        let mut i = 0;
        while i < self.tabs.len() {
            let is_dirty = self.tabs[i].editor.as_ref().map(|e| e.read(cx).is_dirty(cx)).unwrap_or(false);
            if !is_dirty {
                self.tabs.remove(i);
                if self.active_tab >= self.tabs.len() && self.active_tab > 0 {
                    self.active_tab = self.tabs.len() - 1;
                }
            } else {
                i += 1;
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

    /// タブを開く（既存ならフォーカス）
    pub fn open_tab(&mut self, path: String, title: String, content: String, cx: &mut Context<Self>) {
        if let Some(pos) = self.tabs.iter().position(|t| t.path == path) {
            self.active_tab = pos;
        } else {
            self.tabs.push(EditorTab {
                path: path.clone(),
                title,
                editor: None,
                settings_view: None,
                keybindings_view: None,
            });
            self.pending_contents.insert(path, content);
            self.active_tab = self.tabs.len() - 1;
        }
        cx.notify();
    }

    /// アクティブなタブの内容をファイルに保存する
    pub fn save_active_tab(&mut self, cx: &mut Context<Self>) {
        if self.active_tab < self.tabs.len() {
            let tab = &self.tabs[self.active_tab];
            if let Some(editor_entity) = &tab.editor {
                editor_entity.update(cx, |editor, cx| {
                    let _ = editor.save(cx);
                });
                cx.notify();
            }
        }
    }

    /// 外部ファイル変更検知時のクリーンタブ再読み込み
    pub fn reload_tab_if_clean(&mut self, path: &str, cx: &mut Context<Self>) {
        if let Some(tab) = self.tabs.iter_mut().find(|t| t.path == path) {
            let is_dirty = tab.editor.as_ref().map(|e| e.read(cx).is_dirty(cx)).unwrap_or(false);
            if !is_dirty {
                if let Ok(new_content) = std::fs::read_to_string(path) {
                    tab.editor = None;
                    self.pending_contents.insert(path.to_string(), new_content);
                    cx.notify();
                }
            }
        }
    }

    /// シンボルリネームの開始 (F2)
    pub fn start_rename(&mut self, current_name: String, cx: &mut Context<Self>) {
        self.rename_state = Some(RenameState {
            new_name: current_name.clone(),
            current_name,
        });
        cx.notify();
    }

    /// クイックフィックスの表示 (Ctrl+.)
    pub fn show_quick_fix(&mut self, x: f32, y: f32, items: Vec<QuickFixItem>, cx: &mut Context<Self>) {
        self.quick_fix_state = QuickFixState {
            is_open: true,
            x,
            y,
            items,
        };
        cx.notify();
    }

    /// ファイル拡張子に応じた言語判定
    pub fn detect_language(path_str: &str) -> &'static str {
        let p = std::path::Path::new(path_str);
        let file_name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
        
        if file_name.ends_with(".lock") || file_name == "Cargo.lock" || file_name == "Pipfile" || file_name == "Gopkg.lock" {
            return "toml";
        }

        match p.extension().and_then(|ext| ext.to_str()) {
            Some("rs") => "rust",
            Some("toml") => "toml",
            Some("json") => "json",
            Some("js") => "javascript",
            Some("ts") => "typescript",
            Some("md") => "markdown",
            Some("yaml") | Some("yml") => "yaml",
            _ => "text",
        }
    }

    /// コンテキストメニューの描画
    fn render_context_menu(&self, cx: &mut Context<Self>) -> Option<impl IntoElement> {
        let menu = self.context_menu.as_ref()?;
        let target_idx = menu.tab_index;
        let pos = menu.position;
        let theme = cx.theme().clone();

        Some(
            gpui::deferred(
                div()
                    .absolute()
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

    /// 補完サジェストポップアップの描画
    fn render_completion_popup(&self, cx: &mut Context<Self>) -> Option<impl IntoElement> {
        if !self.completion_state.is_open {
            return None;
        }

        let theme = cx.theme().clone();
        let items = self.completion_state.filtered_items();
        if items.is_empty() {
            return None;
        }

        let mut list = div().flex().flex_col().w_full().max_h(gpui::px(200.0)).overflow_hidden().p_1();

        for (idx, item) in items.iter().enumerate() {
            let is_selected = idx == self.completion_state.selected_index;
            let icon_label = item.kind.icon_label();
            let label = item.label.clone();
            let detail = item.detail.clone().unwrap_or_default();

            let row = div()
                .px_2()
                .py_1()
                .rounded_sm()
                .flex()
                .items_center()
                .justify_between()
                .bg(if is_selected { theme.secondary } else { theme.background })
                .hover(|s| s.bg(theme.secondary))
                .cursor_pointer()
                .child(
                    div().flex().items_center().gap_2()
                        .child(
                            div().text_xs().font_weight(FontWeight::BOLD).text_color(gpui::rgb(0x38bdf8)).child(icon_label)
                        )
                        .child(
                            div().text_xs().font_weight(FontWeight::SEMIBOLD).text_color(theme.foreground).child(label)
                        )
                )
                .children(if !detail.is_empty() {
                    Some(div().text_xs().text_color(theme.muted_foreground).child(detail))
                } else {
                    None
                });

            list = list.child(row);
        }

        Some(
            gpui::deferred(
                div()
                    .absolute()
                    .top(gpui::px(60.0))
                    .left(gpui::px(80.0))
                    .w(gpui::px(280.0))
                    .bg(theme.background)
                    .border_1()
                    .border_color(theme.border)
                    .rounded_md()
                    .shadow_xl()
                    .child(list)
            )
        )
    }

    /// ホバーツールチップの描画
    fn render_hover_tooltip(&self, cx: &mut Context<Self>) -> Option<impl IntoElement> {
        if !self.hover_state.is_visible {
            return None;
        }

        let info = self.hover_state.info.as_ref()?;
        let theme = cx.theme().clone();

        Some(
            gpui::deferred(
                div()
                    .absolute()
                    .top(gpui::px(self.hover_state.y))
                    .left(gpui::px(self.hover_state.x))
                    .max_w(gpui::px(400.0))
                    .bg(theme.background)
                    .border_1()
                    .border_color(theme.border)
                    .rounded_md()
                    .shadow_lg()
                    .p_2()
                    .text_xs()
                    .text_color(theme.foreground)
                    .child(info.contents.clone())
            )
        )
    }

    /// リネーム入力モーダルの描画 (F2)
    fn render_rename_modal(&self, cx: &mut Context<Self>) -> Option<impl IntoElement> {
        let rename = self.rename_state.as_ref()?;
        let theme = cx.theme().clone();

        Some(
            gpui::deferred(
                div()
                    .absolute()
                    .top(gpui::px(100.0))
                    .left(gpui::px(150.0))
                    .w(gpui::px(320.0))
                    .bg(theme.background)
                    .border_1()
                    .border_color(theme.border)
                    .rounded_md()
                    .shadow_2xl()
                    .p_3()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .on_mouse_down(MouseButton::Left, cx.listener(|_, _, _, cx| {
                        cx.stop_propagation();
                    }))
                    .child(
                        div().text_xs().font_weight(FontWeight::BOLD).text_color(theme.muted_foreground).child("RENAME SYMBOL")
                    )
                    .child(
                        div().px_2().py_1p5().bg(theme.muted.opacity(0.3)).border_1().border_color(theme.border).rounded_md()
                            .child(div().text_xs().text_color(theme.foreground).child(rename.new_name.clone()))
                    )
                    .child(
                        div().flex().justify_end().gap_2()
                            .child(
                                div().px_2().py_1().bg(theme.muted).rounded_sm().text_xs().cursor_pointer()
                                    .hover(|s| s.bg(theme.secondary))
                                    .on_mouse_down(MouseButton::Left, cx.listener(|this, _, _, cx| {
                                        this.rename_state = None;
                                        cx.notify();
                                    }))
                                    .child("Cancel")
                            )
                            .child(
                                div().px_2().py_1().bg(gpui::rgb(0x007acc)).text_color(gpui::rgb(0xffffff)).rounded_sm().text_xs().cursor_pointer()
                                    .hover(|s| s.bg(gpui::rgb(0x0062a3)))
                                    .on_mouse_down(MouseButton::Left, cx.listener(|this, _, _, cx| {
                                        this.rename_state = None;
                                        cx.notify();
                                    }))
                                    .child("Rename")
                            )
                    )
            )
        )
    }

    /// クイックフィックスポップアップの描画 (Ctrl+.)
    fn render_quick_fix_popup(&self, cx: &mut Context<Self>) -> Option<impl IntoElement> {
        if !self.quick_fix_state.is_open || self.quick_fix_state.items.is_empty() {
            return None;
        }

        let theme = cx.theme().clone();
        let mut list = div().flex().flex_col().w_full().p_1();

        for item in &self.quick_fix_state.items {
            let title = item.title.clone();
            let row = div()
                .px_2()
                .py_1()
                .rounded_sm()
                .flex()
                .items_center()
                .gap_2()
                .hover(|s| s.bg(theme.secondary))
                .cursor_pointer()
                .on_mouse_down(MouseButton::Left, cx.listener(|this, _, _, cx| {
                    this.quick_fix_state.is_open = false;
                    cx.notify();
                }))
                .child(div().text_xs().text_color(gpui::rgb(0xeab308)).child("💡"))
                .child(div().text_xs().text_color(theme.foreground).child(title));

            list = list.child(row);
        }

        Some(
            gpui::deferred(
                div()
                    .absolute()
                    .top(gpui::px(self.quick_fix_state.y))
                    .left(gpui::px(self.quick_fix_state.x))
                    .w(gpui::px(260.0))
                    .bg(theme.background)
                    .border_1()
                    .border_color(theme.border)
                    .rounded_md()
                    .shadow_xl()
                    .child(list)
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
            if tab.editor.is_none() && tab.settings_view.is_none() {
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

        // カスタムファイルタブバーの構築（横スクロール・省略表示対応）
        let mut tabs_row = div()
            .flex()
            .flex_row()
            .items_center()
            .h_full()
            .flex_1()
            .overflow_x_hidden();

        let active_tab_idx = self.active_tab;

        for (idx, tab) in self.tabs.iter().enumerate() {
            let is_active = idx == active_tab_idx;
            let is_dirty = tab.editor.as_ref().map(|e| e.read(cx).is_dirty(cx)).unwrap_or(false);
            let is_settings = tab.settings_view.is_some();

            let tab_element = div()
                .relative()
                .id(format!("custom-tab-{}", idx))
                .h_full()
                .px_3()
                .flex_shrink_0()
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
                // アクティブタブの上部ブルーインジケータ (2px)
                .children(if is_active {
                    Some(
                        div()
                            .absolute()
                            .top_0()
                            .left_0()
                            .right_0()
                            .h(gpui::px(2.0))
                            .bg(gpui::rgb(0x007acc))
                    )
                } else {
                    None
                })
                // アイコン
                .child(
                    if is_settings {
                        Icon::new(IconName::Settings).size(gpui::px(14.0)).text_color(if is_active { cx.theme().foreground } else { cx.theme().muted_foreground })
                    } else {
                        Icon::new(IconName::File).size(gpui::px(14.0)).text_color(if is_active { cx.theme().foreground } else { cx.theme().muted_foreground })
                    }
                )
                // タイトル（最大幅と省略表示）
                .child(
                    div()
                        .max_w(gpui::px(160.0))
                        .overflow_hidden()
                        .text_xs()
                        .font_weight(if is_active { FontWeight::SEMIBOLD } else { FontWeight::NORMAL })
                        .text_color(if is_active { cx.theme().foreground } else { cx.theme().muted_foreground })
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

        let active_content = if self.active_tab < self.tabs.len() {
            let tab = &self.tabs[self.active_tab];
            if let Some(sv) = &tab.settings_view {
                sv.clone().into_any_element()
            } else if let Some(kb) = &tab.keybindings_view {
                kb.clone().into_any_element()
            } else if let Some(editor_entity) = &tab.editor {
                editor_entity.clone().into_any_element()
            } else {
                div().p_4().text_sm().text_color(cx.theme().muted_foreground).child("Loading...").into_any_element()
            }
        } else {
            div().into_any_element()
        };

        div()
            .relative()
            .size_full()
            .flex()
            .flex_col()
            .bg(cx.theme().background)
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
                div().flex_grow(1.).w_full().child(active_content)
            )
            .children(self.render_context_menu(cx))
            .children(self.render_completion_popup(cx))
            .children(self.render_hover_tooltip(cx))
            .children(self.render_rename_modal(cx))
            .children(self.render_quick_fix_popup(cx))
            .children(self.render_find_replace_bar(cx))
            .children(self.render_signature_help(cx))
            .into_any_element()
    }
}

impl EditorArea {
    /// インライン検索・置換バーの描画
    fn render_find_replace_bar(&self, cx: &mut Context<Self>) -> Option<impl IntoElement> {
        if !self.find_replace.is_open {
            return None;
        }

        let theme = cx.theme();
        let match_count = self.find_replace.matches.len();
        let current_idx = if match_count > 0 { self.find_replace.current_match_index + 1 } else { 0 };
        let count_label = format!("{}/{}", current_idx, match_count);

        Some(
            gpui::deferred(
                div()
                    .absolute()
                    .top(gpui::px(40.0))
                    .right(gpui::px(20.0))
                    .w(gpui::px(360.0))
                    .bg(theme.background)
                    .border_1()
                    .border_color(theme.border)
                    .rounded_md()
                    .shadow_lg()
                    .p_2()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(
                        // 検索入力行
                        div().flex().items_center().gap_1()
                            .child(
                                div()
                                    .flex_1()
                                    .px_2()
                                    .py_1()
                                    .bg(theme.muted.opacity(0.3))
                                    .border_1()
                                    .border_color(theme.border)
                                    .rounded_sm()
                                    .text_xs()
                                    .child(if self.find_replace.query.is_empty() { "Find...".to_string() } else { self.find_replace.query.clone() })
                            )
                            .child(div().text_xs().text_color(theme.muted_foreground).child(count_label))
                            .child(
                                div().px_1p5().py_0p5().bg(theme.muted).rounded_sm().text_xs().cursor_pointer()
                                    .hover(|s| s.bg(theme.secondary))
                                    .on_mouse_down(MouseButton::Left, cx.listener(|this, _, _, cx| {
                                        this.find_replace.prev_match();
                                        cx.notify();
                                    }))
                                    .child("↑")
                            )
                            .child(
                                div().px_1p5().py_0p5().bg(theme.muted).rounded_sm().text_xs().cursor_pointer()
                                    .hover(|s| s.bg(theme.secondary))
                                    .on_mouse_down(MouseButton::Left, cx.listener(|this, _, _, cx| {
                                        this.find_replace.next_match();
                                        cx.notify();
                                    }))
                                    .child("↓")
                            )
                            .child(
                                div().px_1p5().py_0p5().bg(theme.muted).rounded_sm().text_xs().cursor_pointer()
                                    .hover(|s| s.bg(theme.secondary))
                                    .on_mouse_down(MouseButton::Left, cx.listener(|this, _, _, cx| {
                                        this.find_replace.close();
                                        cx.notify();
                                    }))
                                    .child("✕")
                            )
                    )
                    .children(if self.find_replace.is_replace_open {
                        Some(
                            // 置換入力行
                            div().flex().items_center().gap_1()
                                .child(
                                    div()
                                        .flex_1()
                                        .px_2()
                                        .py_1()
                                        .bg(theme.muted.opacity(0.3))
                                        .border_1()
                                        .border_color(theme.border)
                                        .rounded_sm()
                                        .text_xs()
                                        .child(if self.find_replace.replace_text.is_empty() { "Replace...".to_string() } else { self.find_replace.replace_text.clone() })
                                )
                                .child(
                                    div().px_2().py_0p5().bg(theme.muted).rounded_sm().text_xs().cursor_pointer()
                                        .hover(|s| s.bg(theme.secondary))
                                        .child("Replace")
                                )
                        )
                    } else {
                        None
                    })
            )
        )
    }

    /// シグネチャヘルプツールチップの描画
    fn render_signature_help(&self, cx: &mut Context<Self>) -> Option<impl IntoElement> {
        if !self.signature_help.is_visible {
            return None;
        }
        let theme = cx.theme();
        Some(
            gpui::deferred(
                div()
                    .absolute()
                    .top(gpui::px(50.0))
                    .left(gpui::px(100.0))
                    .max_w(gpui::px(400.0))
                    .bg(theme.background)
                    .border_1()
                    .border_color(theme.border)
                    .rounded_md()
                    .shadow_lg()
                    .p_2()
                    .text_xs()
                    .child(div().font_family("Consolas").font_weight(FontWeight::BOLD).text_color(theme.foreground).child(self.signature_help.label.clone()))
                    .children(if let Some(doc) = &self.signature_help.doc {
                        Some(div().mt_1().text_color(theme.muted_foreground).child(doc.clone()))
                    } else {
                        None
                    })
            )
        )
    }
}
