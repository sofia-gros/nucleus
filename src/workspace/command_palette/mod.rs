/// コマンドパレットおよびクイックファイルオープン (Quick Open) モーダル UI モジュール

pub mod fuzzy;

use gpui::*;
use gpui_component::theme::ActiveTheme;
use gpui_component::{Icon, IconName};
use fuzzy::fuzzy_match;
use std::path::Path;

/// パレットの表示モード
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PaletteMode {
    /// ファイル検索 (Ctrl+P)
    FileSearch,
    /// コマンド検索 (Ctrl+Shift+P)
    CommandPalette,
}

/// パレットで選択可能なアイテム
#[derive(Clone, Debug)]
pub enum PaletteItem {
    File {
        path: String,
        file_name: String,
        dir: String,
    },
    Command {
        title: String,
        category: String,
        command: String,
        shortcut: Option<String>,
    },
}

pub struct CommandPalette {
    pub is_open: bool,
    pub mode: PaletteMode,
    pub query: String,
    pub selected_index: usize,
    pub all_files: Vec<PaletteItem>,
    pub all_commands: Vec<PaletteItem>,
    pub focus_handle: FocusHandle,
}

impl CommandPalette {
    pub fn new(cx: &mut Context<Self>) -> Self {
        Self {
            is_open: false,
            mode: PaletteMode::FileSearch,
            query: String::new(),
            selected_index: 0,
            all_files: Vec::new(),
            all_commands: Self::default_commands(),
            focus_handle: cx.focus_handle(),
        }
    }

    /// デフォルトの IDE コマンド一覧
    fn default_commands() -> Vec<PaletteItem> {
        vec![
            PaletteItem::Command { category: "File".into(), title: "New File".into(), command: "file.new".into(), shortcut: Some("Ctrl+N".into()) },
            PaletteItem::Command { category: "File".into(), title: "Save".into(), command: "file.save".into(), shortcut: Some("Ctrl+S".into()) },
            PaletteItem::Command { category: "File".into(), title: "Save All".into(), command: "file.save_all".into(), shortcut: None },
            PaletteItem::Command { category: "View".into(), title: "Toggle Primary Sidebar".into(), command: "view.toggle_sidebar".into(), shortcut: Some("Ctrl+B".into()) },
            PaletteItem::Command { category: "View".into(), title: "Toggle Terminal".into(), command: "view.toggle_terminal".into(), shortcut: Some("Ctrl+`".into()) },
            PaletteItem::Command { category: "View".into(), title: "Show Explorer".into(), command: "view.show_explorer".into(), shortcut: Some("Ctrl+Shift+E".into()) },
            PaletteItem::Command { category: "View".into(), title: "Show Source Control".into(), command: "view.show_git".into(), shortcut: Some("Ctrl+Shift+G".into()) },
            PaletteItem::Command { category: "Git".into(), title: "Commit".into(), command: "git.commit".into(), shortcut: None },
            PaletteItem::Command { category: "Git".into(), title: "Refresh Status".into(), command: "git.refresh".into(), shortcut: None },
            PaletteItem::Command { category: "Terminal".into(), title: "New Terminal".into(), command: "terminal.new".into(), shortcut: Some("Ctrl+Shift+`".into()) },
            PaletteItem::Command { category: "Terminal".into(), title: "Clear Terminal".into(), command: "terminal.clear".into(), shortcut: None },
            PaletteItem::Command { category: "Preferences".into(), title: "Open Settings".into(), command: "preferences.open_settings".into(), shortcut: Some("Ctrl+,".into()) },
        ]
    }

    /// ファイル検索モーダルを開く (Ctrl+P)
    pub fn open_file_search(&mut self, root_path: Option<&Path>, cx: &mut Context<Self>) {
        self.mode = PaletteMode::FileSearch;
        self.query.clear();
        self.selected_index = 0;
        self.is_open = true;

        if let Some(root) = root_path {
            self.all_files = Self::collect_files(root);
        }
        cx.notify();
    }

