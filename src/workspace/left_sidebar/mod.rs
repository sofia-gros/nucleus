use gpui::*;

pub struct LeftSidebar;

impl LeftSidebar {
    pub fn new() -> Self {
        Self
    }
}

impl Render for LeftSidebar {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .h_full()
            .w(px(256.0))
            .bg(gpui::rgb(0x0f172a))
            .border_r_1()
            .border_color(gpui::rgb(0x1e293b))
            .p_2()
            .child(
                div().text_sm().text_color(gpui::rgb(0x64748b)).child("Explorer")
            )
    }
}
