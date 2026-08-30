/// ワークスペース統括コンポーネント (TitleBar, Sidebars, EditorArea, BottomPanel, CommandPalette, FileWatcher 統合)

use gpui::*;
use gpui_component::theme::ActiveTheme;

pub mod title_bar;
pub mod status_bar;
pub mod activity_bar;
pub mod left_sidebar;
pub mod right_sidebar;
pub mod bottom_panel;
pub mod editor_area;
pub mod state;
pub mod command_palette;
pub mod recovery;

use std::path::PathBuf;
use crate::plugin_manager::action::PluginAction;
use crate::file_system::watcher::{FileWatcher, FileWatchEvent};

use title_bar::TitleBar;
use status_bar::StatusBar;
use activity_bar::ActivityBar;
use left_sidebar::LeftSidebar;
use right_sidebar::RightSidebar;
use bottom_panel::BottomPanel;
use editor_area::EditorArea;
use command_palette::CommandPalette;
use state::WorkspaceState;
use gpui::{MouseDownEvent, MouseMoveEvent, MouseUpEvent, MouseButton};

actions!(
    workspace,
    [ToggleLeftSidebar, ToggleRightSidebar, ToggleBottomPanel, OpenFileFinder, OpenCommandPalette]
);

pub struct Workspace {
    pub focus_handle: FocusHandle,
    title_bar: Entity<TitleBar>,
    status_bar: Entity<StatusBar>,
    activity_bar: Entity<ActivityBar>,
    pub left_sidebar: Entity<LeftSidebar>,
    pub right_sidebar: Entity<RightSidebar>,
    pub bottom_panel: Entity<BottomPanel>,
    pub editor_area: Entity<EditorArea>,
    pub command_palette: Entity<CommandPalette>,
    pub file_watcher: Option<FileWatcher>,
    pub root_path: Option<PathBuf>,
    state: WorkspaceState,
    save_task: Option<Task<()>>,
    resizing_left: bool,
    resizing_right: Option<(f32, f32)>,
    resizing_bottom: Option<(f32, f32)>,
}

impl Workspace {
    pub fn new(root_path: Option<PathBuf>, cx: &mut Context<Self>) -> Self {
        let state = Self::load_state();
        let left_sidebar_entity = cx.new(|cx| LeftSidebar::new(root_path.clone(), cx));
        
        if let Some(p) = &root_path {
            left_sidebar_entity.update(cx, |sidebar, cx| {
                sidebar.set_root(Some(p.clone()), cx);
            });
        }

        // バックグラウンドファイルウォッチャーの起動
        let watcher = if let Some(ref root) = root_path {
            FileWatcher::watch(root).ok()
        } else {
            None
        };
        
        // 定期的なファイル変更ポーリングのバックグラウンドタスク起動 (500msデバウンス)
        if watcher.is_some() {
            let executor = cx.background_executor().clone();
            cx.spawn(|this_weak: WeakEntity<Self>, async_cx: &mut gpui::AsyncApp| {
                let async_cx = async_cx.clone();
                async move {
                    loop {
                        executor.timer(std::time::Duration::from_millis(500)).await;
                        let _ = async_cx.update(|cx| {
                            if let Some(entity) = this_weak.upgrade() {
                                entity.update(cx, |workspace, cx| {
                                    workspace.poll_file_watcher(cx);
                                });
                            }
                        });
                    }
                }
            }).detach();
        }

        Self {
            focus_handle: cx.focus_handle(),
            title_bar: cx.new(|_| TitleBar::new()),
            status_bar: cx.new(|_| StatusBar::new()),
            activity_bar: cx.new(|_| ActivityBar::new()),
            left_sidebar: left_sidebar_entity,
            right_sidebar: cx.new(|_| RightSidebar::new()),
            bottom_panel: cx.new(|cx| BottomPanel::new(cx)),
            editor_area: cx.new(|cx| EditorArea::new(cx)),
            command_palette: cx.new(|cx| CommandPalette::new(cx)),
            file_watcher: watcher,
            root_path,
            state,
            save_task: None,
            resizing_left: false,
            resizing_right: None,
            resizing_bottom: None,
        }
    }

