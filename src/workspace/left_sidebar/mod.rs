use gpui::*;
use gpui_component::*;
use std::path::{Path, PathBuf};
use crate::file_system::{FileEntry, FileType};

pub struct LeftSidebar {
    root_path: Option<PathBuf>,
    entries: Option<Vec<FileEntry>>,
}

impl LeftSidebar {
    pub fn new(root_path: Option<PathBuf>) -> Self {
        Self {
            root_path,
            entries: None,
        }
    }

    pub fn set_root(&mut self, path: Option<PathBuf>, cx: &mut Context<Self>) {
        self.root_path = path.clone();
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
                        this.entries = entries;
                        cx.notify();
                    });
                });
            }).detach();
        } else {
            self.entries = None;
            cx.notify();
        }
    }

    fn toggle_expand(&mut self, path: &Path, cx: &mut Context<Self>) {
        if let Some(entries) = &mut self.entries {
            if Self::toggle_in_tree(entries, path, cx) {
                cx.notify();
            }
        }
    }

    fn toggle_in_tree(entries: &mut Vec<FileEntry>, target: &Path, cx: &mut Context<Self>) -> bool {
        for entry in entries.iter_mut() {
            if entry.path == target {
                if entry.file_type == FileType::Directory {
                    entry.is_expanded = !entry.is_expanded;
                    if entry.is_expanded && entry.children.is_none() {
                        let path = entry.path.clone();
                        // Load children asynchronously
                        let executor = cx.background_executor().clone();
                        let path_clone = path.clone();
                        let window_cx = cx.to_async();
                        let this_weak = cx.weak_entity();
                        let app_cx: &mut gpui::App = cx;
                        
                        app_cx.spawn(|_cx: &mut gpui::AsyncApp| async move {
                            let children = executor.spawn(async move {
                                FileEntry::read_dir(&path_clone)
                            }).await;
                            
                            window_cx.update(|cx| {
                                let _ = this_weak.update(cx, |this: &mut Self, cx| {
                                    if let Some(entries) = &mut this.entries {
                                        Self::set_children(entries, &path, children);
                                        cx.notify();
                                    }
                                });
                            });
                        }).detach();
                    }
                }
                return true;
            } else if let Some(children) = &mut entry.children {
                if Self::toggle_in_tree(children, target, cx) {
                    return true;
                }
            }
        }
        false
    }

    fn set_children(entries: &mut Vec<FileEntry>, target: &Path, children: Option<Vec<FileEntry>>) -> bool {
        for entry in entries.iter_mut() {
            if entry.path == target {
                entry.children = children;
                return true;
            } else if let Some(c) = &mut entry.children {
                if Self::set_children(c, target, children.clone()) {
                    return true;
                }
            }
        }
        false
    }

    fn render_entry(&self, entry: &FileEntry, depth: usize, cx: &mut Context<Self>) -> impl IntoElement {
        let path = entry.path.clone();
        let is_dir = entry.file_type == FileType::Directory;
        let is_expanded = entry.is_expanded;

        let icon = if is_dir {
            if is_expanded { IconName::FolderOpen } else { IconName::Folder }
        } else {
            IconName::File
        };

        let mut item = div()
            .flex()
            .flex_row()
            .items_center()
            .pl(gpui::px(8.0 + depth as f32 * 12.0))
            .py(gpui::px(4.0))
            .hover(|s| s.bg(gpui::rgb(0x1e293b)))
            .cursor_pointer()
            .child(
                Icon::new(icon)
            )
            .child(
                div().ml(gpui::px(6.0)).text_sm().text_color(gpui::rgb(0xe2e8f0)).child(entry.name.clone())
            );

        if is_dir {
            item = item.on_mouse_down(MouseButton::Left, cx.listener(move |this, _, _, cx| {
                this.toggle_expand(&path, cx);
            }));
        } else {
            // For files, we would open them
            item = item.on_mouse_down(MouseButton::Left, cx.listener(move |_this, _, _, _cx| {
                println!("Opened file: {:?}", path);
            }));
        }

        let mut container = div().flex().flex_col().child(item);

        if is_expanded {
            if let Some(children) = &entry.children {
                for child in children {
                    container = container.child(self.render_entry(child, depth + 1, cx));
                }
            }
        }

        container
    }
}

impl Render for LeftSidebar {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let mut list = div().flex().flex_col().w_full();
        
        if let Some(entries) = &self.entries {
            for entry in entries {
                list = list.child(self.render_entry(entry, 0, cx));
            }
        } else if self.root_path.is_some() {
            list = list.child(div().p_2().text_sm().text_color(gpui::rgb(0x64748b)).child("Loading..."));
        } else {
            list = list.child(div().p_2().text_sm().text_color(gpui::rgb(0x64748b)).child("No folder opened"));
        }

        div()
            .size_full()
            .bg(gpui::rgb(0x0f172a))
            .border_r_1()
            .border_color(gpui::rgb(0x1e293b))
            .child(
                div().p_2().border_b_1().border_color(gpui::rgb(0x1e293b))
                    .child(div().text_sm().font_weight(FontWeight::BOLD).text_color(gpui::rgb(0x94a3b8)).child("EXPLORER"))
            )
            .child(list)
    }
}
