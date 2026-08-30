/// エクスプローラーおよびプラグインサイドバーのUI描画コンポーネント (VSCode 精密再現版)

use gpui::*;
use gpui_component::*;
use gpui_component::tree::{tree, TreeState, TreeItem};
use gpui_component::list::ListItem;
use gpui_component::theme::ActiveTheme;
use std::sync::Arc;
use std::path::{Path, PathBuf};
use crate::file_system::FileEntry;
use crate::search::{search_in_project, replace_in_file, SearchResult};

fn hex_to_color(hex: &str) -> gpui::Hsla {
    let clean = hex.trim_start_matches('#');
    if clean.len() >= 6 {
        if let (Ok(r), Ok(g), Ok(b)) = (
            u8::from_str_radix(&clean[0..2], 16),
            u8::from_str_radix(&clean[2..4], 16),
            u8::from_str_radix(&clean[4..6], 16),
        ) {
            return gpui::rgb(((r as u32) << 16) | ((g as u32) << 8) | (b as u32)).into();
        }
    }
    gpui::rgb(0xcccccc).into()
}

pub struct LeftSidebar {
    root_path: Option<PathBuf>,
    pub tree_state: Option<Entity<TreeState>>,
    raw_entries: Option<Vec<FileEntry>>,
    pub active_panel: String,
    pub commit_message: String,
    pub search_query: String,
    pub replace_query: String,
    pub search_results: Vec<SearchResult>,
    pub case_sensitive: bool,
    pub focus_handle: FocusHandle,
}

impl LeftSidebar {
    pub fn new(root_path: Option<PathBuf>, cx: &mut Context<Self>) -> Self {
        Self {
            root_path,
            tree_state: None,
            raw_entries: None,
            active_panel: "explorer".to_string(),
            commit_message: String::new(),
            search_query: String::new(),
            replace_query: String::new(),
            search_results: Vec::new(),
            case_sensitive: false,
            focus_handle: cx.focus_handle(),
        }
    }

    pub fn set_active_panel(&mut self, id: String, cx: &mut Context<Self>) {
        self.active_panel = id;
        cx.notify();
    }

    pub fn set_root(&mut self, path: Option<PathBuf>, cx: &mut Context<Self>) {
        self.root_path = path.clone();
        if self.tree_state.is_none() {
            self.tree_state = Some(cx.new(|cx| TreeState::new(cx)));
        }
        
        if let Some(p) = path {
            let executor = cx.background_executor().clone();
            let window_cx = cx.to_async();
            let this_weak = cx.weak_entity();
            let app_cx: &mut gpui::App = cx;
            
            app_cx.spawn(|_cx: &mut gpui::AsyncApp| async move {
                let entries = executor.spawn(async move {
                    FileEntry::read_dir(&p)
                }).await;
                
                window_cx.update(|cx| {
                    let _ = this_weak.update(cx, |this: &mut Self, cx| {
                        this.raw_entries = entries.clone();
                        if let (Some(entries), Some(tree_state)) = (&this.raw_entries, &this.tree_state) {
                            let items = Self::convert_to_tree_items(entries);
                            tree_state.update(cx, |state, cx| state.set_items(items, cx));
                        }
                        cx.notify();
                    });
                });
            }).detach();
        }
        cx.notify();
    }

    /// プロジェクト内検索を実行
    pub fn execute_search(&mut self, cx: &mut Context<Self>) {
        if let Some(ref root) = self.root_path {
            self.search_results = search_in_project(root, &self.search_query, self.case_sensitive);
        }
        cx.notify();
    }

    /// 一致箇所の一括置換を実行
    pub fn execute_replace_all(&mut self, cx: &mut Context<Self>) {
        if !self.search_query.is_empty() {
            for res in &self.search_results {
                let _ = replace_in_file(&res.file_path, &self.search_query, &self.replace_query);
            }
            self.execute_search(cx);
        }
    }

    fn convert_to_tree_items(entries: &[FileEntry]) -> Vec<TreeItem> {
        entries.iter().map(|entry| {
            let is_dir = entry.file_type == crate::file_system::FileType::Directory;
            let children = entry.children.as_ref().map(|c| Self::convert_to_tree_items(c)).unwrap_or_default();
            
            TreeItem::new(
                entry.path.to_string_lossy().to_string(),
                entry.name.clone(),
            )
            .expanded(false)
            .children(if is_dir { children } else { vec![] })
        }).collect()
    }
}

impl Render for LeftSidebar {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let git_status_map = Arc::new(Self::build_git_status_map(cx));

