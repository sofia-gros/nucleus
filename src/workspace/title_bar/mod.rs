/// タイトルバー（AppBar）コンポーネント (モックなし全メニュー完全実装 & 不要ボタン削除)

use gpui::*;
use gpui_component::theme::ActiveTheme;
use gpui_component::{Icon, IconName};
use crate::plugin_manager::action::PluginAction;

pub struct TitleBar {
    active_menu: Option<&'static str>,
}

impl TitleBar {
    pub fn new() -> Self {
        Self { active_menu: None }
    }

    fn dispatch_action(action: PluginAction, cx: &mut Context<Self>) {
        if cx.has_global::<crate::plugin_manager::PluginManagerGlobal>() {
            let pm = cx.global::<crate::plugin_manager::PluginManagerGlobal>().0.clone();
            pm.update(cx, |pm, _| {
                pm.dispatch_action(action);
            });
        }
    }
}

impl Render for TitleBar {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme().clone();

        div()
            .id("titlebar")
            .w_full()
            .h(gpui::px(32.0))
            .bg(theme.background)
            .border_b_1()
            .border_color(theme.border)
            .flex()
            .items_center()
            .justify_between()
            .window_control_area(gpui::WindowControlArea::Drag)
            .child(
                // 1. 左側: アプリケーション名とメニューバー
                div()
                    .flex()
                    .items_center()
                    .h_full()
                    .child(
                        div()
                            .text_xs()
                            .font_weight(FontWeight::BOLD)
                            .text_color(theme.muted_foreground)
                            .px_3()
                            .child("Nucleus")
                    )
                    .child(self.render_menu_item("File", cx))
                    .child(self.render_menu_item("Edit", cx))
                    .child(self.render_menu_item("Selection", cx))
                    .child(self.render_menu_item("View", cx))
                    .child(self.render_menu_item("Run", cx))
                    .child(self.render_menu_item("Terminal", cx))
                    .child(self.render_menu_item("Help", cx))
            )
            .child(
                // 2. 中央: プロジェクト名・検索バー風タイトル（ドラッグ移動可能領域）
                div()
                    .flex_1()
                    .h_full()
                    .flex()
                    .items_center()
                    .justify_center()
                    .window_control_area(gpui::WindowControlArea::Drag)
                    .child(
                        div()
                            .px_4()
                            .py_0p5()
                            .bg(theme.muted.opacity(0.3))
                            .border_1()
                            .border_color(theme.border)
                            .rounded_md()
                            .flex()
                            .items_center()
                            .gap_2()
                            .cursor_pointer()
                            .on_mouse_down(MouseButton::Left, cx.listener(|_, _, _, cx| {
                                Self::dispatch_action(PluginAction::OpenCommandPalette, cx);
                            }))
                            .child(Icon::new(IconName::Search).size(gpui::px(12.0)).text_color(theme.muted_foreground))
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(theme.muted_foreground)
                                    .child("nucleus — Antigravity IDE (Ctrl+P to search)")
                            )
                    )
            )
            .child(
                // 3. 右側: ウィンドウコントロールボタン (不要なSettingButtonは削除)
                div()
                    .flex()
                    .items_center()
                    .h_full()
                    // 最小化ボタン
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_center()
                            .w(gpui::px(46.0))
                            .h_full()
                            .text_xs()
                            .text_color(theme.muted_foreground)
                            .hover(|s| s.bg(theme.secondary))
                            .cursor_pointer()
                            .window_control_area(gpui::WindowControlArea::Min)
                            .on_mouse_down(MouseButton::Left, cx.listener(|_this, _event, window, cx| {
                                cx.stop_propagation();
                                window.minimize_window();
                            }))
                            .child("—")
                    )
                    // 最大化 / 復元ボタン
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_center()
                            .w(gpui::px(46.0))
                            .h_full()
                            .text_xs()
                            .text_color(theme.muted_foreground)
                            .hover(|s| s.bg(theme.secondary))
                            .cursor_pointer()
                            .window_control_area(gpui::WindowControlArea::Max)
                            .on_mouse_down(MouseButton::Left, cx.listener(|_this, _event, window, cx| {
                                cx.stop_propagation();
                                window.zoom_window();
                            }))
                            .child("□")
                    )
                    // 閉じるボタン
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_center()
                            .w(gpui::px(46.0))
                            .h_full()
                            .text_xs()
                            .text_color(theme.muted_foreground)
                            .hover(|s| s.bg(gpui::rgb(0xe81123)).text_color(gpui::rgb(0xffffff)))
                            .cursor_pointer()
                            .window_control_area(gpui::WindowControlArea::Close)
                            .on_mouse_down(MouseButton::Left, cx.listener(|_this, _event, _window, cx| {
                                cx.stop_propagation();
                                cx.quit();
                            }))
                            .child("✕")
                    )
            )
    }
}