    pub fn schedule_save(&mut self, cx: &mut Context<Self>) {
        let state_clone = self.state.clone();
        let executor = cx.background_executor().clone();
        self.save_task = Some(executor.clone().spawn(async move {
            executor.timer(std::time::Duration::from_millis(500)).await;
            Self::save_state_to_disk(&state_clone);
        }));
    }

    fn load_state() -> WorkspaceState {
        if let Ok(data) = std::fs::read_to_string(".nucleus/state.json") {
            if let Ok(state) = serde_json::from_str(&data) {
                return state;
            }
        }
        WorkspaceState::default()
    }

    fn save_state_to_disk(state: &WorkspaceState) {
        let _ = std::fs::create_dir_all(".nucleus");
        if let Ok(data) = serde_json::to_string_pretty(state) {
            let _ = std::fs::write(".nucleus/state.json", data);
        }
    }

    pub fn poll_file_watcher(&mut self, cx: &mut Context<Self>) {
        if let Some(ref watcher) = self.file_watcher {
            let mut modified = false;
            while let Ok(event) = watcher.event_rx.try_recv() {
                match event {
                    FileWatchEvent::Created(_) | FileWatchEvent::Removed(_) | FileWatchEvent::Modified(_) => {
                        modified = true;
                    }
                    _ => {}
                }
            }
            if modified {
                // ファイル変更検知時: エクスプローラーのリフレッシュと通知
                if let Some(ref p) = self.root_path {
                    let root_clone = p.clone();
                    self.left_sidebar.update(cx, |sidebar, cx| {
                        sidebar.set_root(Some(root_clone), cx);
                    });
                }
            }
        }
    }

    pub fn handle_action(&mut self, action: PluginAction, cx: &mut Context<Self>) {
        match action {
            PluginAction::OpenTab { path, title, content } => {
                self.editor_area.update(cx, |editor, cx| {
                    editor.open_tab(path, title, content, cx);
                });
            }
            PluginAction::CloseTab { title } => {
                self.editor_area.update(cx, |editor, cx| {
                    if let Some(pos) = editor.tabs.iter().position(|t| t.title == title) {
                        editor.close_tab(pos, cx);
                    }
                });
            }
            PluginAction::OpenPanel { id } => {
                if id == "git_sidebar" {
                    self.left_sidebar.update(cx, |ls, cx| {
                        ls.set_active_panel("git_sidebar".to_string(), cx);
                    });
                    self.state.left_sidebar_open = true;
                    self.schedule_save(cx);
                } else if id == "explorer" {
                    self.left_sidebar.update(cx, |ls, cx| {
                        ls.set_active_panel("explorer".to_string(), cx);
                    });
                    self.state.left_sidebar_open = true;
                    self.schedule_save(cx);
                } else if id == "search" {
                    self.left_sidebar.update(cx, |ls, cx| {
                        ls.set_active_panel("search".to_string(), cx);
                    });
                    self.state.left_sidebar_open = true;
                    self.schedule_save(cx);
                } else if id == "terminal" {
                    self.state.bottom_panel_open = true;
                    self.bottom_panel.update(cx, |bp, cx| {
                        bp.current_tab = "TERMINAL";
                        cx.notify();
                    });
                    self.schedule_save(cx);
                } else if id == "problems" {
                    self.state.bottom_panel_open = true;
                    self.bottom_panel.update(cx, |bp, cx| {
                        bp.current_tab = "PROBLEMS";
                        cx.notify();
                    });
                    self.schedule_save(cx);
                } else if id == "settings" {
                    self.editor_area.update(cx, |ea, cx| {
                        ea.open_settings(cx);
                    });
                }
                cx.notify();
            }
            PluginAction::ClosePanel { .. } => {}
            PluginAction::ShowNotification { message } => {
                self.bottom_panel.update(cx, |bp, cx| {
                    bp.write_log(format!("[Notification] {}", message), cx);
                });
            }
            PluginAction::TerminalClear => {
                self.bottom_panel.update(cx, |bp, cx| {
                    bp.logs.clear();
                    cx.notify();
                });
            }
            PluginAction::SaveActiveTab => {
                self.editor_area.update(cx, |ea, cx| {
                    ea.save_active_tab(cx);
                });
            }
            PluginAction::ToggleSidebar => {
                self.state.left_sidebar_open = !self.state.left_sidebar_open;
                self.schedule_save(cx);
                cx.notify();
            }
            PluginAction::ToggleTerminal => {
                self.state.bottom_panel_open = !self.state.bottom_panel_open;
                self.schedule_save(cx);
                cx.notify();
            }
            PluginAction::OpenFileFinder => {
                let root = self.root_path.clone();
                self.command_palette.update(cx, |cp, cx| {
                    cp.open_file_search(root.as_deref(), cx);
                });
            }
            PluginAction::OpenCommandPalette => {
                self.command_palette.update(cx, |cp, cx| {
                    cp.open_command_palette(cx);
                });
            }
            PluginAction::OpenKeybindings => {
                self.editor_area.update(cx, |ea, cx| {
                    ea.open_keybindings(cx);
                });
            }
            PluginAction::OpenSettings => {
                self.editor_area.update(cx, |ea, cx| {
                    ea.open_settings(cx);
                });
            }
            _ => {}
        }
    }

