use gpui::*;

pub struct TitleBar;

impl TitleBar {
    pub fn new() -> Self {
        Self
    }
}

impl Render for TitleBar {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .w_full()
            .h(px(32.0))
            .bg(gpui::rgb(0x1e293b))
            .flex()
            .items_center()
            .px_2()
            .child(
                div().text_sm().text_color(gpui::rgb(0x94a3b8)).child("Nucleus IDE")
            )
    }
}
