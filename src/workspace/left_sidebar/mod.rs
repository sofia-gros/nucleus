use gpui::*;
use gpui_component::*;
use gpui_component::tree::{tree, TreeState, TreeItem};
use gpui_component::list::ListItem;
use gpui_component::theme::ActiveTheme;
use std::path::{Path, PathBuf};
use crate::file_system::{FileEntry, FileType};

pub struct LeftSidebar {
    root_path: Option<PathBuf>,
    pub tree_state: Option<Entity<TreeState>>,
    raw_entries: Option<Vec<FileEntry>>,
    pub active_panel: String,
}

impl LeftSidebar {
    pub fn new(root_path: Option<PathBuf>) -> Self {
        Self {
            root_path,
            tree_state: None,
            raw_entries: None,
            active_panel: "explorer".to_string(),
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
            } else if entry.file_type == FileType::Directory {
                // Leaf directory, but needs to be recognized as folder in GPUI Tree
                // GPUI Tree requires at least one child to be recognized as a folder via is_folder().
                // However, we just need the tree state to be able to expand it.
                // We don't add dummy items so it doesn't show "loading" forever.
            }
            item
        }).collect()
    }

    // (Old toggle_expand methods removed)
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
                        let icon = if !entry.is_folder() {
                            IconName::File
                        } else if entry.is_expanded() {
                            IconName::FolderOpen
                        } else {
                            IconName::Folder
                        };

                        let is_folder = entry.is_folder();
                        let path = item.id.clone();
                        let mut text_color = cx.theme().foreground;
                        let mut badge = None;

                        // Check git status
                        if cx.has_global::<crate::settings::SettingsGlobal>() {
                            let settings = cx.global::<crate::settings::SettingsGlobal>().0.read().unwrap();
                            if let Some(serde_json::Value::Object(git_stats)) = settings.get("git.status") {
                                let file_name = std::path::Path::new(path.as_ref()).file_name().unwrap_or_default().to_string_lossy().to_string();
                                for (git_path, status_val) in git_stats {
                                    if git_path.ends_with(&file_name) {
                                        let status = status_val.as_str().unwrap_or("");
                                        if status == "M" || status == "MM" {
                                            text_color = gpui::rgb(0xeab308).into();
                                            badge = Some(div().w_2().h_2().rounded_full().bg(gpui::rgb(0xeab308)).mr_2());
                                        } else if status == "??" || status == "A" {
                                            text_color = gpui::rgb(0x22c55e).into();
                                            badge = Some(div().text_xs().text_color(gpui::rgb(0x22c55e)).mr_2().child("U"));
                                        } else if status == "D" {
                                            text_color = gpui::rgb(0xef4444).into();
                                        }
                                        break;
                                    }
                                }
                            }
                        }

                        let mut list_item = ListItem::new(ix)
                            .w_full()
                            .rounded(cx.theme().radius)
                            .px_3()
                            .pl(gpui::px(16.) * entry.depth() as f32 + gpui::px(12.))
                            .child(
                                h_flex()
                                    .w_full()
                                    .justify_between()
                                    .child(
                                        h_flex()
                                            .gap_2()
                                            .child(Icon::new(icon).text_color(text_color))
                                            .child(div().text_sm().text_color(text_color).child(item.label.clone()))
                                    )
                                    .children(badge)
                            );

                        if !is_folder {
                            list_item = list_item.on_click(cx.listener({
                                move |_this, _, _window, cx| {
                                    if let Ok(content) = std::fs::read_to_string(path.as_ref()) {
                                        let title = Path::new(path.as_ref()).file_name().unwrap_or_default().to_string_lossy().to_string();
                                        if cx.has_global::<crate::plugin_manager::PluginManagerGlobal>() {
                                            let pm_global = cx.global::<crate::plugin_manager::PluginManagerGlobal>().0.clone();
                                            pm_global.update(cx, |pm, _cx| {
                                                pm.dispatch_action(crate::plugin_manager::action::PluginAction::OpenTab {
                                                    path: path.to_string(),
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
            div().p_2().text_sm().text_color(cx.theme().muted_foreground).child("Loading...").into_any_element()
        } else {
            div().p_2().text_sm().text_color(cx.theme().muted_foreground).child("No folder opened").into_any_element()
        };

        let active_content = if self.active_panel == "explorer" {
            div()
                .flex()
                .flex_col()
                .size_full()
                .child(
                    div().p_2().border_b_1().border_color(cx.theme().border)
                        .child(div().text_sm().font_weight(FontWeight::BOLD).text_color(cx.theme().muted_foreground).child("EXPLORER"))
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
                                if t == "text" {
                                    let val = map.get("value").and_then(|v| v.as_str()).unwrap_or("").to_string();
                                    div().p_2().text_sm().text_color(cx.theme().foreground).child(val).into_any_element()
                                } else if t == "source_control" || t == "tree" {
                                    let mut list_el = div().flex().flex_col().w_full();
                                    
                                    if t == "source_control" {
                                        // Source control specific UI (Input box)
                                        list_el = list_el.child(
                                            div().p_2().w_full().child(
                                                div().w_full().p_1().border_1().border_color(cx.theme().border).rounded_md()
                                                    .text_sm().text_color(cx.theme().muted_foreground)
                                                    .child("Message (Ctrl+Enter to commit)")
                                            )
                                        ).child(
                                            div().px_2().pb_2().w_full().child(
                                                div().w_full().p_1().bg(gpui::rgb(0x0e639c)).rounded_md().flex().justify_center()
                                                    .text_sm().text_color(gpui::rgb(0xffffff)).cursor_pointer()
                                                    .child("✓ Commit")
                                            )
                                        );
                                    }
                                    
                                    if let Some(serde_json::Value::Array(nodes)) = map.get("nodes") {
                                        for node in nodes {
                                            let label = node.get("label").and_then(|l| l.as_str()).unwrap_or("").to_string();
                                            let node_icon = node.get("icon").and_then(|i| i.as_str()).unwrap_or("");
                                            let icon_name = match node_icon {
                                                "file" => IconName::File,
                                                "folder" => IconName::Folder,
                                                "plus" => IconName::Plus,
                                                "minus" => IconName::Minus,
                                                "edit-2" => IconName::File, // Mapped to file for now, but color can change
                                                _ => IconName::File,
                                            };
                                            
                                            // Extract status from label like "src/main.rs (M)"
                                            let color: gpui::Hsla = if label.ends_with("(M)") {
                                                gpui::rgb(0xeab308).into() // Yellow
                                            } else if label.ends_with("(U)") || label.ends_with("(??)") {
                                                gpui::rgb(0x22c55e).into() // Green
                                            } else if label.ends_with("(D)") {
                                                gpui::rgb(0xef4444).into() // Red
                                            } else {
                                                cx.theme().foreground
                                            };
                                            
                                            list_el = list_el.child(
                                                h_flex()
                                                    .gap_2()
                                                    .px_2()
                                                    .py_1()
                                                    .w_full()
                                                    .hover(|s| s.bg(cx.theme().secondary))
                                                    .cursor_pointer()
                                                    .child(Icon::new(icon_name).text_color(color))
                                                    .child(div().text_sm().text_color(color).child(label))
                                            );
                                        }
                                    }
                                    list_el.into_any_element()
                                } else {
                                    div().p_2().text_sm().text_color(gpui::rgb(0xef4444)).child(format!("Unsupported UI type: {}", t)).into_any_element()
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
                                    div().p_2().border_b_1().border_color(cx.theme().border)
                                        .child(div().text_sm().font_weight(FontWeight::BOLD).text_color(cx.theme().muted_foreground).child(title))
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