    /// コマンドパレットを開く (Ctrl+Shift+P)
    pub fn open_command_palette(&mut self, cx: &mut Context<Self>) {
        self.mode = PaletteMode::CommandPalette;
        self.query.clear();
        self.selected_index = 0;
        self.is_open = true;
        cx.notify();
    }

    /// パレットを閉じる
    pub fn close(&mut self, cx: &mut Context<Self>) {
        self.is_open = false;
        cx.notify();
    }

    /// ディレクトリ内のファイルを再帰収集
    fn collect_files(root: &Path) -> Vec<PaletteItem> {
        let mut items = Vec::new();
        let mut stack = vec![root.to_path_buf()];

        while let Some(dir) = stack.pop() {
            if let Ok(entries) = std::fs::read_dir(&dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    let name = entry.file_name().to_string_lossy().to_string();

                    if path.is_dir() {
                        if name != "target" && name != ".git" && name != "node_modules" && name != ".nucleus" {
                            stack.push(path);
                        }
                    } else {
                        let rel_path = path.strip_prefix(root).unwrap_or(&path).to_string_lossy().replace("\\", "/");
                        let dir_path = std::path::Path::new(&rel_path).parent().map(|p| p.to_string_lossy().to_string()).unwrap_or_default();
                        items.push(PaletteItem::File {
                            path: path.to_string_lossy().to_string(),
                            file_name: name,
                            dir: dir_path,
                        });
                    }
                }
            }
        }
        items
    }

    /// 現在のクエリに基づくマッチング & スコア順ソート
    pub fn filtered_items(&self) -> Vec<PaletteItem> {
        let source = match self.mode {
            PaletteMode::FileSearch => &self.all_files,
            PaletteMode::CommandPalette => &self.all_commands,
        };

        if self.query.is_empty() {
            return source.iter().take(30).cloned().collect();
        }

        let mut scored: Vec<(i32, PaletteItem)> = Vec::new();
        for item in source {
            let target_text = match item {
                PaletteItem::File { file_name, dir, .. } => format!("{}/{}", dir, file_name),
                PaletteItem::Command { category, title, .. } => format!("{}: {}", category, title),
            };

            if let Some(m) = fuzzy_match(&self.query, &target_text) {
                scored.push((m.score, item.clone()));
            }
        }

        scored.sort_by(|a, b| b.0.cmp(&a.0));
        scored.into_iter().take(30).map(|(_, item)| item).collect()
    }
}

impl Render for CommandPalette {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if !self.is_open {
            return div().into_any_element();
        }

        let theme = cx.theme().clone();
        let items = self.filtered_items();
        let selected_idx = self.selected_index.min(items.len().saturating_sub(1));

        let placeholder = match self.mode {
            PaletteMode::FileSearch => "Search files by name (e.g. main.rs, Cargo.toml)...",
            PaletteMode::CommandPalette => "Type a command to execute...",
        };

        let prefix_icon = match self.mode {
            PaletteMode::FileSearch => IconName::Search,
            PaletteMode::CommandPalette => IconName::ChevronRight,
        };

        // アイテムリスト
        let mut list_elements = div().flex().flex_col().w_full().max_h(gpui::px(320.0)).overflow_hidden();

