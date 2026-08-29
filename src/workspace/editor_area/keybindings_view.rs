/// キーボードショートカット設定画面 (Keybindings Editor) UI モジュール

use gpui::*;
use gpui_component::theme::ActiveTheme;
use gpui_component::{Icon, IconName};

/// 1つのキーバインド行データ
#[derive(Clone, Debug)]
pub struct KeybindingRow {
    pub command: String,
    pub key: String,
    pub when: String,
    pub source: String,
}

pub struct KeybindingsView {
    pub query: String,
    pub keybindings: Vec<KeybindingRow>,
    pub selected_index: usize,
}

impl KeybindingsView {
    pub fn new() -> Self {
        Self {
            query: String::new(),
            keybindings: Self::default_keybindings(),
            selected_index: 0,
        }
    }

    /// デフォルトの主要キーバインド一覧
    fn default_keybindings() -> Vec<KeybindingRow> {
        vec![
            KeybindingRow { command: "workbench.action.showCommands".to_string(), key: "Ctrl+Shift+P".to_string(), when: "Global".to_string(), source: "Default".to_string() },
            KeybindingRow { command: "workbench.action.quickOpen".to_string(), key: "Ctrl+P".to_string(), when: "Global".to_string(), source: "Default".to_string() },
            KeybindingRow { command: "workbench.action.files.save".to_string(), key: "Ctrl+S".to_string(), when: "Editor".to_string(), source: "Default".to_string() },
            KeybindingRow { command: "workbench.action.files.newUntitledFile".to_string(), key: "Ctrl+N".to_string(), when: "Global".to_string(), source: "Default".to_string() },
            KeybindingRow { command: "actions.find".to_string(), key: "Ctrl+F".to_string(), when: "Editor".to_string(), source: "Default".to_string() },
            KeybindingRow { command: "editor.action.startFindReplaceAction".to_string(), key: "Ctrl+H".to_string(), when: "Editor".to_string(), source: "Default".to_string() },
            KeybindingRow { command: "workbench.action.toggleSidebarVisibility".to_string(), key: "Ctrl+B".to_string(), when: "Global".to_string(), source: "Default".to_string() },
            KeybindingRow { command: "workbench.action.terminal.toggleTerminal".to_string(), key: "Ctrl+`".to_string(), when: "Global".to_string(), source: "Default".to_string() },
            KeybindingRow { command: "editor.action.quickFix".to_string(), key: "Ctrl+.".to_string(), when: "Editor".to_string(), source: "Default".to_string() },
            KeybindingRow { command: "editor.action.formatDocument".to_string(), key: "Shift+Alt+F".to_string(), when: "Editor".to_string(), source: "Default".to_string() },
            KeybindingRow { command: "editor.action.rename".to_string(), key: "F2".to_string(), when: "Editor".to_string(), source: "Default".to_string() },
            KeybindingRow { command: "editor.action.revealDefinition".to_string(), key: "F12".to_string(), when: "Editor".to_string(), source: "Default".to_string() },
            KeybindingRow { command: "editor.action.goToReferences".to_string(), key: "Shift+F12".to_string(), when: "Editor".to_string(), source: "Default".to_string() },
            KeybindingRow { command: "workbench.action.debug.start".to_string(), key: "F5".to_string(), when: "Global".to_string(), source: "Default".to_string() },
        ]
    }

    /// フィルタされたキーバインド一覧
    pub fn filtered_items(&self) -> Vec<&KeybindingRow> {
        if self.query.is_empty() {
            self.keybindings.iter().collect()
        } else {
            let q = self.query.to_lowercase();
            self.keybindings.iter().filter(|k| {
                k.command.to_lowercase().contains(&q) || k.key.to_lowercase().contains(&q)
            }).collect()
        }
    }
}

impl Render for KeybindingsView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let items = self.filtered_items();

        div()
            .flex()
            .flex_col()
            .size_full()
            .bg(theme.background)
            .child(
                // ヘッダー & 検索バー
                div()
                    .p_4()
                    .border_b_1()
                    .border_color(theme.border)
                    .flex()
                    .flex_col()
                    .gap_3()
                    .child(
                        div().flex().items_center().gap_2()
                            .child(Icon::new(IconName::Settings).size(gpui::px(16.0)).text_color(theme.foreground))
                            .child(div().text_sm().font_weight(FontWeight::BOLD).text_color(theme.foreground).child("Keyboard Shortcuts"))
                    )
                    .child(
                        div()
                            .px_3()
                            .py_1p5()
                            .bg(theme.muted.opacity(0.3))
                            .border_1()
                            .border_color(theme.border)
                            .rounded_md()
                            .flex()
                            .items_center()
                            .child(
                                div().text_xs().text_color(if self.query.is_empty() { theme.muted_foreground } else { theme.foreground })
                                    .child(if self.query.is_empty() { "Type to search in keybindings...".to_string() } else { self.query.clone() })
                            )
                    )
            )
            .child(
                // テーブルヘッダー
                div()
                    .px_4()
                    .py_2()
                    .bg(theme.muted.opacity(0.2))
                    .border_b_1()
                    .border_color(theme.border)
                    .flex()
                    .items_center()
                    .text_xs()
                    .font_weight(FontWeight::BOLD)
                    .text_color(theme.muted_foreground)
                    .child(div().w(gpui::px(300.0)).child("Command"))
                    .child(div().w(gpui::px(150.0)).child("Keybinding"))
                    .child(div().w(gpui::px(100.0)).child("When"))
                    .child(div().flex_1().child("Source"))
            )
            .child(
                // テーブル行一覧
                div()
                    .flex_1()
                    .overflow_hidden()
                    .children(
                        items.into_iter().enumerate().map(|(idx, row)| {
                            let is_selected = idx == self.selected_index;
                            div()
                                .px_4()
                                .py_2()
                                .border_b_1()
                                .border_color(theme.border.opacity(0.5))
                                .flex()
                                .items_center()
                                .text_xs()
                                .bg(if is_selected { theme.secondary } else { theme.background })
                                .hover(|s| s.bg(theme.secondary.opacity(0.5)))
                                .cursor_pointer()
                                .child(div().w(gpui::px(300.0)).font_weight(FontWeight::SEMIBOLD).text_color(theme.foreground).child(row.command.clone()))
                                .child(
                                    div().w(gpui::px(150.0)).child(
                                        div().px_1p5().py_0p5().bg(theme.muted).border_1().border_color(theme.border).rounded_sm().text_color(theme.foreground).child(row.key.clone())
                                    )
                                )
                                .child(div().w(gpui::px(100.0)).text_color(theme.muted_foreground).child(row.when.clone()))
                                .child(div().flex_1().text_color(theme.muted_foreground).child(row.source.clone()))
                        })
                    )
            )
    }
}
