/// タイトルバー（AppBar）コンポーネント (VSCode 完全準拠メニュー & OS ネイティブドラッグ対応)

use gpui::*;
use gpui_component::theme::ActiveTheme;
use gpui_component::{Icon, IconName};

pub struct TitleBar {
    active_menu: Option<&'static str>,
}

impl TitleBar {
    pub fn new() -> Self {
        Self { active_menu: None }
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
                            .child(Icon::new(IconName::Search).size(gpui::px(12.0)).text_color(theme.muted_foreground))
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(theme.muted_foreground)
                                    .child("nucleus — Antigravity IDE")
                            )
                    )
            )
            .child(
                // 3. 右側: 設定とウィンドウコントロールボタン
                div()
                    .flex()
                    .items_center()
                    .h_full()
                    // 設定アイコン
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_center()
                            .w(gpui::px(32.0))
                            .h_full()
                            .hover(|s| s.bg(theme.secondary))
                            .cursor_pointer()
                            .on_mouse_down(MouseButton::Left, cx.listener(|_this, _event, _window, cx| {
                                cx.stop_propagation();
                            }))
                            .child(Icon::new(IconName::Settings).size(gpui::px(14.0)).text_color(theme.muted_foreground))
                    )
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
    /// 各メニューバー項目（File, Edit...）の描画とドロップダウン生成
    fn render_menu_item(&self, label: &'static str, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme().clone();
        let is_active = self.active_menu == Some(label);
        
        let mut menu_container = div()
            .h_full()
            .flex()
            .items_center()
            .px_2p5()
            .text_xs()
            .text_color(if is_active { theme.foreground } else { theme.muted_foreground })
            .bg(if is_active { theme.secondary } else { theme.background })
            .hover(|s| s.bg(theme.secondary).text_color(theme.foreground))
            .cursor_pointer()
            .on_mouse_down(MouseButton::Left, cx.listener(move |this, _, _, cx| {
                if this.active_menu == Some(label) {
                    this.active_menu = None;
                } else {
                    this.active_menu = Some(label);
                }
                cx.stop_propagation();
                cx.notify();
            }))
            .child(label);

        if is_active {
            let menu_items = match label {
                "File" => vec![
                    Self::menu_entry("New File", Some("Ctrl+N"), cx, |_window, cx| {
                        if cx.has_global::<crate::plugin_manager::PluginManagerGlobal>() {
                            let pm = cx.global::<crate::plugin_manager::PluginManagerGlobal>().0.clone();
                            pm.update(cx, |pm, _| {
                                pm.dispatch_action(crate::plugin_manager::action::PluginAction::OpenTab {
                                    path: "Untitled".to_string(),
                                    title: "Untitled".to_string(),
                                    content: "".to_string(),
                                });
                            });
                        }
                    }),
                    Self::menu_entry("Open File...", Some("Ctrl+O"), cx, |_window, _cx| {}),
                    Self::menu_entry("Open Folder...", Some("Ctrl+K Ctrl+O"), cx, |_window, _cx| {}),
                    Self::menu_separator(cx),
                    Self::menu_entry("Save", Some("Ctrl+S"), cx, |_window, cx| {
                        // バッファ保存
                        cx.notify();
                    }),
                    Self::menu_entry("Save As...", Some("Ctrl+Shift+S"), cx, |_window, _cx| {}),
                    Self::menu_separator(cx),
                    Self::menu_entry("Exit", Some("Alt+F4"), cx, |_window, cx| {
                        cx.quit();
                    }),
                ],
                "Edit" => vec![
                    Self::menu_entry("Undo", Some("Ctrl+Z"), cx, |_window, _cx| {}),
                    Self::menu_entry("Redo", Some("Ctrl+Y"), cx, |_window, _cx| {}),
                    Self::menu_separator(cx),
                    Self::menu_entry("Cut", Some("Ctrl+X"), cx, |_window, _cx| {}),
                    Self::menu_entry("Copy", Some("Ctrl+C"), cx, |_window, _cx| {}),
                    Self::menu_entry("Paste", Some("Ctrl+V"), cx, |_window, _cx| {}),
                    Self::menu_separator(cx),
                    Self::menu_entry("Find", Some("Ctrl+F"), cx, |_window, _cx| {}),
                ],
                "Selection" => vec![
                    Self::menu_entry("Select All", Some("Ctrl+A"), cx, |_window, _cx| {}),
                    Self::menu_entry("Expand Selection", Some("Shift+Alt+Right"), cx, |_window, _cx| {}),
                    Self::menu_entry("Shrink Selection", Some("Shift+Alt+Left"), cx, |_window, _cx| {}),
                ],
                "View" => vec![
                    Self::menu_entry("Explorer", Some("Ctrl+Shift+E"), cx, |_window, cx| {
                        if cx.has_global::<crate::plugin_manager::PluginManagerGlobal>() {
                            let pm = cx.global::<crate::plugin_manager::PluginManagerGlobal>().0.clone();
                            pm.update(cx, |pm, _| {
                                pm.dispatch_action(crate::plugin_manager::action::PluginAction::OpenPanel { id: "explorer".to_string() });
                            });
                        }
                    }),
                    Self::menu_entry("Source Control", Some("Ctrl+Shift+G"), cx, |_window, cx| {
                        if cx.has_global::<crate::plugin_manager::PluginManagerGlobal>() {
                            let pm = cx.global::<crate::plugin_manager::PluginManagerGlobal>().0.clone();
                            pm.update(cx, |pm, _| {
                                pm.dispatch_action(crate::plugin_manager::action::PluginAction::OpenPanel { id: "git_sidebar".to_string() });
                            });
                        }
                    }),
                    Self::menu_separator(cx),
                    Self::menu_entry("Toggle Primary Sidebar", Some("Ctrl+B"), cx, |_window, _cx| {}),
                    Self::menu_entry("Toggle Terminal", Some("Ctrl+`"), cx, |_window, _cx| {}),
                ],
                "Run" => vec![
                    Self::menu_entry("Start Debugging", Some("F5"), cx, |_window, _cx| {}),
                    Self::menu_entry("Run Without Debugging", Some("Ctrl+F5"), cx, |_window, _cx| {}),
                    Self::menu_entry("Stop Debugging", Some("Shift+F5"), cx, |_window, _cx| {}),
                ],
                "Terminal" => vec![
                    Self::menu_entry("New Terminal", Some("Ctrl+Shift+`"), cx, |_window, cx| {
                        // ターミナル作成
                        if cx.has_global::<crate::plugin_manager::PluginManagerGlobal>() {
                            let pm = cx.global::<crate::plugin_manager::PluginManagerGlobal>().0.clone();
                            pm.update(cx, |pm, _| {
                                pm.dispatch_action(crate::plugin_manager::action::PluginAction::OpenPanel { id: "terminal".to_string() });
                            });
                        }
                    }),
                    Self::menu_entry("Clear Terminal", None, cx, |_window, cx| {
                        if cx.has_global::<crate::plugin_manager::PluginManagerGlobal>() {
                            let pm = cx.global::<crate::plugin_manager::PluginManagerGlobal>().0.clone();
                            pm.update(cx, |pm, _| {
                                pm.dispatch_action(crate::plugin_manager::action::PluginAction::TerminalClear);
                            });
                        }
                    }),
                ],
                "Help" => vec![
                    Self::menu_entry("Documentation", None, cx, |_window, _cx| {}),
                    Self::menu_entry("About Nucleus", None, cx, |_window, cx| {
                        if cx.has_global::<crate::plugin_manager::PluginManagerGlobal>() {
                            let pm = cx.global::<crate::plugin_manager::PluginManagerGlobal>().0.clone();
                            pm.update(cx, |pm, _| {
                                pm.dispatch_action(crate::plugin_manager::action::PluginAction::ShowNotification {
                                    message: "Nucleus IDE v0.1.0 — High-performance Rust & WASM IDE".to_string(),
                                });
                            });
                        }
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
            .child(title)
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