        let list = if let Some(tree_state) = &self.tree_state {
            tree(
                tree_state,
                {
                    let status_map = git_status_map.clone();
                    move |_ix, entry: &gpui_component::tree::TreeEntry, _selected, _window, cx| {
                        let item = entry.item();
                        let path_str = item.id.to_string();
                        let is_folder = !item.children.is_empty();
                        let (icon_text, icon_color_val, status_badge_opt, status_color_val) = if cx.has_global::<crate::plugin_manager::PluginManagerGlobal>() {
                            let pm_global = cx.global::<crate::plugin_manager::PluginManagerGlobal>().0.clone();
                            let pm = pm_global.read(cx);
                            let deco = pm.ui_registry.merge_file_decorations(&path_str, is_folder, &status_map);
                            (
                                deco.icon_text.unwrap_or_else(|| if is_folder { "📁".to_string() } else { "📄".to_string() }),
                                deco.icon_color.unwrap_or_else(|| "#cccccc".to_string()),
                                deco.status_badge,
                                deco.status_color.unwrap_or_else(|| "#888888".to_string()),
                            )
                        } else {
                            let git_status = Self::lookup_git_status_from_map(&status_map, &path_str, is_folder);
                            (
                                if is_folder { "📁".to_string() } else { "📄".to_string() },
                                "#cccccc".to_string(),
                                git_status,
                                "#eab308".to_string(),
                            )
                        };

                        let status_badge: Option<Div> = status_badge_opt.map(|badge| {
                            let is_dot = badge == "●";
                            let color = hex_to_color(&status_color_val);
                            div()
                                .text_xs()
                                .font_weight(if is_dot { FontWeight::NORMAL } else { FontWeight::BOLD })
                                .text_color(color)
                                .child(badge)
                        });

                        let mut list_item = ListItem::new(item.id.clone())
                            .child(
                                div()
                                    .w_full()
                                    .flex()
                                    .items_center()
                                    .justify_between()
                                    .pr_2()
                                    .child(
                                        div()
                                            .flex()
                                            .items_center()
                                            .gap_1p5()
                                            .child(div().text_xs().text_color(hex_to_color(&icon_color_val)).child(icon_text))
                                            .child(div().text_xs().text_color(cx.theme().foreground).child(item.label.clone()))
                                    )
                                    .children(status_badge)
                            );

                        if !is_folder {
                            let click_path = path_str.clone();
                            list_item = list_item.on_click(move |_, _window, cx| {
                                if let Ok(content) = std::fs::read_to_string(&click_path) {
                                    let title = Path::new(&click_path).file_name().unwrap_or_default().to_string_lossy().to_string();
                                    if cx.has_global::<crate::plugin_manager::PluginManagerGlobal>() {
                                        let pm_global = cx.global::<crate::plugin_manager::PluginManagerGlobal>().0.clone();
                                        pm_global.update(cx, |pm, _cx| {
                                            pm.dispatch_action(crate::plugin_manager::action::PluginAction::OpenTab {
                                                path: click_path.clone(),
                                                title,
                                                content,
                                            });
                                        });
                                    }
                                }
                            });
                        }
                        list_item
                    }
                },
            ).into_any_element()
        } else if self.root_path.is_some() {
            div().p_3().text_sm().text_color(cx.theme().muted_foreground).child("Loading...").into_any_element()
        } else {
            div().p_3().text_sm().text_color(cx.theme().muted_foreground).child("No folder opened").into_any_element()
        };

