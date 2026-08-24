use gpui::*;
use crate::editor::CoreEditor;

pub struct EditorArea {
    _core_editor: CoreEditor,
    pub open_tabs: Vec<String>,
}

impl EditorArea {
    pub fn new() -> Self {
        Self {
            _core_editor: CoreEditor::new(),
            open_tabs: vec!["main.rs".to_string()],
        }
    }

    pub fn open_tab(&mut self, title: String, _content: String, cx: &mut Context<Self>) {
        self.open_tabs.push(title);
        cx.notify();
    }
}

impl Render for EditorArea {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let tabs = self.open_tabs.iter().map(|tab| {
            div()
                .h_full()
                .px_4()
                .flex()
                .items_center()
                .border_r_1()
                .border_color(gpui::rgb(0x1e293b))
                .child(
                    div().text_sm().text_color(gpui::rgb(0x94a3b8)).child(tab.clone())
                )
        }).collect::<Vec<_>>();

        div()
            .flex_grow(1.)
            .h_full()
            .bg(gpui::rgb(0x020617)) // Slate 950 (Darker for editor)
            .flex()
            .flex_col()
            .child(
                // Tab bar placeholder
                div().h(px(32.0)).w_full().bg(gpui::rgb(0x0f172a))
                    .border_b_1()
                    .border_color(gpui::rgb(0x1e293b))
                    .flex()
                    .flex_row()
                    .children(tabs)
            )
            .child(
                // Editor content placeholder
                div().flex_grow(1.).p_4().child(
                    div().text_sm().text_color(gpui::rgb(0x64748b)).child(
                        if self.open_tabs.is_empty() {
                            "// No tabs open"
                        } else {
                            "// Editor Content"
                        }
                    )
                )
            )
    }
}
