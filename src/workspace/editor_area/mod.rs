use gpui::*;
use crate::editor::CoreEditor;

pub struct EditorArea {
    core_editor: CoreEditor,
}

impl EditorArea {
    pub fn new() -> Self {
        Self {
            core_editor: CoreEditor::new(),
        }
    }
}

impl Render for EditorArea {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
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
                    .items_center()
                    .px_2()
                    .child(
                        div().text_sm().text_color(gpui::rgb(0x94a3b8)).child("main.rs")
                    )
            )
            .child(
                // Editor content placeholder
                div().flex_grow(1.).p_4().child(
                    div().text_sm().text_color(gpui::rgb(0x64748b)).child("// Editor Content")
                )
            )
    }
}
