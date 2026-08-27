use gpui::*;
use gpui_component::*;
use gpui_component::input::{Editor, EditorState};
use gpui_component::tab::TabBar;
use gpui_component::theme::ActiveTheme;
pub mod highlighter;

pub struct EditorArea {
    pub tabs: Vec<(String, String, Option<Entity<EditorState>>)>, // path, title, state
    pub pending_contents: std::collections::HashMap<String, String>,
    pub active_tab: usize,
    pub pending_close_tab: Option<usize>,
}

impl EditorArea {
    pub fn new() -> Self {
        Self {
            tabs: vec![],
            pending_contents: std::collections::HashMap::new(),
            active_tab: 0,
            pending_close_tab: None,
        }
    }

    pub fn close_tab(&mut self, idx: usize, cx: &mut Context<Self>) {
        if idx < self.tabs.len() {
            self.tabs.remove(idx);
            if self.active_tab >= self.tabs.len() && self.active_tab > 0 {
                self.active_tab = self.tabs.len() - 1;
            }
        }
        cx.notify();
    }

    pub fn open_tab(&mut self, path: String, title: String, content: String, cx: &mut Context<Self>) {
        if let Some(idx) = self.tabs.iter().position(|(p, _, _)| p == &path) {
            self.active_tab = idx;
            self.pending_contents.insert(path, content);
        } else {
            self.pending_contents.insert(path.clone(), content);
            self.tabs.push((path, title, None));
            self.active_tab = self.tabs.len() - 1;
        }
        cx.notify();
    }
}

impl Render for EditorArea {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if let Some(idx) = self.pending_close_tab.take() {
            self.close_tab(idx, cx);
        }

        if self.tabs.is_empty() {
            return div()
                .size_full()
                .bg(cx.theme().background)
                .flex()
                .items_center()
                .justify_center()
                .child(
                    div().flex().flex_col().items_center().child(
                        div().text_3xl().font_weight(FontWeight::BOLD).text_color(cx.theme().accent)
                            .child("Nucleus")
                    )
                    .child(
                        div().mt_4().text_sm().text_color(cx.theme().muted_foreground)
                            .child("No files opened. Open a file from the Explorer.")
                    )
                    .child(
                        div().mt_8().flex().flex_col().items_center().text_sm().text_color(cx.theme().muted_foreground)
                            .child(div().child("Press Ctrl+B to toggle sidebar."))
                    )
                )
                .into_any_element();
        }

        let mut tab_bar = TabBar::new("editor-tabs")
            .w_full()
            .selected_index(self.active_tab)
            .on_click(cx.listener(|this, selected: &usize, _, cx| {
                this.active_tab = *selected;
                cx.notify();
            }));

        for (idx, (_, title, _)) in self.tabs.iter().enumerate() {
            tab_bar = tab_bar.child(title.clone());
        }

        // Initialize pending tabs
        for (idx, (path, _title, state_opt)) in self.tabs.iter_mut().enumerate() {
            if state_opt.is_none() {
                if let Some(content) = self.pending_contents.remove(path) {
                    let editor_state = cx.new(|cx| {
                        EditorState::new(_window, cx)
                            .language("rust")
                            .folding(true)
                            .default_value(&content)
                    });
                    *state_opt = Some(editor_state);
                }
            }
        }

        let active_editor = if self.active_tab < self.tabs.len() {
            if let Some(state) = &self.tabs[self.active_tab].2 {
                Editor::new(state).size_full().into_any_element()
            } else {
                div().child("Loading...").into_any_element()
            }
        } else {
            div().into_any_element()
        };

        div()
            .size_full()
            .flex()
            .flex_col()
            .bg(cx.theme().background)
            .child(
                div().w_full().bg(cx.theme().background)
                    .border_b_1()
                    .border_color(cx.theme().border)
                    .child(tab_bar)
            )
            .child(
                div().flex_grow(1.).w_full().child(active_editor)
            )
            .into_any_element()
    }
}
