/// 最下部ステータスバーの描画コンポーネント (VSCode 精密再現版)

use gpui::*;
use gpui_component::theme::ActiveTheme;

pub struct StatusBar {
    pub profiler: crate::debug::profiler::FrameProfiler,
}

impl StatusBar {
    pub fn new() -> Self {
        Self {
            profiler: crate::debug::profiler::FrameProfiler::new(),
        }
    }
}

impl Render for StatusBar {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.profiler.start_render();
        let mut left_items = Vec::new();
        let mut right_items = Vec::new();

        if cx.has_global::<crate::plugin_manager::PluginManagerGlobal>() {
            let pm_global = cx.global::<crate::plugin_manager::PluginManagerGlobal>().0.clone();
            let pm = pm_global.read(cx);
            
            for item in &pm.ui_registry.status_bar_items {
                let text = item.text.clone();
                let mut el = div()
                    .px_2()
                    .h_full()
                    .flex()
                    .items_center()
                    .text_xs()
                    .text_color(cx.theme().muted_foreground)
                    .hover(|s| s.bg(cx.theme().muted).text_color(cx.theme().foreground))
                    .cursor_pointer()
                    .child(text);
                
                if let Some(cmd) = item.command.clone() {
                    el = el.on_mouse_down(MouseButton::Left, cx.listener(move |_bar, _event, _window, cx| {
                        if cx.has_global::<crate::plugin_manager::PluginManagerGlobal>() {
                            let pm_global = cx.global::<crate::plugin_manager::PluginManagerGlobal>().0.clone();
                            pm_global.update(cx, |pm, _| {
                                pm.dispatch_event(crate::plugin_manager::event::PluginEvent::CommandExecuted { command: cmd.clone() });
                            });
                        }
                    }));
                }

                match item.alignment {
                    crate::plugin_manager::ui::StatusBarAlignment::Left => left_items.push(el),
                    crate::plugin_manager::ui::StatusBarAlignment::Right => right_items.push(el),
                }
            }
        }

        self.profiler.mark_frame();
        let hud_text = self.profiler.hud_label();

        div()
            .w_full()
            .h(gpui::px(22.0))
            .bg(cx.theme().background)
            .border_t_1()
            .border_color(cx.theme().border)
            .flex()
            .items_center()
            .justify_between()
            .text_xs()
            .child(
                div().flex().items_center().h_full()
                    .children(left_items)
                    .child(
                        div().px_2().h_full().flex().items_center().gap_1().text_color(cx.theme().muted_foreground)
                            .hover(|s| s.bg(cx.theme().muted).text_color(cx.theme().foreground)).cursor_pointer()
                            .on_mouse_down(MouseButton::Left, cx.listener(|_bar, _event, _window, cx| {
                                if cx.has_global::<crate::plugin_manager::PluginManagerGlobal>() {
                                    let pm_global = cx.global::<crate::plugin_manager::PluginManagerGlobal>().0.clone();
                                    pm_global.update(cx, |pm, _| {
                                        pm.dispatch_action(crate::plugin_manager::action::PluginAction::OpenPanel { id: "problems".to_string() });
                                    });
                                }
                            }))
                            .child("⨂ 0  ⚠ 0")
                    )
            )
            .child(
                div().flex().items_center().h_full()
                    .children(right_items)
                    .child(
                        div().px_2().h_full().flex().items_center().text_color(cx.theme().muted_foreground)
                            .hover(|s| s.bg(cx.theme().muted).text_color(cx.theme().foreground)).cursor_pointer()
                            .child("Ln 1, Col 1")
                    )
                    .child(
                        div().px_2().h_full().flex().items_center().text_color(cx.theme().muted_foreground)
                            .hover(|s| s.bg(cx.theme().muted).text_color(cx.theme().foreground)).cursor_pointer()
                            .child("Spaces: 4")
                    )
                    .child(
                        div().px_2().h_full().flex().items_center().text_color(cx.theme().muted_foreground)
                            .hover(|s| s.bg(cx.theme().muted).text_color(cx.theme().foreground)).cursor_pointer()
                            .child("UTF-8")
                    )
                    .child(
                        div().px_2().h_full().flex().items_center().text_color(cx.theme().muted_foreground)
                            .hover(|s| s.bg(cx.theme().muted).text_color(cx.theme().foreground)).cursor_pointer()
                            .child("LF")
                    )
                    .child(
                        div().px_2().h_full().flex().items_center().text_color(gpui::rgb(0x38bdf8))
                            .font_weight(FontWeight::SEMIBOLD)
                            .hover(|s| s.bg(cx.theme().muted)).cursor_pointer()
                            .child(hud_text)
                    )
            )
    }
}