    fn render_left_sidebar(&self, cx: &mut Context<Self>) -> Option<impl IntoElement> {
        if !self.state.left_sidebar_open {
            return None;
        }
        Some(
            div().flex().flex_row()
                .child(
                    gpui::div()
                        .w(gpui::px(self.state.left_sidebar_width))
                        .child(self.left_sidebar.clone())
                )
                .child(
                    gpui::div()
                        .w(gpui::px(4.0))
                        .cursor_col_resize()
                        .hover(|s| s.bg(gpui::rgb(0x38bdf8)))
                        .on_mouse_down(MouseButton::Left, cx.listener(|workspace, _event: &MouseDownEvent, _window, cx| {
                            workspace.resizing_left = true;
                            cx.notify();
                        }))
                )
        )
    }

    fn render_bottom_panel(&self, cx: &mut Context<Self>) -> Option<impl IntoElement> {
        if !self.state.bottom_panel_open {
            return None;
        }
        Some(
            div().flex().flex_col()
                .child(
                    gpui::div()
                        .h(gpui::px(4.0))
                        .cursor_row_resize()
                        .hover(|s| s.bg(gpui::rgb(0x38bdf8)))
                        .on_mouse_down(MouseButton::Left, cx.listener(|workspace, event: &MouseDownEvent, _window, cx| {
                            workspace.resizing_bottom = Some((f32::from(event.position.y), workspace.state.bottom_panel_height));
                            cx.notify();
                        }))
                )
                .child(
                    gpui::div()
                        .h(gpui::px(self.state.bottom_panel_height))
                        .child(self.bottom_panel.clone())
                )
        )
    }

    fn render_right_sidebar(&self, cx: &mut Context<Self>) -> Option<impl IntoElement> {
        if !self.state.right_sidebar_open {
            return None;
        }
        Some(
            div().flex().flex_row()
                .child(
                    gpui::div()
                        .w(gpui::px(4.0))
                        .cursor_col_resize()
                        .hover(|s| s.bg(gpui::rgb(0x38bdf8)))
                        .on_mouse_down(MouseButton::Left, cx.listener(|workspace, event: &MouseDownEvent, _window, cx| {
                            workspace.resizing_right = Some((f32::from(event.position.x), workspace.state.right_sidebar_width));
                            cx.notify();
                        }))
                )
                .child(
                    gpui::div()
                        .w(gpui::px(self.state.right_sidebar_width))
                        .child(self.right_sidebar.clone())
                )
        )
    }
}

