use gpui::*;
use gpui_component::*;
use gpui_component::theme::ActiveTheme;

pub struct ActivityBar;

impl ActivityBar {
    pub fn new() -> Self {
        Self
    }
}

impl Render for ActivityBar {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let mut items = Vec::new();

        if cx.has_global::<crate::plugin_manager::PluginManagerGlobal>() {
            let pm_global = cx.global::<crate::plugin_manager::PluginManagerGlobal>().0.clone();
            let pm = pm_global.read(cx);
            
            for item in &pm.ui_registry.activity_bar_items {
                let tooltip = item.tooltip.clone();
                let icon_str = item.icon.clone();
                // Simple representation for now: first two letters of tooltip or icon string
                let display_text = if icon_str.starts_with("lucide-") {
                    icon_str.chars().skip(7).take(2).collect::<String>().to_uppercase()
                } else if tooltip.len() >= 2 {
                    tooltip.chars().take(2).collect::<String>().to_uppercase()
                } else {
                    "P".to_string()
                };

                let is_active = false;
                let mut el = div()
                    .w_full()
                    .flex()
                    .justify_center()
                    .py_2()
                    .text_sm()
                    .text_color(if is_active { cx.theme().foreground } else { cx.theme().muted_foreground })
                    .hover(|s| s.bg(cx.theme().muted).text_color(cx.theme().foreground))
                    .cursor_pointer()
                    .child(display_text);

                if !item.command.is_empty() {
                    let cmd = item.command.clone();
                    el = el.on_mouse_down(MouseButton::Left, cx.listener(move |_bar, _event, _window, cx| {
                        if cx.has_global::<crate::plugin_manager::PluginManagerGlobal>() {
                            let pm_global = cx.global::<crate::plugin_manager::PluginManagerGlobal>().0.clone();
                            pm_global.update(cx, |pm, _| {
                                pm.dispatch_event(crate::plugin_manager::event::PluginEvent::CommandExecuted { command: cmd.clone() });
                            });
                        }
                    }));
                }

                items.push(el);
            }
        }

        let is_explorer_active = true; // TODO: read from state
        let explorer_icon = div()
            .w_full()
            .flex()
            .justify_center()
            .py_2()
            .text_sm()
            .text_color(if is_explorer_active { cx.theme().foreground } else { cx.theme().muted_foreground })
            .hover(|s| s.bg(cx.theme().muted).text_color(cx.theme().foreground))
            .cursor_pointer()
            .child(Icon::new(IconName::File))
            .on_mouse_down(MouseButton::Left, cx.listener(|_bar, _event, _window, cx| {
                if cx.has_global::<crate::plugin_manager::PluginManagerGlobal>() {
                    let pm_global = cx.global::<crate::plugin_manager::PluginManagerGlobal>().0.clone();
                    pm_global.update(cx, |pm, _| {
                        // Dispatch an action that main.rs / workspace will catch to open explorer
                        pm.dispatch_action(crate::plugin_manager::action::PluginAction::OpenPanel { id: "explorer".to_string() });
                    });
                }
            }));
        items.insert(0, explorer_icon);

        div()
            .flex()
            .flex_col()
            .w(px(48.0))
            .h_full()
            .bg(cx.theme().background)
            .border_r_1()
            .border_color(cx.theme().border)
            .flex()
            .flex_col()
            .items_center()
            .children(items)
    }
}
