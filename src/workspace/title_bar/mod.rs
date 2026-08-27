use gpui::*;
use gpui_component::theme::ActiveTheme;
use gpui_component::{Icon, IconName};

pub struct TitleBar {
    active_menu: Option<&'static str>,
    should_move: bool,
}

impl TitleBar {
    pub fn new() -> Self {
        Self { active_menu: None, should_move: false }
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
            .flex()
            .items_center()
            .justify_between()
            .on_mouse_down(MouseButton::Left, cx.listener(|this, _event: &MouseDownEvent, _window, _cx| {
                this.should_move = true;
            }))
            .on_mouse_up(MouseButton::Left, cx.listener(|this, _event: &MouseUpEvent, _window, _cx| {
                this.should_move = false;
            }))
            .on_mouse_move(cx.listener(|this, _event: &MouseMoveEvent, window, _cx| {
                if this.should_move {
                    this.should_move = false;
                    window.start_window_move();
                }
            }))
            .child(
                // Left side: Logo and Menus
                div()
                    .flex()
                    .items_center()
                    .h_full()
                    .child(
                        div().text_sm().text_color(theme.muted_foreground).px_2().child("Nucleus IDE")
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
                // Right side: Settings and Window Controls
                div()
                    .flex()
                    .items_center()
                    .h_full()
                    // Settings icon
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_center()
                            .w(gpui::px(32.0))
                            .h_full()
                            .hover(|s| s.bg(theme.secondary))
                            .cursor_pointer()
                            .child(Icon::new(IconName::Settings).text_color(theme.muted_foreground))
                    )
                    // Minimize
                    .child(Self::render_window_control("—", false, cx, |window, _cx| {
                        window.minimize_window();
                    }))
                    // Maximize
                    .child(Self::render_window_control("□", false, cx, |window, _cx| {
                        window.zoom_window();
                    }))
                    // Close
                    .child(Self::render_window_control("✕", true, cx, |_window, cx| {
                        cx.quit();
                    }))
            )
    }
}

impl TitleBar {
    fn render_menu_item(&self, label: &'static str, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme().clone();
        let is_active = self.active_menu == Some(label);
        
        let mut menu_container = div()
            .h_full()
            .flex()
            .items_center()
            .px_3()
            .text_sm()
            .text_color(theme.foreground)
            .hover(|s| s.bg(theme.secondary))
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
            // Render the mock dropdown using deferred to paint it above everything else
            let dropdown = gpui::deferred(
                div()
                    .absolute()
                    .occlude()
                    .top(gpui::px(32.0))
                    .left(gpui::px(0.0))
                    .w(gpui::px(150.0))
                    .bg(theme.background)
                    .border_1()
                    .border_color(theme.border)
                    .rounded_md()
                    .shadow_sm()
                    .p_1()
                    .on_mouse_down(MouseButton::Left, cx.listener(|_, _, _, cx| {
                        cx.stop_propagation();
                    }))
                    .child(
                        div().p_1().px_2().text_sm().text_color(theme.foreground)
                            .hover(|s| s.bg(theme.secondary).cursor_pointer())
                            .on_mouse_down(MouseButton::Left, cx.listener(|this, _, _, cx| {
                                this.active_menu = None;
                                cx.stop_propagation();
                                cx.notify();
                            }))
                            .child(format!("{} Action 1", label))
                    )
                    .child(
                        div().p_1().px_2().text_sm().text_color(theme.foreground)
                            .hover(|s| s.bg(theme.secondary).cursor_pointer())
                            .on_mouse_down(MouseButton::Left, cx.listener(|this, _, _, cx| {
                                this.active_menu = None;
                                cx.stop_propagation();
                                cx.notify();
                            }))
                            .child(format!("{} Action 2", label))
                    )
            );
            menu_container = menu_container.child(dropdown);
        }

        // Wrap in a relative container so the absolute dropdown is positioned correctly
        div().relative().h_full().child(menu_container)
    }

    fn render_window_control<F>(label: &'static str, is_close: bool, cx: &mut Context<Self>, _action: F) -> impl IntoElement
    where
        F: Fn(&mut Window, &mut Context<Self>) + 'static
    {
        let theme = cx.theme();
        div()
            .flex()
            .items_center()
            .justify_center()
            .w(gpui::px(46.0))
            .h_full()
            .text_sm()
            .text_color(theme.muted_foreground)
            .hover(|s| {
                if is_close {
                    s.bg(gpui::rgb(0xe81123)).text_color(gpui::rgb(0xffffff)) // Windows standard close red
                } else {
                    s.bg(theme.secondary)
                }
            })
            .cursor_pointer()
            .on_mouse_down(MouseButton::Left, cx.listener(move |_this, _event, window, cx| {
                cx.stop_propagation();
                _action(window, cx);
            }))
            .child(label)
    }
}
