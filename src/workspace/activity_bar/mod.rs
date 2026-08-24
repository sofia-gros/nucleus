use gpui::*;

pub struct ActivityBar;

impl ActivityBar {
    pub fn new() -> Self {
        Self
    }
}

impl Render for ActivityBar {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .h_full()
            .w(px(48.0))
            .bg(gpui::rgb(0x0f172a))
            .border_r_1()
            .border_color(gpui::rgb(0x1e293b))
            .flex()
            .flex_col()
            .items_center()
            .py_2()
            .child(
                div().text_sm().text_color(gpui::rgb(0x64748b)).child("AB")
            )
    }
}