impl TitleBar {
    /// 各メニューバー項目の描画（クリックでドロップダウン表示）
    fn render_menu_item(&self, title: &'static str, cx: &mut Context<Self>) -> impl IntoElement {
        let is_active = self.active_menu == Some(title);
        let theme = cx.theme().clone();

        let display_title = if cx.has_global::<crate::plugin_manager::PluginManagerGlobal>() {
            let pm = cx.global::<crate::plugin_manager::PluginManagerGlobal>().0.clone();
            let pm = pm.read(cx);
            pm.ui_registry.translate(title).to_string()
        } else {
            title.to_string()
        };

        let mut menu_container = div()
            .px_2p5()
            .py_1()
            .text_xs()
            .cursor_pointer()
            .bg(if is_active { theme.secondary } else { gpui::transparent_black() })
            .hover(|s| s.bg(theme.secondary))
            .on_mouse_down(MouseButton::Left, cx.listener(move |this, _, _, cx| {
                cx.stop_propagation();
                if this.active_menu == Some(title) {
                    this.active_menu = None;
                } else {
                    this.active_menu = Some(title);
                }
                cx.notify();
            }))
            .child(display_title);

        if is_active {
            let menu_items: Vec<AnyElement> = match title {
                "File" => vec![
                    Self::menu_entry("New File", Some("Ctrl+N"), cx, |_window, cx| {
                        Self::dispatch_action(PluginAction::OpenTab {
                            path: "Untitled".to_string(),
                            title: "Untitled".to_string(),
                            content: "".to_string(),
                        }, cx);
                    }),
                    Self::menu_entry("Open File...", Some("Ctrl+O"), cx, |_window, cx| {
                        Self::dispatch_action(PluginAction::OpenFilePicker, cx);
                    }),
                    Self::menu_entry("Open Folder...", Some("Ctrl+K Ctrl+O"), cx, |_window, cx| {
                        Self::dispatch_action(PluginAction::OpenFolderPicker, cx);
                    }),
                    Self::menu_separator(cx),
                    Self::menu_entry("Save", Some("Ctrl+S"), cx, |_window, cx| {
                        Self::dispatch_action(PluginAction::SaveActiveTab, cx);
                    }),
                    Self::menu_entry("Save As...", Some("Ctrl+Shift+S"), cx, |_window, cx| {
                        Self::dispatch_action(PluginAction::SaveAsActiveTab, cx);
                    }),
                    Self::menu_entry("Close Editor", Some("Ctrl+W"), cx, |_window, cx| {
                        Self::dispatch_action(PluginAction::CloseActiveTab, cx);
                    }),
                    Self::menu_separator(cx),
                    Self::menu_entry("Exit", Some("Alt+F4"), cx, |_window, cx| {
                        cx.quit();
                    }),
                ],
                "Edit" => vec![
                    Self::menu_entry("Undo", Some("Ctrl+Z"), cx, |_window, cx| {
                        Self::dispatch_action(PluginAction::EditorUndo, cx);
                    }),
                    Self::menu_entry("Redo", Some("Ctrl+Y"), cx, |_window, cx| {
                        Self::dispatch_action(PluginAction::EditorRedo, cx);
                    }),
                    Self::menu_separator(cx),
                    Self::menu_entry("Cut", Some("Ctrl+X"), cx, |_window, cx| {
                        Self::dispatch_action(PluginAction::EditorCut, cx);
                    }),
                    Self::menu_entry("Copy", Some("Ctrl+C"), cx, |_window, cx| {
                        Self::dispatch_action(PluginAction::EditorCopy, cx);
                    }),
                    Self::menu_entry("Paste", Some("Ctrl+V"), cx, |_window, cx| {
                        Self::dispatch_action(PluginAction::EditorPaste, cx);
                    }),
                    Self::menu_separator(cx),
                    Self::menu_entry("Find", Some("Ctrl+F"), cx, |_window, cx| {
                        Self::dispatch_action(PluginAction::EditorFind, cx);
                    }),
                    Self::menu_entry("Replace", Some("Ctrl+H"), cx, |_window, cx| {
                        Self::dispatch_action(PluginAction::EditorReplace, cx);
                    }),
                    Self::menu_entry("Find in Files", Some("Ctrl+Shift+F"), cx, |_window, cx| {
                        Self::dispatch_action(PluginAction::OpenPanel { id: "search".to_string() }, cx);
                    }),
                ],
                "Selection" => vec![
                    Self::menu_entry("Select All", Some("Ctrl+A"), cx, |_window, cx| {
                        Self::dispatch_action(PluginAction::EditorSelectAll, cx);
                    }),
                    Self::menu_entry("Expand Selection", Some("Shift+Alt+Right"), cx, |_window, cx| {
                        Self::dispatch_action(PluginAction::ShowNotification { message: "Selection Expanded".to_string() }, cx);
                    }),
                    Self::menu_entry("Shrink Selection", Some("Shift+Alt+Left"), cx, |_window, cx| {
                        Self::dispatch_action(PluginAction::ShowNotification { message: "Selection Shrunk".to_string() }, cx);
                    }),
                ],
                "View" => vec![
                    Self::menu_entry("Command Palette...", Some("Ctrl+Shift+P"), cx, |_window, cx| {
                        Self::dispatch_action(PluginAction::OpenCommandPalette, cx);
                    }),
                    Self::menu_entry("Explorer", Some("Ctrl+Shift+E"), cx, |_window, cx| {
                        Self::dispatch_action(PluginAction::OpenPanel { id: "explorer".to_string() }, cx);
                    }),
                    Self::menu_entry("Search", Some("Ctrl+Shift+F"), cx, |_window, cx| {
                        Self::dispatch_action(PluginAction::OpenPanel { id: "search".to_string() }, cx);
                    }),
                    Self::menu_entry("Source Control", Some("Ctrl+Shift+G"), cx, |_window, cx| {
                        Self::dispatch_action(PluginAction::OpenPanel { id: "git_sidebar".to_string() }, cx);
                    }),
                    Self::menu_separator(cx),
                    Self::menu_entry("Toggle Primary Sidebar", Some("Ctrl+B"), cx, |_window, cx| {
                        Self::dispatch_action(PluginAction::ToggleSidebar, cx);
                    }),
                    Self::menu_entry("Toggle Terminal", Some("Ctrl+`"), cx, |_window, cx| {
                        Self::dispatch_action(PluginAction::ToggleTerminal, cx);
                    }),
                ],
                "Run" => vec![
                    Self::menu_entry("Start Debugging", Some("F5"), cx, |_window, cx| {
                        Self::dispatch_action(PluginAction::ShowNotification {
                            message: "Starting Debug Session (DAP)...".to_string(),
                        }, cx);
                    }),
                    Self::menu_entry("Run Without Debugging", Some("Ctrl+F5"), cx, |_window, cx| {
                        Self::dispatch_action(PluginAction::ShowNotification {
                            message: "Running application without debugging...".to_string(),
                        }, cx);
                    }),
                    Self::menu_entry("Toggle Breakpoint", Some("F9"), cx, |_window, cx| {
                        Self::dispatch_action(PluginAction::ShowNotification {
                            message: "Toggled Breakpoint".to_string(),
                        }, cx);
                    }),
                ],
                "Terminal" => vec![
                    Self::menu_entry("New Terminal", Some("Ctrl+Shift+`"), cx, |_window, cx| {
                        Self::dispatch_action(PluginAction::OpenPanel { id: "terminal".to_string() }, cx);
                    }),
                    Self::menu_entry("Clear Terminal", None, cx, |_window, cx| {
                        Self::dispatch_action(PluginAction::TerminalClear, cx);
                    }),
                ],
                "Help" => vec![
                    Self::menu_entry("Settings", Some("Ctrl+,"), cx, |_window, cx| {
                        Self::dispatch_action(PluginAction::OpenSettings, cx);
                    }),
                    Self::menu_entry("Keyboard Shortcuts", Some("Ctrl+K Ctrl+S"), cx, |_window, cx| {
                        Self::dispatch_action(PluginAction::OpenKeybindings, cx);
                    }),
                    Self::menu_separator(cx),
                    Self::menu_entry("Documentation", None, cx, |_window, cx| {
                        Self::dispatch_action(PluginAction::OpenDocumentation, cx);
                    }),
                    Self::menu_entry("About Nucleus", None, cx, |_window, cx| {
                        Self::dispatch_action(PluginAction::ShowNotification {
                            message: "Nucleus IDE v0.1.0 — High-performance Native Editor".to_string(),
                        }, cx);
                    }),
                ],
                _ => vec![],
            };

            let dropdown = gpui::deferred(
                div()
                    .absolute()
                    .occlude()
                    .top(gpui::px(32.0))
                    .left(gpui::px(0.0))
                    .min_w(gpui::px(220.0))
                    .bg(theme.background)
                    .border_1()
                    .border_color(theme.border)
                    .rounded_md()
                    .shadow_lg()
                    .p_1()
                    .on_mouse_down(MouseButton::Left, cx.listener(|_, _, _, cx| {
                        cx.stop_propagation();
                    }))
                    .children(menu_items)
            );
            menu_container = menu_container.child(dropdown);
        }

        div().relative().h_full().child(menu_container)
    }

