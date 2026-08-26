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
            open_tabs: vec![],
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
                if self.open_tabs.is_empty() {
                    // Welcome screen
                    div().flex_grow(1.).flex().flex_col().justify_center().items_center()
                        .child(
                            div().text_3xl().font_weight(FontWeight::BOLD).text_color(gpui::rgb(0x38bdf8))
                                .child("Nucleus")
                        )
                        .child(
                            div().mt_4().text_sm().text_color(gpui::rgb(0x94a3b8))
                                .child("A next-generation IDE built with Rust & GPUI.")
                        )
                        .child(
                            div().mt_8().flex().flex_col().items_center().text_sm().text_color(gpui::rgb(0x64748b))
                                .child(div().child("No workspace opened."))
                                .child(div().mt_2().child("Use 'File > Open Folder' to begin."))
                        )
                } else {
                    // Editor content placeholder
                    div().flex_grow(1.).p_4().child(
                        div().text_sm().text_color(gpui::rgb(0x64748b)).child("// Editor Content")
                    )
                }
            )
    }
}