        let active_content = if self.active_panel == "explorer" {
            div()
                .flex()
                .flex_col()
                .size_full()
                .child(
                    div().px_4().py_2().flex().items_center().justify_between().border_b_1().border_color(cx.theme().border)
                        .child(div().text_xs().font_weight(FontWeight::BOLD).text_color(cx.theme().muted_foreground).child("EXPLORER"))
                        .child(
                            div().flex().gap_1().text_xs().text_color(cx.theme().muted_foreground)
                                .child("•••")
                        )
                )
                .child(list)
                .into_any_element()
        } else if self.active_panel == "search" {
            self.render_search_panel(cx).into_any_element()
        } else {
            let mut matched_plugin = None;
            if cx.has_global::<crate::plugin_manager::PluginManagerGlobal>() {
                let pm_global = cx.global::<crate::plugin_manager::PluginManagerGlobal>().0.clone();
                let pm = pm_global.read(cx);
                
                for item in &pm.ui_registry.sidebar_items {
                    if item.id == self.active_panel || (self.active_panel == "git" && item.id == "git_sidebar") || (self.active_panel == "git_sidebar" && item.id == "git") {
                        let title = item.title.clone();
                        let ui_ast = item.ui_ast.clone();
                        
                        let content = if let serde_json::Value::Object(map) = ui_ast {
                            if let Some(t) = map.get("type").and_then(|t| t.as_str()) {
                                if t == "source_control" {
                                    let _branch = map.get("branch").and_then(|b| b.as_str()).unwrap_or("main");
                                    let staged_nodes = map.get("staged_nodes").and_then(|n| n.as_array());
                                    let changes_nodes = map.get("changes_nodes").and_then(|n| n.as_array()).or_else(|| map.get("nodes").and_then(|n| n.as_array()));

                                    let mut sc_layout = div().flex().flex_col().size_full().overflow_hidden();

                                    // 1. コミット入力フォーム & リモート同期バー
                                    sc_layout = sc_layout.child(
                                        div().p_3().flex().flex_col().gap_2().child(
                                            // リモート同期バー (Push, Pull, Sync)
                                            div().flex().items_center().gap_1()
                                                .child(
                                                    div().flex_1().py_1().bg(cx.theme().muted).hover(|s| s.bg(cx.theme().secondary)).rounded_sm().text_xs().flex().items_center().justify_center().cursor_pointer()
                                                        .on_mouse_down(MouseButton::Left, cx.listener(|_, _, _, cx| {
                                                            if cx.has_global::<crate::plugin_manager::PluginManagerGlobal>() {
                                                                let pm = cx.global::<crate::plugin_manager::PluginManagerGlobal>().0.clone();
                                                                pm.update(cx, |pm, _| {
                                                                    pm.dispatch_event(crate::plugin_manager::event::PluginEvent::CommandExecuted { command: "git.push".to_string() });
                                                                });
                                                            }
                                                        }))
                                                        .child("↑ Push")
                                                )
                                                .child(
                                                    div().flex_1().py_1().bg(cx.theme().muted).hover(|s| s.bg(cx.theme().secondary)).rounded_sm().text_xs().flex().items_center().justify_center().cursor_pointer()
                                                        .on_mouse_down(MouseButton::Left, cx.listener(|_, _, _, cx| {
                                                            if cx.has_global::<crate::plugin_manager::PluginManagerGlobal>() {
                                                                let pm = cx.global::<crate::plugin_manager::PluginManagerGlobal>().0.clone();
                                                                pm.update(cx, |pm, _| {
                                                                    pm.dispatch_event(crate::plugin_manager::event::PluginEvent::CommandExecuted { command: "git.pull".to_string() });
                                                                });
                                                            }
                                                        }))
                                                        .child("↓ Pull")
                                                )
                                                .child(
                                                    div().flex_1().py_1().bg(cx.theme().muted).hover(|s| s.bg(cx.theme().secondary)).rounded_sm().text_xs().flex().items_center().justify_center().cursor_pointer()
                                                        .on_mouse_down(MouseButton::Left, cx.listener(|_, _, _, cx| {
                                                            if cx.has_global::<crate::plugin_manager::PluginManagerGlobal>() {
                                                                let pm = cx.global::<crate::plugin_manager::PluginManagerGlobal>().0.clone();
                                                                pm.update(cx, |pm, _| {
                                                                    pm.dispatch_event(crate::plugin_manager::event::PluginEvent::CommandExecuted { command: "git.sync".to_string() });
                                                                });
                                                            }
                                                        }))
                                                        .child("⟳ Sync")
                                                )
                                        ).child(
                                            div()
                                                .w_full()
                                                .px_2()
                                                .py_1p5()
                                                .bg(cx.theme().muted.opacity(0.3))
                                                .border_1()
                                                .border_color(cx.theme().border)
                                                .rounded_md()
                                                .flex()
                                                .items_center()
                                                .justify_between()
                                                .overflow_hidden()
                                                .track_focus(&self.focus_handle)
                                                .cursor(CursorStyle::IBeam)
                                                .on_mouse_down(MouseButton::Left, cx.listener(|this, _, window, cx| {
                                                    this.focus_handle.focus(window, cx);
                                                }))
                                                .on_key_down(cx.listener(|this, event: &KeyDownEvent, _, cx| {
                                                    let key = &event.keystroke.key;
                                                    if key == "enter" && event.keystroke.modifiers.control {
                                                        let msg = if this.commit_message.is_empty() {
                                                            "Update".to_string()
                                                        } else {
                                                            this.commit_message.clone()
                                                        };
                                                        if cx.has_global::<crate::plugin_manager::PluginManagerGlobal>() {
                                                            let pm_global = cx.global::<crate::plugin_manager::PluginManagerGlobal>().0.clone();
                                                            pm_global.update(cx, |pm, _| {
                                                                pm.dispatch_event(crate::plugin_manager::event::PluginEvent::CommandExecuted {
                                                                    command: format!("git.commit:{}", msg),
                                                                });
                                                            });
                                                        }
                                                        this.commit_message.clear();
                                                        cx.notify();
                                                    } else if key == "backspace" {
                                                        this.commit_message.pop();
                                                        cx.notify();
                                                    } else if !event.keystroke.modifiers.control && !event.keystroke.modifiers.alt {
                                                        if key.len() == 1 {
                                                            this.commit_message.push_str(key);
                                                            cx.notify();
                                                        } else if key == "space" {
                                                            this.commit_message.push(' ');
                                                            cx.notify();
                                                        }
                                                    }
                                                }))
                                                .child(
                                                    div()
                                                        .text_xs()
                                                        .text_color(if self.commit_message.is_empty() { cx.theme().muted_foreground } else { cx.theme().foreground })
                                                        .overflow_hidden()
                                                        .child(if self.commit_message.is_empty() {
                                                            "Message (Ctrl+Enter to commit)...".to_string()
                                                        } else {
                                                            format!("{}_", self.commit_message)
                                                        })
                                                )
                                        ).child(
                                            div()
                                                .w_full()
                                                .py_1()
                                                .bg(gpui::rgb(0x007acc))
                                                .hover(|s| s.bg(gpui::rgb(0x0062a3)))
                                                .text_color(gpui::rgb(0xffffff))
                                                .rounded_md()
                                                .flex()
                                                .items_center()
                                                .justify_center()
                                                .cursor_pointer()
                                                .text_xs()
                                                .font_weight(FontWeight::SEMIBOLD)
                                                .overflow_hidden()
                                                .on_mouse_down(MouseButton::Left, cx.listener(|this, _, _, cx| {
                                                    let msg = if this.commit_message.is_empty() {
                                                        "Update".to_string()
                                                    } else {
                                                        this.commit_message.clone()
                                                    };
                                                    if cx.has_global::<crate::plugin_manager::PluginManagerGlobal>() {
                                                        let pm_global = cx.global::<crate::plugin_manager::PluginManagerGlobal>().0.clone();
                                                        pm_global.update(cx, |pm, _| {
                                                            pm.dispatch_event(crate::plugin_manager::event::PluginEvent::CommandExecuted {
                                                                command: format!("git.commit:{}", msg),
                                                            });
                                                        });
                                                    }
                                                    this.commit_message.clear();
                                                    cx.notify();
                                                }))
                                                .child("Commit")
                                        )
                                    );

                                    // 2. Staged Changes セクション
                                    if let Some(staged) = staged_nodes {
                                        if !staged.is_empty() {
                                            sc_layout = sc_layout.child(
                                                Self::render_git_section("STAGED CHANGES", staged.len(), true, cx)
                                            );
                                            for node in staged {
                                                sc_layout = sc_layout.child(Self::render_git_file_row(node, true, cx));
                                            }
                                        }
                                    }

                                    // 3. Changes セクション
                                    let changes_count = changes_nodes.map(|n| n.len()).unwrap_or(0);
                                    sc_layout = sc_layout.child(
                                        Self::render_git_section("CHANGES", changes_count, false, cx)
                                    );
                                    if let Some(changes) = changes_nodes {
                                        for node in changes {
                                            sc_layout = sc_layout.child(Self::render_git_file_row(node, false, cx));
                                        }
                                    }

                                    sc_layout.into_any_element()
                                } else if t == "tree" {
                                    list.into_any_element()
                                } else {
                                    div().p_4().text_sm().text_color(cx.theme().foreground).child("Plugin View").into_any_element()
                                }
                            } else {
                                div().p_4().text_sm().text_color(cx.theme().foreground).child("Empty UI").into_any_element()
                            }
                        } else {
                            div().p_4().text_sm().text_color(cx.theme().foreground).child("Invalid UI").into_any_element()
                        };
                        
                        matched_plugin = Some(
                            div()
                                .flex()
                                .flex_col()
                                .size_full()
                                .child(
                                    div().px_4().py_2().flex().items_center().justify_between().border_b_1().border_color(cx.theme().border)
                                        .child(div().text_xs().font_weight(FontWeight::BOLD).text_color(cx.theme().muted_foreground).child(title.to_uppercase()))
                                        .child(
                                            div().flex().gap_1().text_xs().text_color(cx.theme().muted_foreground)
                                                .child("•••")
                                        )
                                )
                                .child(content)
                                .into_any_element()
                        );
                        break;
                    }
                }
            }
            
            matched_plugin.unwrap_or_else(|| {
                div().p_4().text_sm().text_color(cx.theme().muted_foreground).child(format!("Sidebar '{}' not found", self.active_panel)).into_any_element()
            })
        };