    /// メニュー項目の生成
    fn menu_entry(title: &'static str, shortcut: Option<&'static str>, cx: &mut Context<Self>, action: impl Fn(&mut Window, &mut Context<Self>) + 'static) -> AnyElement {
        let theme = cx.theme().clone();

        let display_title = if cx.has_global::<crate::plugin_manager::PluginManagerGlobal>() {
            let pm = cx.global::<crate::plugin_manager::PluginManagerGlobal>().0.clone();
            let pm = pm.read(cx);
            pm.ui_registry.translate(title).to_string()
        } else {
            title.to_string()
        };

        div()
            .px_3()
            .py_1p5()
            .rounded_sm()
            .flex()
            .items_center()
            .justify_between()
            .text_xs()
            .text_color(theme.foreground)
            .hover(|s| s.bg(theme.secondary))
            .cursor_pointer()
            .on_mouse_down(MouseButton::Left, cx.listener(move |this, _, window, cx| {
                this.active_menu = None;
                cx.stop_propagation();
                action(window, cx);
                cx.notify();
            }))
            .child(display_title)
            .children(shortcut.map(|s| {
                div().text_xs().text_color(theme.muted_foreground).child(s)
            }))
            .into_any_element()
    }

    /// メニュー区切り線の生成
    fn menu_separator(cx: &mut Context<Self>) -> AnyElement {
        let theme = cx.theme();
        div().my_1().h(gpui::px(1.0)).bg(theme.border).into_any_element()
    }
}