impl Render for Workspace {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let root_path_ref = self.root_path.clone();

        div()
            .track_focus(&self.focus_handle)
            .size_full()
            .bg(cx.theme().background)
            .flex()
            .flex_col()
            .on_mouse_move(cx.listener(|workspace, event: &MouseMoveEvent, _window, cx| {
                let mut changed = false;
                if workspace.resizing_left {
                    workspace.state.left_sidebar_width = (f32::from(event.position.x) - 48.0).max(100.0).min(800.0);
                    changed = true;
                }
                if let Some((start_x, _)) = workspace.resizing_right.as_mut() {
                    let delta = *start_x - f32::from(event.position.x);
                    workspace.state.right_sidebar_width = (workspace.state.right_sidebar_width + delta).max(100.0).min(800.0);
                    *start_x = f32::from(event.position.x);
                    changed = true;
                }
                if let Some((start_y, _)) = workspace.resizing_bottom.as_mut() {
                    let delta = *start_y - f32::from(event.position.y);
                    workspace.state.bottom_panel_height = (workspace.state.bottom_panel_height + delta).max(100.0).min(600.0);
                    *start_y = f32::from(event.position.y);
                    changed = true;
                }
                if changed {
                    cx.notify();
                }
            }))
            .on_mouse_up(MouseButton::Left, cx.listener(|workspace, _: &MouseUpEvent, _window, cx| {
                if workspace.resizing_left || workspace.resizing_right.is_some() || workspace.resizing_bottom.is_some() {
                    workspace.resizing_left = false;
                    workspace.resizing_right = None;
                    workspace.resizing_bottom = None;
                    workspace.schedule_save(cx);
                    cx.notify();
                }
            }))
            .on_action(cx.listener(|workspace, _: &ToggleLeftSidebar, _window, cx| {
                workspace.state.left_sidebar_open = !workspace.state.left_sidebar_open;
                workspace.schedule_save(cx);
                cx.notify();
            }))
            .on_action(cx.listener(|workspace, _: &ToggleRightSidebar, _window, cx| {
                workspace.state.right_sidebar_open = !workspace.state.right_sidebar_open;
                workspace.schedule_save(cx);
                cx.notify();
            }))
            .on_action(cx.listener(|workspace, _: &ToggleBottomPanel, _window, cx| {
                workspace.state.bottom_panel_open = !workspace.state.bottom_panel_open;
                workspace.schedule_save(cx);
                cx.notify();
            }))
            .on_action(cx.listener(move |workspace, _: &OpenFileFinder, window, cx| {
                let rp = root_path_ref.clone();
                workspace.command_palette.update(cx, |pal, cx| {
                    pal.open_file_search(rp.as_deref(), cx);
                    pal.focus_handle.focus(window, cx);
                });
            }))
            .on_action(cx.listener(|workspace, _: &OpenCommandPalette, window, cx| {
                workspace.command_palette.update(cx, |pal, cx| {
                    pal.open_command_palette(cx);
                    pal.focus_handle.focus(window, cx);
                });
            }))
            .child(self.title_bar.clone())
            .child(
                div().flex_grow(1.).flex().flex_row()
                    .child(self.activity_bar.clone())
                    .children(self.render_left_sidebar(cx))
                    .child(
                        div().flex_1().overflow_hidden().flex().flex_col()
                            .child(
                                gpui::div().flex_1().overflow_hidden()
                                    .child(self.editor_area.clone())
                            )
                            .children(self.render_bottom_panel(cx))
                    )
                    .children(self.render_right_sidebar(cx))
            )
            .child(self.status_bar.clone())
            .child(self.command_palette.clone())
    }
}
