use gpui::*;

pub struct BottomPanel {
    pub logs: Vec<String>,
}

impl BottomPanel {
    pub fn new() -> Self {
        Self { logs: Vec::new() }
    }

    pub fn write_log(&mut self, text: String, cx: &mut Context<Self>) {
        self.logs.push(text);
        cx.notify();
    }
}

impl Render for BottomPanel {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let logs_element = div().flex().flex_col().gap_1().children(
            self.logs.iter().map(|log| div().text_sm().text_color(gpui::rgb(0x94a3b8)).child(log.clone()))
        );

        div()
            .size_full()
            .bg(gpui::rgb(0x0f172a))
            .border_t_1()
            .border_color(gpui::rgb(0x1e293b))
            .p_2()
            .child(
                div().text_sm().text_color(gpui::rgb(0x64748b)).child("Terminal Output")
            )
            .child(logs_element)
    }
}
