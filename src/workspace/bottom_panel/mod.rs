use gpui::*;

pub struct BottomPanel;

impl BottomPanel {
    pub fn new() -> Self {
        Self
    }
}

impl Render for BottomPanel {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .w_full()
            .h(px(200.0))
            .bg(gpui::rgb(0x0f172a))
            .border_t_1()
            .border_color(gpui::rgb(0x1e293b))
            .p_2()
            .child(
                div().text_sm().text_color(gpui::rgb(0x64748b)).child("Terminal Output")
            )
    }
}
