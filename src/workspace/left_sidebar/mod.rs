/// エクスプローラーおよびプラグインサイドバーのUI描画コンポーネント (VSCode 精密再現版)

use gpui::*;
use gpui_component::*;
use gpui_component::tree::{tree, TreeState, TreeItem};
use gpui_component::list::ListItem;
use gpui_component::theme::ActiveTheme;
use std::path::{Path, PathBuf};
use crate::file_system::FileEntry;

pub struct LeftSidebar {
    root_path: Option<PathBuf>,
    pub tree_state: Option<Entity<TreeState>>,
    raw_entries: Option<Vec<FileEntry>>,
    pub active_panel: String,
    pub commit_message: String,
}

impl LeftSidebar {
    pub fn new(root_path: Option<PathBuf>) -> Self {
        Self {
            root_path,
            tree_state: None,
            raw_entries: None,
            active_panel: "explorer".to_string(),
            commit_message: String::new(),
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
        } else {
            self.raw_entries = None;
            if let Some(ts) = &self.tree_state {
                ts.update(cx, |state, cx| state.set_items(vec![], cx));
            }
            cx.notify();
        }
    }

    fn convert_to_tree_items(entries: &[FileEntry]) -> Vec<TreeItem> {
        entries.iter().map(|entry| {
            let mut item = TreeItem::new(entry.path.to_string_lossy().to_string(), entry.name.clone());
            if let Some(children) = &entry.children {
                item = item.children(Self::convert_to_tree_items(children));
            }
            item
        }).collect()
    }
}

