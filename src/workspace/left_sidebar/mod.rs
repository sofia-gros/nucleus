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
}

impl LeftSidebar {
    pub fn new(root_path: Option<PathBuf>) -> Self {
        Self {
            root_path,
            tree_state: None,
            raw_entries: None,
        }
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
                        let mut list_item = ListItem::new(ix)
                            .w_full()
                            .rounded(cx.theme().radius)
                            .px_3()
                            .pl(gpui::px(16.) * entry.depth() as f32 + gpui::px(12.))
                            .child(
                                h_flex()
                                    .gap_2()
                                    .child(Icon::new(icon))
                                    .child(div().text_sm().text_color(cx.theme().foreground).child(item.label.clone())),
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

        let mut plugin_items = Vec::new();
        if cx.has_global::<crate::plugin_manager::PluginManagerGlobal>() {
            let pm_global = cx.global::<crate::plugin_manager::PluginManagerGlobal>().0.clone();
            let pm = pm_global.read(cx);
            
            for item in &pm.ui_registry.sidebar_items {
                let title = item.title.clone();
                let ui_ast = item.ui_ast.clone();
                
                let content = if let serde_json::Value::Object(map) = ui_ast {
                    if let Some(t) = map.get("type").and_then(|t| t.as_str()) {
                        if t == "text" {
                            let val = map.get("value").and_then(|v| v.as_str()).unwrap_or("").to_string();
                            div().p_2().text_sm().text_color(cx.theme().foreground).child(val).into_any_element()
                        } else {
                            div().p_2().text_sm().text_color(gpui::rgb(0xef4444)).child(format!("Unsupported UI type: {}", t)).into_any_element()
                        }
                    } else {
                        div().p_2().text_sm().text_color(cx.theme().foreground).child("Empty Plugin UI").into_any_element()
                    }
                } else {
                    div().p_2().text_sm().text_color(cx.theme().foreground).child("Invalid UI AST").into_any_element()
                };

                plugin_items.push(
                    div()
                        .w_full()
                        .flex()
                        .flex_col()
                        .border_b_1()
                        .border_color(cx.theme().border)
                        .child(
                            div().p_2().text_sm().font_weight(FontWeight::BOLD).text_color(cx.theme().muted_foreground).child(title)
                        )
                        .child(content)
                );
            }
        }

        div()
            .size_full()
            .bg(cx.theme().background)
            .border_r_1()
            .border_color(cx.theme().border)
            .flex()
            .flex_col()
            .child(
                div().p_2().border_b_1().border_color(cx.theme().border)
                    .child(div().text_sm().font_weight(FontWeight::BOLD).text_color(cx.theme().muted_foreground).child("EXPLORER"))
            )
            .child(list)
            .children(plugin_items)
    }
}
