use gpui::*;
use gpui_component::theme::ActiveTheme;

pub struct TitleBar;

impl TitleBar {
    pub fn new() -> Self {
        Self
    }
}

impl Render for TitleBar {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .w_full()
            .h(gpui::px(32.0))
            .bg(cx.theme().background)
            .flex()
            .items_center()
            .px_2()
            .child(
                div().text_sm().text_color(cx.theme().muted_foreground).child("Nucleus IDE")
            )
    }
}
