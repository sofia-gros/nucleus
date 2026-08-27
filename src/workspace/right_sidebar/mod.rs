use gpui::*;
use crate::plugin_manager::{PluginManagerGlobal, ui::PanelItem};
use gpui_component::theme::ActiveTheme;

pub struct RightSidebar;

impl RightSidebar {
    pub fn new() -> Self {
        Self
    }
}

impl Render for RightSidebar {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .p_4()
            .bg(cx.theme().background)
            .border_l_1()
            .border_color(cx.theme().border)
            .flex()
            .child(
                div().text_sm().text_color(cx.theme().muted_foreground).child("AI / Plugins")
            )
    }
}