        div()
            .w_full()
            .h_full()
            .bg(cx.theme().background)
            .border_r_1()
            .border_color(cx.theme().border)
            .flex()
            .flex_col()
            .child(active_content)
    }
}

impl LeftSidebar {
    /// 検索パネルの UI 描画
    fn render_search_panel(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme().clone();

        let mut results_tree = div().flex().flex_col().flex_1().overflow_hidden().p_2();

        if self.search_results.is_empty() {
            results_tree = results_tree.child(
                div().p_3().text_xs().text_color(theme.muted_foreground).child("No search results")
            );
        } else {
            let total_matches: usize = self.search_results.iter().map(|r| r.matches.len()).sum();
            results_tree = results_tree.child(
                div().px_2().py_1().text_xs().text_color(theme.muted_foreground)
                    .child(format!("{} results in {} files", total_matches, self.search_results.len()))
            );

            for res in &self.search_results {
                let file_path_str = res.file_path.to_string_lossy().to_string();
                let title = res.file_path.file_name().unwrap_or_default().to_string_lossy().to_string();
                let rel_path = res.relative_path.clone();
                let match_count = res.matches.len();

                // ファイル行
                let file_row = div()
                    .px_2()
                    .py_1()
                    .flex()
                    .items_center()
                    .justify_between()
                    .hover(|s| s.bg(theme.secondary))
                    .cursor_pointer()
                    .child(
                        div().flex().items_center().gap_1p5()
                            .child(Icon::new(IconName::File).size(gpui::px(13.0)).text_color(theme.muted_foreground))
                            .child(div().text_xs().font_weight(FontWeight::SEMIBOLD).text_color(theme.foreground).child(rel_path))
                    )
                    .child(
                        div().px_1p5().py_0p5().bg(theme.muted).rounded_sm().text_xs().text_color(theme.muted_foreground)
                            .child(format!("{}", match_count))
                    );

                results_tree = results_tree.child(file_row);

                // 各マッチ行
                for m in &res.matches {
                    let click_path = file_path_str.clone();
                    let line_text = m.line_text.trim().to_string();
                    let line_num = m.line_number;
                    let title_clone = title.clone();

                    let match_row = div()
                        .pl_5()
                        .pr_2()
                        .py_0p5()
                        .flex()
                        .items_center()
                        .gap_2()
                        .hover(|s| s.bg(theme.secondary))
                        .cursor_pointer()
                        .on_mouse_down(MouseButton::Left, cx.listener(move |_this, _, _, cx| {
                            if let Ok(content) = std::fs::read_to_string(&click_path) {
                                if cx.has_global::<crate::plugin_manager::PluginManagerGlobal>() {
                                    let pm_global = cx.global::<crate::plugin_manager::PluginManagerGlobal>().0.clone();
                                    pm_global.update(cx, |pm, _| {
                                        pm.dispatch_action(crate::plugin_manager::action::PluginAction::OpenTab {
                                            path: click_path.clone(),
                                            title: title_clone.clone(),
                                            content,
                                        });
                                    });
                                }
                            }
                        }))
                        .child(div().text_xs().text_color(theme.muted_foreground).child(format!("{}:", line_num)))
                        .child(div().text_xs().text_color(theme.foreground).child(line_text));

                    results_tree = results_tree.child(match_row);
                }
            }
        }

        div()
            .flex()
            .flex_col()
            .size_full()
            .child(
                div().px_4().py_2().flex().items_center().justify_between().border_b_1().border_color(theme.border)
                    .child(div().text_xs().font_weight(FontWeight::BOLD).text_color(theme.muted_foreground).child("SEARCH"))
                    .child(
                        div().flex().gap_1()
                            .child(
                                div().px_1p5().py_0p5().bg(theme.muted).rounded_sm().text_xs().cursor_pointer()
                                    .hover(|s| s.bg(theme.secondary))
                                    .on_mouse_down(MouseButton::Left, cx.listener(|this, _, _, cx| {
                                        this.execute_search(cx);
                                    }))
                                    .child("Find")
                            )
                            .child(
                                div().px_1p5().py_0p5().bg(theme.muted).rounded_sm().text_xs().cursor_pointer()
                                    .hover(|s| s.bg(theme.secondary))
                                    .on_mouse_down(MouseButton::Left, cx.listener(|this, _, _, cx| {
                                        this.execute_replace_all(cx);
                                    }))
                                    .child("Replace All")
                            )
                    )
            )
            .child(
                // 検索入力フォーム
                div().p_3().flex().flex_col().gap_2().border_b_1().border_color(theme.border)
                    .child(
                        div()
                            .px_2()
                            .py_1p5()
                            .bg(theme.muted.opacity(0.3))
                            .border_1()
                            .border_color(theme.border)
                            .rounded_md()
                            .flex()
                            .items_center()
                            .justify_between()
                            .track_focus(&self.focus_handle)
                            .cursor(CursorStyle::IBeam)
                            .on_mouse_down(MouseButton::Left, cx.listener(|this, _, window, cx| {
                                this.focus_handle.focus(window, cx);
                            }))
                            .on_key_down(cx.listener(|this, event: &KeyDownEvent, _, cx| {
                                let key = &event.keystroke.key;
                                if key == "enter" {
                                    this.execute_search(cx);
                                } else if key == "backspace" {
                                    this.search_query.pop();
                                    cx.notify();
                                } else if !event.keystroke.modifiers.control && !event.keystroke.modifiers.alt {
                                    if key.len() == 1 {
                                        this.search_query.push_str(key);
                                        cx.notify();
                                    } else if key == "space" {
                                        this.search_query.push(' ');
                                        cx.notify();
                                    }
                                }
                            }))
                            .child(
                                div().text_xs().text_color(if self.search_query.is_empty() { theme.muted_foreground } else { theme.foreground })
                                    .child(if self.search_query.is_empty() { "Search (type and press Enter)...".to_string() } else { format!("{}_", self.search_query) })
                            )
                            .child(
                                div()
                                    .px_1p5()
                                    .py_0p5()
                                    .rounded_sm()
                                    .text_xs()
                                    .font_weight(FontWeight::BOLD)
                                    .bg(if self.case_sensitive { gpui::rgb(0x007acc).into() } else { theme.muted })
                                    .text_color(if self.case_sensitive { gpui::rgb(0xffffff).into() } else { theme.muted_foreground })
                                    .hover(|s| s.bg(gpui::rgb(0x007acc)).text_color(gpui::rgb(0xffffff)))
                                    .cursor_pointer()
                                    .on_mouse_down(MouseButton::Left, cx.listener(|this, _, _, cx| {
                                        this.case_sensitive = !this.case_sensitive;
                                        this.execute_search(cx);
                                    }))
                                    .child("Aa")
                            )
                    )
                    .child(
                        div()
                            .px_2()
                            .py_1p5()
                            .bg(theme.muted.opacity(0.3))
                            .border_1()
                            .border_color(theme.border)
                            .rounded_md()
                            .child(div().text_xs().text_color(if self.replace_query.is_empty() { theme.muted_foreground } else { theme.foreground })
                                .child(if self.replace_query.is_empty() { "Replace...".to_string() } else { self.replace_query.clone() }))
                    )
            )
            .child(results_tree)
    }