        for (idx, item) in items.iter().enumerate() {
            let is_selected = idx == selected_idx;
            let item_clone = item.clone();

            let row = match item {
                PaletteItem::File { file_name, dir, .. } => {
                    div()
                        .px_3()
                        .py_2()
                        .rounded_sm()
                        .flex()
                        .items_center()
                        .justify_between()
                        .bg(if is_selected { theme.secondary } else { theme.background })
                        .hover(|s| s.bg(theme.secondary))
                        .cursor_pointer()
                        .on_mouse_down(MouseButton::Left, cx.listener(move |this, _, _, cx| {
                            this.execute_item(&item_clone, cx);
                        }))
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap_2()
                                .child(Icon::new(IconName::File).size(gpui::px(14.0)).text_color(theme.muted_foreground))
                                .child(div().text_xs().font_weight(FontWeight::SEMIBOLD).text_color(theme.foreground).child(file_name.clone()))
                                .children(if !dir.is_empty() {
                                    Some(div().text_xs().text_color(theme.muted_foreground).child(dir.clone()))
                                } else {
                                    None
                                })
                        )
                }
                PaletteItem::Command { category, title, shortcut, .. } => {
                    div()
                        .px_3()
                        .py_2()
                        .rounded_sm()
                        .flex()
                        .items_center()
                        .justify_between()
                        .bg(if is_selected { theme.secondary } else { theme.background })
                        .hover(|s| s.bg(theme.secondary))
                        .cursor_pointer()
                        .on_mouse_down(MouseButton::Left, cx.listener(move |this, _, _, cx| {
                            this.execute_item(&item_clone, cx);
                        }))
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap_1p5()
                                .child(div().text_xs().text_color(theme.muted_foreground).child(format!("{}:", category)))
                                .child(div().text_xs().font_weight(FontWeight::SEMIBOLD).text_color(theme.foreground).child(title.clone()))
                        )
                        .children(shortcut.as_ref().map(|s| {
                            div().text_xs().text_color(theme.muted_foreground).child(s.clone())
                        }))
                }
            };

            list_elements = list_elements.child(row);
        }

        // モーダル全体（画面中央上部にフロート）
        gpui::deferred(
            div()
                .absolute()
                .inset_0()
                .bg(gpui::rgb(0x000000).opacity(0.4))
                .flex()
                .justify_center()
                .items_start()
                .pt(gpui::px(40.0))
                .on_mouse_down(MouseButton::Left, cx.listener(|this, _, _, cx| {
                    this.close(cx);
                }))
                .child(
                    div()
                        .w(gpui::px(600.0))
                        .bg(theme.background)
                        .border_1()
                        .border_color(theme.border)
                        .rounded_lg()
                        .shadow_2xl()
                        .flex()
                        .flex_col()
                        .overflow_hidden()
                        .on_mouse_down(MouseButton::Left, cx.listener(|_, _, _, cx| {
                            cx.stop_propagation();
                        }))
                        // 検索入力欄
                        .child(
                            div()
                                .p_2p5()
                                .border_b_1()
                                .border_color(theme.border)
                                .flex()
                                .items_center()
                                .gap_2()
                                .child(Icon::new(prefix_icon).size(gpui::px(14.0)).text_color(theme.muted_foreground))
                                .child(
                                    div()
                                        .flex_1()
                                        .text_sm()
                                        .text_color(theme.foreground)
                                        .child(if self.query.is_empty() {
                                            div().text_color(theme.muted_foreground).child(placeholder).into_any_element()
                                        } else {
                                            div().child(self.query.clone()).into_any_element()
                                        })
                                )
                        )
                        .child(
                            div().p_1().child(list_elements)
                        )
                )
        ).into_any_element()
    }
}

impl CommandPalette {
    /// 選択されたアイテムの実行
    pub fn execute_item(&mut self, item: &PaletteItem, cx: &mut Context<Self>) {
        self.is_open = false;

        match item {
            PaletteItem::File { path, file_name, .. } => {
                if let Ok(content) = std::fs::read_to_string(path) {
                    if cx.has_global::<crate::plugin_manager::PluginManagerGlobal>() {
                        let pm = cx.global::<crate::plugin_manager::PluginManagerGlobal>().0.clone();
                        let p = path.clone();
                        let title = file_name.clone();
                        pm.update(cx, |pm, _| {
                            pm.dispatch_action(crate::plugin_manager::action::PluginAction::OpenTab {
                                path: p,
                                title,
                                content,
                            });
                        });
                    }
                }
            }
            PaletteItem::Command { command, .. } => {
                if cx.has_global::<crate::plugin_manager::PluginManagerGlobal>() {
                    let pm = cx.global::<crate::plugin_manager::PluginManagerGlobal>().0.clone();
                    let cmd = command.clone();
                    pm.update(cx, |pm, _| {
                        pm.dispatch_event(crate::plugin_manager::event::PluginEvent::CommandExecuted { command: cmd });
                    });
                }
            }
        }
        cx.notify();
    }
}