impl Render for LeftSidebar {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let list = if let Some(tree_state) = &self.tree_state {
            let view = cx.entity().clone();
            tree(
                tree_state,
                move |ix, entry, _selected, _window, cx| {
                    view.update(cx, |_, cx| {
                        let item = entry.item();
                        let is_folder = entry.is_folder();
                        let path_str = item.id.to_string();
                        let file_name = std::path::Path::new(&path_str).file_name().unwrap_or_default().to_string_lossy().to_string();

                        let icon = if !is_folder {
                            IconName::File
                        } else if entry.is_expanded() {
                            IconName::FolderOpen
                        } else {
                            IconName::Folder
                        };

                        let mut text_color = cx.theme().foreground;
                        let mut status_badge = None;

                        // Git ステータスの照合
                        if cx.has_global::<crate::settings::SettingsGlobal>() {
                            let settings = cx.global::<crate::settings::SettingsGlobal>().0.read().unwrap();
                            if let Some(serde_json::Value::Object(git_stats)) = settings.get("git.status") {
                                if is_folder {
                                    // フォルダの場合: 配下に Git 変更があれば丸ドット ● を表示
                                    let mut folder_has_changes = false;
                                    for (git_path, _) in git_stats {
                                        if git_path.contains(&file_name) {
                                            folder_has_changes = true;
                                            break;
                                        }
                                    }
                                    if folder_has_changes {
                                        status_badge = Some(
                                            div()
                                                .w_2()
                                                .h_2()
                                                .rounded_full()
                                                .bg(gpui::rgb(0xd97706)) // 茶/黄色の丸バッジ
                                                .mr_2()
                                        );
                                    }
                                } else {
                                    // ファイルの場合: ステータス文字 M / U / D を表示しファイル名も着色
                                    for (git_path, status_val) in git_stats {
                                        if git_path.ends_with(&file_name) {
                                            let status = status_val.as_str().unwrap_or("");
                                            if status.contains('M') {
                                                text_color = gpui::rgb(0xeab308).into(); // 黄色
                                                status_badge = Some(div().text_xs().font_weight(FontWeight::BOLD).text_color(gpui::rgb(0xeab308)).mr_2().child("M"));
                                            } else if status == "U" || status == "??" || status == "A" {
                                                text_color = gpui::rgb(0x22c55e).into(); // 緑色
                                                status_badge = Some(div().text_xs().font_weight(FontWeight::BOLD).text_color(gpui::rgb(0x22c55e)).mr_2().child("U"));
                                            } else if status.contains('D') {
                                                text_color = gpui::rgb(0xef4444).into(); // 赤色
                                                status_badge = Some(div().text_xs().font_weight(FontWeight::BOLD).text_color(gpui::rgb(0xef4444)).mr_2().child("D"));
                                            }
                                            break;
                                        }
                                    }
                                }
                            }
                        }

                        let mut list_item = ListItem::new(ix)
                            .w_full()
                            .rounded(cx.theme().radius)
                            .px_2()
                            .pl(gpui::px(14.) * entry.depth() as f32 + gpui::px(8.))
                            .child(
                                h_flex()
                                    .w_full()
                                    .justify_between()
                                    .child(
                                        h_flex()
                                            .gap_1p5()
                                            .child(Icon::new(icon).text_color(if is_folder { cx.theme().muted_foreground } else { text_color }))
                                            .child(div().text_sm().text_color(text_color).child(item.label.clone()))
                                    )
                                    .children(status_badge)
                            );

                        if !is_folder {
                            let click_path = path_str.clone();
                            list_item = list_item.on_click(cx.listener({
                                move |_this, _, _window, cx| {
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
                                }
                            }));
                        }
                        list_item
                    })
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
        } else {
            let mut matched_plugin = None;
            if cx.has_global::<crate::plugin_manager::PluginManagerGlobal>() {
                let pm_global = cx.global::<crate::plugin_manager::PluginManagerGlobal>().0.clone();
                let pm = pm_global.read(cx);
                
                for item in &pm.ui_registry.sidebar_items {
                    if item.id == self.active_panel {
                        let title = item.title.clone();
                        let ui_ast = item.ui_ast.clone();
                        
                        let content = if let serde_json::Value::Object(map) = ui_ast {
                            if let Some(t) = map.get("type").and_then(|t| t.as_str()) {
                                if t == "source_control" {
                                    let branch = map.get("branch").and_then(|b| b.as_str()).unwrap_or("main");
                                    let staged_nodes = map.get("staged_nodes").and_then(|n| n.as_array());
                                    let changes_nodes = map.get("changes_nodes").and_then(|n| n.as_array()).or_else(|| map.get("nodes").and_then(|n| n.as_array()));

                                    let mut sc_layout = div().flex().flex_col().size_full();

                                    // 1. コミット入力フォーム
                                    sc_layout = sc_layout.child(
                                        div().p_3().flex().flex_col().gap_2().child(
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
                                                .child(
                                                    div().text_xs().text_color(cx.theme().muted_foreground)
                                                        .child(format!("Message (Ctrl+Enter to commit on \"{}\")", branch))
                                                )
                                                .child(
                                                    div().px_1p5().py_0p5().bg(gpui::rgb(0x0e639c)).rounded_sm().text_xs().text_color(gpui::rgb(0xffffff))
                                                        .child("Generate")
                                                )
                                        ).child(
                                            div()
                                                .w_full()
                                                .py_1p5()
                                                .bg(gpui::rgb(0x007acc))
                                                .hover(|s| s.bg(gpui::rgb(0x0062a3)))
                                                .rounded_md()
                                                .flex()
                                                .items_center()
                                                .justify_center()
                                                .gap_1()
                                                .text_xs()
                                                .font_weight(FontWeight::SEMIBOLD)
                                                .text_color(gpui::rgb(0xffffff))
                                                .cursor_pointer()
                                                .on_mouse_down(MouseButton::Left, cx.listener(|_this, _, _, cx| {
                                                    if cx.has_global::<crate::plugin_manager::PluginManagerGlobal>() {
                                                        let pm = cx.global::<crate::plugin_manager::PluginManagerGlobal>().0.clone();
                                                        pm.update(cx, |pm, _| {
                                                            pm.dispatch_event(crate::plugin_manager::event::PluginEvent::CommandExecuted {
                                                                command: "git.commit".to_string(),
                                                            });
                                                        });
                                                    }
                                                }))
                                                .child("✓ Commit")
                                        )
                                    );

                                    // 2. Staged Changes セクション
                                    if let Some(nodes) = staged_nodes {
                                        if !nodes.is_empty() {
                                            sc_layout = sc_layout.child(Self::render_git_section_header("Staged Changes", nodes.len(), cx));
                                            for node in nodes {
                                                sc_layout = sc_layout.child(Self::render_git_file_row(node, true, cx));
                                            }
                                        }
                                    }

                                    // 3. Changes セクション
                                    let changes_count = changes_nodes.map(|n| n.len()).unwrap_or(0);
                                    sc_layout = sc_layout.child(Self::render_git_section_header("Changes", changes_count, cx));
                                    if let Some(nodes) = changes_nodes {
                                        for node in nodes {
                                            sc_layout = sc_layout.child(Self::render_git_file_row(node, false, cx));
                                        }
                                    }

                                    sc_layout.into_any_element()
                                } else {
                                    div().p_2().text_sm().text_color(cx.theme().foreground).child("Custom Plugin View").into_any_element()
                                }
                            } else {
                                div().p_2().text_sm().text_color(cx.theme().foreground).child("Empty Plugin UI").into_any_element()
                            }
                        } else {
                            div().p_2().text_sm().text_color(cx.theme().foreground).child("Invalid UI AST").into_any_element()
                        };

                        matched_plugin = Some(
                            div()
                                .size_full()
                                .flex()
                                .flex_col()
                                .child(
                                    div().px_4().py_2().border_b_1().border_color(cx.theme().border).flex().items_center().justify_between()
                                        .child(div().text_xs().font_weight(FontWeight::BOLD).text_color(cx.theme().muted_foreground).child(title))
                                        .child(
                                            div().flex().gap_2().text_xs().text_color(cx.theme().muted_foreground).cursor_pointer()
                                                .child("↺")
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
                div().p_4().text_sm().text_color(cx.theme().muted_foreground).child("Panel not found").into_any_element()
            })
        };

        div()
            .size_full()
            .bg(cx.theme().background)
            .border_r_1()
            .border_color(cx.theme().border)
            .flex()
            .flex_col()
            .child(active_content)
    }
}

impl LeftSidebar {
    /// Git セクションヘッダー（例: "v Changes  15"）の描画
    fn render_git_section_header(title: &'static str, count: usize, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .w_full()
            .px_3()
            .py_1()
            .flex()
            .items_center()
            .justify_between()
            .bg(cx.theme().muted.opacity(0.3))
            .child(
                h_flex()
                    .gap_1p5()
                    .child(div().text_xs().text_color(cx.theme().muted_foreground).child("v"))
                    .child(div().text_xs().font_weight(FontWeight::BOLD).text_color(cx.theme().foreground).child(title))
            )
            .child(
                div()
                    .px_1p5()
                    .py_0p5()
                    .rounded_full()
                    .bg(cx.theme().muted)
                    .text_xs()
                    .font_weight(FontWeight::BOLD)
                    .text_color(cx.theme().muted_foreground)
                    .child(format!("{}", count))
            )
    }

    /// Git ファイル行（ファイル名 + 親ディレクトリ + ホバーアクション群 + ステータス文字）の描画
    fn render_git_file_row(node: &serde_json::Value, is_staged: bool, cx: &mut Context<Self>) -> impl IntoElement {
        let name = node.get("name").and_then(|n| n.as_str()).unwrap_or("").to_string();
        let dir = node.get("dir").and_then(|d| d.as_str()).unwrap_or("").to_string();
        let path = node.get("path").and_then(|p| p.as_str()).unwrap_or("").to_string();
        let status = node.get("status").and_then(|s| s.as_str()).unwrap_or("").to_string();

        let status_color: gpui::Hsla = match status.as_str() {
            "M" | "MM" => gpui::rgb(0xeab308).into(), // 黄色
            "U" | "??" | "A" => gpui::rgb(0x22c55e).into(), // 緑色
            "D" => gpui::rgb(0xef4444).into(), // 赤色
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
                h_flex()
                    .gap_2()
                    .items_center()
                    .child(Icon::new(IconName::File).text_color(status_color))
                    .child(div().text_xs().text_color(status_color).child(name))
                    .children(if !dir.is_empty() {
                        Some(div().text_xs().text_color(cx.theme().muted_foreground).child(dir))
                    } else {
                        None
                    })
            )
            .child(
                h_flex()
                    .gap_2()
                    .items_center()
                    // ホバーアクションボタン群
                    .child(
                        h_flex()
                            .gap_1()
                            .items_center()
                            .child(
                                // Stage / Unstage (+) or (-)
                                div()
                                    .px_1()
                                    .rounded_sm()
                                    .text_xs()
                                    .text_color(cx.theme().muted_foreground)
                                    .hover(|s| s.bg(cx.theme().muted).text_color(cx.theme().foreground))
                                    .on_mouse_down(MouseButton::Left, cx.listener(move |_this, _, _, cx| {
                                        cx.stop_propagation();
                                        if cx.has_global::<crate::plugin_manager::PluginManagerGlobal>() {
                                            let pm = cx.global::<crate::plugin_manager::PluginManagerGlobal>().0.clone();
                                            let cmd = if is_staged {
                                                format!("git.unstage:{}", path_for_stage)
                                            } else {
                                                format!("git.stage:{}", path_for_stage)
                                            };
                                            pm.update(cx, |pm, _| {
                                                pm.dispatch_event(crate::plugin_manager::event::PluginEvent::CommandExecuted { command: cmd });
                                            });
                                        }
                                    }))
                                    .child(if is_staged { "—" } else { "+" })
                            )
                            .child(
                                // Discard (↺)
                                div()
                                    .px_1()
                                    .rounded_sm()
                                    .text_xs()
                                    .text_color(cx.theme().muted_foreground)
                                    .hover(|s| s.bg(cx.theme().muted).text_color(cx.theme().foreground))
                                    .on_mouse_down(MouseButton::Left, cx.listener(move |_this, _, _, cx| {
                                        cx.stop_propagation();
                                        if cx.has_global::<crate::plugin_manager::PluginManagerGlobal>() {
                                            let pm = cx.global::<crate::plugin_manager::PluginManagerGlobal>().0.clone();
                                            pm.update(cx, |pm, _| {
                                                pm.dispatch_event(crate::plugin_manager::event::PluginEvent::CommandExecuted {
                                                    command: format!("git.discard:{}", path_for_discard),
                                                });
                                            });
                                        }
                                    }))
                                    .child("↺")
                            )
                    )
                    // 最右端のステータス文字 (M, U, D)
                    .child(
                        div()
                            .text_xs()
                            .font_weight(FontWeight::BOLD)
                            .text_color(status_color)
                            .child(status)
                    )
            )
    }
}