    /// Git セクションヘッダー（STAGED CHANGES / CHANGES + 件数バッジ + 一括アクションボタン）の描画
    fn render_git_section(title: &'static str, count: usize, is_staged: bool, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme().clone();
        div()
            .px_3()
            .py_1()
            .flex()
            .items_center()
            .justify_between()
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_1()
                    .child(Icon::new(IconName::ChevronDown).size(gpui::px(12.0)).text_color(theme.muted_foreground))
                    .child(
                        div()
                            .text_xs()
                            .font_weight(FontWeight::BOLD)
                            .text_color(theme.muted_foreground)
                            .child(title)
                    )
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(
                        // 一括アクションボタン (Stage All / Unstage All / Discard All)
                        div()
                            .flex()
                            .items_center()
                            .gap_1()
                            .children(if is_staged {
                                Some(
                                    div()
                                        .px_1()
                                        .text_xs()
                                        .text_color(theme.muted_foreground)
                                        .hover(|s| s.bg(theme.muted).text_color(theme.foreground))
                                        .rounded_sm()
                                        .cursor_pointer()
                                        .on_mouse_down(MouseButton::Left, cx.listener(|_, _, _, cx| {
                                            cx.stop_propagation();
                                            if cx.has_global::<crate::plugin_manager::PluginManagerGlobal>() {
                                                let pm = cx.global::<crate::plugin_manager::PluginManagerGlobal>().0.clone();
                                                pm.update(cx, |pm, _| {
                                                    pm.dispatch_event(crate::plugin_manager::event::PluginEvent::CommandExecuted { command: "git.unstage_all".to_string() });
                                                });
                                            }
                                        }))
                                        .child("—")
                                )
                            } else {
                                None
                            })
                            .children(if !is_staged {
                                Some(
                                    div()
                                        .flex()
                                        .items_center()
                                        .gap_1()
                                        .child(
                                            div()
                                                .px_1()
                                                .text_xs()
                                                .text_color(theme.muted_foreground)
                                                .hover(|s| s.bg(theme.muted).text_color(theme.foreground))
                                                .rounded_sm()
                                                .cursor_pointer()
                                                .on_mouse_down(MouseButton::Left, cx.listener(|_, _, _, cx| {
                                                    cx.stop_propagation();
                                                    if cx.has_global::<crate::plugin_manager::PluginManagerGlobal>() {
                                                        let pm = cx.global::<crate::plugin_manager::PluginManagerGlobal>().0.clone();
                                                        pm.update(cx, |pm, _| {
                                                            pm.dispatch_event(crate::plugin_manager::event::PluginEvent::CommandExecuted { command: "git.discard_all".to_string() });
                                                        });
                                                    }
                                                }))
                                                .child("↺")
                                        )
                                        .child(
                                            div()
                                                .px_1()
                                                .text_xs()
                                                .text_color(theme.muted_foreground)
                                                .hover(|s| s.bg(theme.muted).text_color(theme.foreground))
                                                .rounded_sm()
                                                .cursor_pointer()
                                                .on_mouse_down(MouseButton::Left, cx.listener(|_, _, _, cx| {
                                                    cx.stop_propagation();
                                                    if cx.has_global::<crate::plugin_manager::PluginManagerGlobal>() {
                                                        let pm = cx.global::<crate::plugin_manager::PluginManagerGlobal>().0.clone();
                                                        pm.update(cx, |pm, _| {
                                                            pm.dispatch_event(crate::plugin_manager::event::PluginEvent::CommandExecuted { command: "git.stage_all".to_string() });
                                                        });
                                                    }
                                                }))
                                                .child("+")
                                        )
                                )
                            } else {
                                None
                            })
                    )
                    .child(
                        div()
                            .px_1p5()
                            .py_0p5()
                            .bg(theme.muted)
                            .text_color(theme.foreground)
                            .rounded_md()
                            .text_xs()
                            .child(format!("{}", count))
                    )
            )
    }

    /// VSCode 完全準拠: Git ファイル行の一行描画 (ファイル名 + ディレクトリ(途切れ可) + 絶対に消えないバッジ/ボタン)
    fn render_git_file_row(node: &serde_json::Value, is_staged: bool, cx: &mut Context<Self>) -> impl IntoElement {
        let name = node.get("name").and_then(|n| n.as_str()).unwrap_or("").to_string();
        let dir = node.get("dir").and_then(|d| d.as_str()).unwrap_or("").to_string();
        let path = node.get("path").and_then(|p| p.as_str()).unwrap_or("").to_string();
        let status = node.get("status").and_then(|s| s.as_str()).unwrap_or("").to_string();

        let status_color: gpui::Hsla = match status.as_str() {
            "M" | "MM" => gpui::rgb(0xeab308).into(),
            "U" | "??" | "A" => gpui::rgb(0x22c55e).into(),
            "D" => gpui::rgb(0xef4444).into(),
            _ => cx.theme().foreground,
        };

        let path_for_open = path.clone();
        let path_for_stage = path.clone();
        let path_for_discard = path.clone();

        div()
            .group("git-row")
            .w_full()
            .px_3()
            .py_1()
            .flex()
            .items_center()
            .justify_between()
            .hover(|s| s.bg(cx.theme().secondary))
            .cursor_pointer()
            .on_mouse_down(MouseButton::Left, cx.listener(move |_this, _, _, cx| {
                if !path_for_open.is_empty() {
                    if let Ok(content) = std::fs::read_to_string(&path_for_open) {
                        let title = Path::new(&path_for_open).file_name().unwrap_or_default().to_string_lossy().to_string();
                        if cx.has_global::<crate::plugin_manager::PluginManagerGlobal>() {
                            let pm = cx.global::<crate::plugin_manager::PluginManagerGlobal>().0.clone();
                            pm.update(cx, |pm, _| {
                                pm.dispatch_action(crate::plugin_manager::action::PluginAction::OpenTab {
                                    path: path_for_open.clone(),
                                    title,
                                    content,
                                });
                            });
                        }
                    }
                }
            }))
            .child(
                // 左側: ファイル名 + ディレクトリ（完全一行・ディレクトリは横幅が狭いとき末尾省略）
                div()
                    .flex_1()
                    .min_w_0()
                    .overflow_hidden()
                    .flex()
                    .items_baseline()
                    .gap_1p5()
                    .child(
                        div()
                            .text_xs()
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(status_color)
                            .flex_shrink_0()
                            .child(name)
                    )
                    .children(if !dir.is_empty() {
                        Some(
                            div()
                                .text_xs()
                                .text_color(cx.theme().muted_foreground.opacity(0.75))
                                .overflow_hidden()
                                .whitespace_nowrap()
                                .text_ellipsis()
                                .child(dir)
                        )
                    } else {
                        None
                    })
            )
            .child(
                // 右側: ホバーアクション (+/- / ↺) + ステータスバッジ (flex_shrink_0 で絶対に押し出されない・消えない)
                div()
                    .flex_shrink_0()
                    .flex()
                    .gap_2()
                    .items_center()
                    .child(
                        div()
                            .flex()
                            .gap_1()
                            .items_center()
                            .child(
                                div()
                                    .px_1()
                                    .text_xs()
                                    .text_color(cx.theme().muted_foreground)
                                    .hover(|s| s.bg(cx.theme().muted).text_color(cx.theme().foreground))
                                    .rounded_sm()
                                    .on_mouse_down(MouseButton::Left, cx.listener(move |_this, _, _, cx| {
                                        cx.stop_propagation();
                                        let cmd = if is_staged {
                                            format!("git.unstage:{}", path_for_stage)
                                        } else {
                                            format!("git.stage:{}", path_for_stage)
                                        };
                                        if cx.has_global::<crate::plugin_manager::PluginManagerGlobal>() {
                                            let pm = cx.global::<crate::plugin_manager::PluginManagerGlobal>().0.clone();
                                            pm.update(cx, |pm, _| {
                                                pm.dispatch_event(crate::plugin_manager::event::PluginEvent::CommandExecuted { command: cmd });
                                            });
                                        }
                                    }))
                                    .child(if is_staged { "—" } else { "+" })
                            )
                            .child(
                                div()
                                    .px_1()
                                    .text_xs()
                                    .text_color(cx.theme().muted_foreground)
                                    .hover(|s| s.bg(cx.theme().muted).text_color(cx.theme().foreground))
                                    .rounded_sm()
                                    .on_mouse_down(MouseButton::Left, cx.listener(move |_this, _, _, cx| {
                                        cx.stop_propagation();
                                        let cmd = format!("git.discard:{}", path_for_discard);
                                        if cx.has_global::<crate::plugin_manager::PluginManagerGlobal>() {
                                            let pm = cx.global::<crate::plugin_manager::PluginManagerGlobal>().0.clone();
                                            pm.update(cx, |pm, _| {
                                                pm.dispatch_event(crate::plugin_manager::event::PluginEvent::CommandExecuted { command: cmd });
                                            });
                                        }
                                    }))
                                    .child("↺")
                            )
                    )
                    .child(
                        div()
                            .w(gpui::px(16.0))
                            .text_xs()
                            .font_weight(FontWeight::BOLD)
                            .text_color(status_color)
                            .flex()
                            .justify_center()
                            .child(status)
                    )
            )
    }

    /// Git ステータスを一括で HashMap にキャッシュ構築 (O(N))
    fn build_git_status_map(cx: &App) -> std::collections::HashMap<String, String> {
        let mut map_res = std::collections::HashMap::new();
        if !cx.has_global::<crate::plugin_manager::PluginManagerGlobal>() {
            return map_res;
        }
        let pm = cx.global::<crate::plugin_manager::PluginManagerGlobal>().0.clone();
        let pm = pm.read(cx);
        let Some(git_panel) = pm.ui_registry.sidebar_items.iter().find(|i| i.id == "git_sidebar") else {
            return map_res;
        };

        let mut insert_nodes = |nodes: &Vec<serde_json::Value>| {
            for node in nodes {
                if let (Some(path), Some(status)) = (node.get("path").and_then(|p| p.as_str()), node.get("status").and_then(|s| s.as_str())) {
                    let norm = path.replace('\\', "/");
                    map_res.insert(norm, status.to_string());
                }
            }
        };

        if let serde_json::Value::Object(map) = &git_panel.ui_ast {
            if let Some(staged) = map.get("staged_nodes").and_then(|n| n.as_array()) {
                insert_nodes(staged);
            }
            if let Some(changes) = map.get("changes_nodes").and_then(|n| n.as_array()).or_else(|| map.get("nodes").and_then(|n| n.as_array())) {
                insert_nodes(changes);
            }
        }

        map_res
    }

    /// キャッシュされたマップからの高速ステータス照合
    fn lookup_git_status_from_map(
        map: &std::collections::HashMap<String, String>,
        path_str: &str,
        is_dir: bool,
    ) -> Option<String> {
        let norm_path_str = path_str.replace('\\', "/");

        if !is_dir {
            // 1. 完全一致
            if let Some(status) = map.get(&norm_path_str) {
                return Some(status.clone());
            }
            // 2. 末尾サフィックス一致 (絶対パス vs 相対パス)
            for (node_path, status) in map {
                if norm_path_str.ends_with(&format!("/{}", node_path.trim_start_matches('/')))
                    || node_path.ends_with(&format!("/{}", norm_path_str.trim_start_matches('/')))
                {
                    return Some(status.clone());
                }
            }
            None
        } else {
            let trimmed_file = norm_path_str.trim_end_matches('/');
            for (node_path, _) in map {
                let trimmed_git = node_path.trim_start_matches('/');
                if trimmed_git.starts_with(trimmed_file) {
                    return Some("M".to_string());
                }
                for part in trimmed_file.split('/') {
                    if !part.is_empty() && !part.ends_with(':') {
                        if let Some(pos) = trimmed_file.rfind(part) {
                            let suffix = &trimmed_file[pos..];
                            if trimmed_git.starts_with(&format!("{}/", suffix)) {
                                return Some("M".to_string());
                            }
                        }
                    }
                }
            }
            None
        }
    }
}
