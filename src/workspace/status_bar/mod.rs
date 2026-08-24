use gpui::*;

pub struct StatusBar;

impl StatusBar {
    pub fn new() -> Self {
        Self
    }
}

impl Render for StatusBar {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .w_full()
            .h(px(24.0))
            .bg(gpui::rgb(0x0f172a))
            // using solid background for now to simulate border-top visually if border is tricky
            .flex()
            .items_center()
            .px_2()
            .child(
                div().text_sm().text_color(gpui::rgb(0x64748b)).child("Ready")
            )
    }
}
