use gpui::*;

pub mod title_bar;
pub mod status_bar;
pub mod activity_bar;
pub mod left_sidebar;
pub mod right_sidebar;
pub mod bottom_panel;
pub mod editor_area;
pub mod state;

use std::sync::mpsc::Receiver;
use crate::plugin_manager::{action::PluginAction, event::PluginEvent, PluginManagerGlobal};

use title_bar::TitleBar;
use status_bar::StatusBar;
use activity_bar::ActivityBar;
use left_sidebar::LeftSidebar;
use right_sidebar::RightSidebar;
use bottom_panel::BottomPanel;
use editor_area::EditorArea;
use state::WorkspaceState;
use gpui::{MouseDownEvent, MouseMoveEvent, MouseUpEvent, MouseButton};

actions!(
    workspace,
    [ToggleLeftSidebar, ToggleRightSidebar, ToggleBottomPanel]
);

pub struct Workspace {
    pub focus_handle: FocusHandle,
    title_bar: Entity<TitleBar>,
    status_bar: Entity<StatusBar>,
    activity_bar: Entity<ActivityBar>,
    left_sidebar: Entity<LeftSidebar>,
    right_sidebar: Entity<RightSidebar>,
    bottom_panel: Entity<BottomPanel>,
    editor_area: Entity<EditorArea>,
    state: WorkspaceState,
    save_task: Option<Task<()>>,
    resizing_left: bool,
    resizing_right: Option<(f32, f32)>,
    resizing_bottom: Option<(f32, f32)>,
}

impl Workspace {
    pub fn new(cx: &mut Context<Self>) -> Self {
        let state = Self::load_state();
        Self {
            focus_handle: cx.focus_handle(),
            title_bar: cx.new(|_| TitleBar::new()),
            status_bar: cx.new(|_| StatusBar::new()),
            activity_bar: cx.new(|_| ActivityBar::new()),
            left_sidebar: cx.new(|_| LeftSidebar::new()),
            right_sidebar: cx.new(|_| RightSidebar::new()),
            bottom_panel: cx.new(|_| BottomPanel::new()),
            editor_area: cx.new(|_| EditorArea::new()),
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
            // Wait 500ms for debounce
            executor.timer(std::time::Duration::from_millis(500)).await;
            
            let path = Self::state_path();
            if let Some(parent) = path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            if let Ok(json_str) = serde_json::to_string_pretty(&state_clone) {
                let _ = std::fs::write(path, json_str);
            }
        }));
    }

    fn state_path() -> std::path::PathBuf {
        std::path::PathBuf::from(".nucleus").join("state.json")
    }

    fn load_state() -> WorkspaceState {
        let path = Self::state_path();
        if let Ok(content) = std::fs::read_to_string(&path) {
            if let Ok(state) = serde_json::from_str(&content) {
                return state;
            }
        }
        WorkspaceState::default()
    }

    pub fn save_state(&self) {
        let path = Self::state_path();
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Ok(json_str) = serde_json::to_string_pretty(&self.state) {
            let _ = std::fs::write(path, json_str);
        }
    }

    pub fn handle_action(&mut self, action: PluginAction, cx: &mut Context<Self>) {
        match action {
            PluginAction::OpenTab { title, content } => {
                self.editor_area.update(cx, |editor, cx| {
                    editor.open_tab(title.clone(), content, cx);
                });
                
                if cx.has_global::<PluginManagerGlobal>() {
                    let pm_entity = cx.global::<PluginManagerGlobal>().0.clone();
                    pm_entity.update(cx, |pm, _cx| {
                        pm.dispatch_event(PluginEvent::FileOpened { path: title });
                    });
                }
            }
            PluginAction::ShowNotification { message } => {
                println!("UI Notification: {}", message);
            }
            PluginAction::OpenPanel { id } => {
                println!("UI Action: OpenPanel {}", id);
            }
            PluginAction::UpdateSetting { key, value } => {
                if cx.has_global::<crate::settings::SettingsGlobal>() {
                    let store = cx.global::<crate::settings::SettingsGlobal>().0.clone();
                    store.write().unwrap().set(&key, value);
                    println!("UI Action: Updated Setting");
                }
            }
            PluginAction::TerminalWrite { text } => {
                self.bottom_panel.update(cx, |panel, cx| {
                    panel.write_log(text, cx);
                });
            }
            PluginAction::TerminalClear => {
                self.bottom_panel.update(cx, |panel, cx| {
                    panel.logs.clear();
                    cx.notify();
                });
            }
            _ => {}
        }
    }
}

impl Workspace {
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
                        .on_mouse_down(MouseButton::Left, cx.listener(|workspace, _: &MouseDownEvent, _window, cx| {
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
        div()
            .track_focus(&self.focus_handle)
            .size_full()
            .bg(gpui::rgb(0x0f172a)) // Slate 900
            .flex()
            .flex_col()
            .on_mouse_move(cx.listener(|workspace, event: &MouseMoveEvent, _window, cx| {
                let mut changed = false;
                if workspace.resizing_left {
                    workspace.state.left_sidebar_width = (f32::from(event.position.x) - 48.0).max(100.0).min(800.0);
                    changed = true;
                }
                if let Some((start_x, start_w)) = workspace.resizing_right {
                    let delta = start_x - f32::from(event.position.x);
                    workspace.state.right_sidebar_width = (start_w + delta).max(100.0).min(800.0);
                    changed = true;
                }
                if let Some((start_y, start_h)) = workspace.resizing_bottom {
                    let delta = start_y - f32::from(event.position.y);
                    workspace.state.bottom_panel_height = (start_h + delta).max(100.0).min(600.0);
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
            .child(self.title_bar.clone())
            .child(
                div().flex_grow(1.).flex().flex_row()
                    .child(self.activity_bar.clone())
                    .children(self.render_left_sidebar(cx))
                    .child(
                        div().flex_grow(1.).flex().flex_col()
                            .child(
                                gpui::div().flex_grow(1.)
                                    .child(self.editor_area.clone())
                            )
                            .children(self.render_bottom_panel(cx))
                    )
                    .children(self.render_right_sidebar(cx))
            )
            .child(self.status_bar.clone())
    }
}
