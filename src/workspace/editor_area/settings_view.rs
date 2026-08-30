/// 設定画面（Settings View: User / Workspace 切り替え対応）UI コンポーネント

use gpui::*;
use gpui_component::*;
use gpui_component::theme::ActiveTheme;
use crate::settings::{SettingsGlobal, SettingsTarget};

/// 設定画面のエンティティ
pub struct SettingsView {
    pub current_target: SettingsTarget,
    pub theme_mode: String,
    pub font_size: usize,
    pub tab_size: usize,
    pub soft_wrap: bool,
    pub auto_save_interval: usize,
}

impl SettingsView {
    /// 新規 SettingsView の作成
    pub fn new(cx: &mut Context<Self>) -> Self {
        let mut view = Self {
            current_target: SettingsTarget::User,
            theme_mode: "dark".to_string(),
            font_size: 14,
            tab_size: 4,
            soft_wrap: false,
            auto_save_interval: 30,
        };
        view.load_from_store(cx);
        view
    }

    /// 設定ストアから現在のターゲットの値を読み込む
    pub fn load_from_store(&mut self, cx: &App) {
        if cx.has_global::<SettingsGlobal>() {
            let store = cx.global::<SettingsGlobal>().0.read().unwrap();
            
            let (theme, font, tab, wrap, interval) = match self.current_target {
                SettingsTarget::User => (
                    store.get_user("theme").and_then(|v| v.as_str().map(|s| s.to_string())).unwrap_or_else(|| "dark".to_string()),
                    store.get_user("editor.font_size").and_then(|v| v.as_u64()).unwrap_or(14) as usize,
                    store.get_user("editor.tab_size").and_then(|v| v.as_u64()).unwrap_or(4) as usize,
                    store.get_user("editor.soft_wrap").and_then(|v| v.as_bool()).unwrap_or(false),
                    store.get_user("files.auto_save_interval").and_then(|v| v.as_u64()).unwrap_or(30) as usize,
                ),
                SettingsTarget::Workspace => (
                    store.get_workspace("theme").and_then(|v| v.as_str().map(|s| s.to_string())).unwrap_or_else(|| "dark".to_string()),
                    store.get_workspace("editor.font_size").and_then(|v| v.as_u64()).unwrap_or(14) as usize,
                    store.get_workspace("editor.tab_size").and_then(|v| v.as_u64()).unwrap_or(4) as usize,
                    store.get_workspace("editor.soft_wrap").and_then(|v| v.as_bool()).unwrap_or(false),
                    store.get_workspace("files.auto_save_interval").and_then(|v| v.as_u64()).unwrap_or(30) as usize,
                ),
            };

            self.theme_mode = theme;
            self.font_size = font;
            self.tab_size = tab;
            self.soft_wrap = wrap;
            self.auto_save_interval = interval;
        }
    }

    /// 設定変更をストアに保存し、UIへ即時反映
    pub fn save_setting(&mut self, key: &str, value: serde_json::Value, cx: &mut Context<Self>) {
        if key == "theme" {
            if let Some(s) = value.as_str() {
                let mode = match s {
                    "light" => gpui_component::theme::ThemeMode::Light,
                    _ => gpui_component::theme::ThemeMode::Dark,
                };
                gpui_component::theme::Theme::change(mode, None, cx);
            }
        }
        if cx.has_global::<SettingsGlobal>() {
            let mut store = cx.global::<SettingsGlobal>().0.write().unwrap();
            store.set_target(self.current_target, key, value);
        }
        self.load_from_store(cx);
        cx.notify();
    }
}

impl Render for SettingsView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme().clone();
        let is_user = self.current_target == SettingsTarget::User;
        let is_workspace = self.current_target == SettingsTarget::Workspace;

        div()
            .size_full()
            .bg(theme.background)
            .flex()
            .flex_col()
            .p_6()
            .gap_6()
            .overflow_hidden()
            .child(
                // ヘッダー & User / Workspace 切り替えタブ
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .border_b_1()
                    .border_color(theme.border)
                    .pb_4()
                    .child(
                        div().text_xl().font_weight(FontWeight::BOLD).text_color(theme.foreground).child("Settings")
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .bg(theme.muted.opacity(0.3))
                            .rounded_md()
                            .p_1()
                            .gap_1()
                            .child(
                                div()
                                    .px_3()
                                    .py_1()
                                    .rounded_sm()
                                    .text_xs()
                                    .font_weight(if is_user { FontWeight::BOLD } else { FontWeight::NORMAL })
                                    .bg(if is_user { theme.background } else { theme.muted.opacity(0.0) })
                                    .text_color(if is_user { theme.foreground } else { theme.muted_foreground })
                                    .hover(|s| s.bg(theme.secondary))
                                    .cursor_pointer()
                                    .on_mouse_down(MouseButton::Left, cx.listener(|this, _, _, cx| {
                                        this.current_target = SettingsTarget::User;
                                        this.load_from_store(cx);
                                        cx.notify();
                                    }))
                                    .child("User (Global)")
                            )
                            .child(
                                div()
                                    .px_3()
                                    .py_1()
                                    .rounded_sm()
                                    .text_xs()
                                    .font_weight(if is_workspace { FontWeight::BOLD } else { FontWeight::NORMAL })
                                    .bg(if is_workspace { theme.background } else { theme.muted.opacity(0.0) })
                                    .text_color(if is_workspace { theme.foreground } else { theme.muted_foreground })
                                    .hover(|s| s.bg(theme.secondary))
                                    .cursor_pointer()
                                    .on_mouse_down(MouseButton::Left, cx.listener(|this, _, _, cx| {
                                        this.current_target = SettingsTarget::Workspace;
                                        this.load_from_store(cx);
                                        cx.notify();
                                    }))
                                    .child("Workspace")
                            )
                    )
            )
            .child(
                // 設定項目リスト
                div()
                    .flex()
                    .flex_col()
                    .gap_6()
                    .overflow_hidden()
                    // 1. Appearance
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_3()
                            .child(div().text_sm().font_weight(FontWeight::BOLD).text_color(gpui::rgb(0x38bdf8)).child("Appearance"))
                            .child(
                                Self::render_setting_row(
                                    "Color Theme",
                                    "Controls the theme used across the workbench.",
                                    div()
                                        .flex()
                                        .gap_2()
                                        .child(
                                            Self::render_choice_button("Dark", self.theme_mode == "dark", cx.listener(|this, _, _, cx| {
                                                this.save_setting("theme", serde_json::json!("dark"), cx);
                                            }))
                                        )
                                        .child(
                                            Self::render_choice_button("Light", self.theme_mode == "light", cx.listener(|this, _, _, cx| {
                                                this.save_setting("theme", serde_json::json!("light"), cx);
                                            }))
                                        ),
                                    &theme,
                                )
                            )
                            .child(
                                Self::render_setting_row(
                                    "Font Size",
                                    "Controls the font size in pixels for the editor buffer.",
                                    div()
                                        .flex()
                                        .gap_2()
                                        .child(Self::render_choice_button("12px", self.font_size == 12, cx.listener(|this, _, _, cx| {
                                            this.save_setting("editor.font_size", serde_json::json!(12), cx);
                                        })))
                                        .child(Self::render_choice_button("14px", self.font_size == 14, cx.listener(|this, _, _, cx| {
                                            this.save_setting("editor.font_size", serde_json::json!(14), cx);
                                        })))
                                        .child(Self::render_choice_button("16px", self.font_size == 16, cx.listener(|this, _, _, cx| {
                                            this.save_setting("editor.font_size", serde_json::json!(16), cx);
                                        })))
                                        .child(Self::render_choice_button("18px", self.font_size == 18, cx.listener(|this, _, _, cx| {
                                            this.save_setting("editor.font_size", serde_json::json!(18), cx);
                                        }))),
                                    &theme,
                                )
                            )
                    )
                    // 2. Editor
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_3()
                            .child(div().text_sm().font_weight(FontWeight::BOLD).text_color(gpui::rgb(0x38bdf8)).child("Editor"))
                            .child(
                                Self::render_setting_row(
                                    "Tab Size",
                                    "The number of spaces a tab is equal to.",
                                    div()
                                        .flex()
                                        .gap_2()
                                        .child(Self::render_choice_button("2", self.tab_size == 2, cx.listener(|this, _, _, cx| {
                                            this.save_setting("editor.tab_size", serde_json::json!(2), cx);
                                        })))
                                        .child(Self::render_choice_button("4", self.tab_size == 4, cx.listener(|this, _, _, cx| {
                                            this.save_setting("editor.tab_size", serde_json::json!(4), cx);
                                        })))
                                        .child(Self::render_choice_button("8", self.tab_size == 8, cx.listener(|this, _, _, cx| {
                                            this.save_setting("editor.tab_size", serde_json::json!(8), cx);
                                        }))),
                                    &theme,
                                )
                            )
                            .child(
                                Self::render_setting_row(
                                    "Soft Wrap",
                                    "Controls whether lines should wrap around the viewport.",
                                    div()
                                        .flex()
                                        .gap_2()
                                        .child(Self::render_choice_button("Off", !self.soft_wrap, cx.listener(|this, _, _, cx| {
                                            this.save_setting("editor.soft_wrap", serde_json::json!(false), cx);
                                        })))
                                        .child(Self::render_choice_button("On", self.soft_wrap, cx.listener(|this, _, _, cx| {
                                            this.save_setting("editor.soft_wrap", serde_json::json!(true), cx);
                                        }))),
                                    &theme,
                                )
                            )
                    )
                    // 3. Files
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_3()
                            .child(div().text_sm().font_weight(FontWeight::BOLD).text_color(gpui::rgb(0x38bdf8)).child("Files"))
                            .child(
                                Self::render_setting_row(
                                    "Auto Save Interval (seconds)",
                                    "Controls the interval in seconds for automatic backup and state snapshots.",
                                    div()
                                        .flex()
                                        .gap_2()
                                        .child(Self::render_choice_button("10s", self.auto_save_interval == 10, cx.listener(|this, _, _, cx| {
                                            this.save_setting("files.auto_save_interval", serde_json::json!(10), cx);
                                        })))
                                        .child(Self::render_choice_button("30s", self.auto_save_interval == 30, cx.listener(|this, _, _, cx| {
                                            this.save_setting("files.auto_save_interval", serde_json::json!(30), cx);
                                        })))
                                        .child(Self::render_choice_button("60s", self.auto_save_interval == 60, cx.listener(|this, _, _, cx| {
                                            this.save_setting("files.auto_save_interval", serde_json::json!(60), cx);
                                        }))),
                                    &theme,
                                )
                            )
                    )
            )
    }
}

impl SettingsView {
    fn render_setting_row(title: &'static str, desc: &'static str, control: impl IntoElement, theme: &Theme) -> impl IntoElement {
        div()
            .w_full()
            .p_3()
            .bg(theme.muted.opacity(0.15))
            .border_1()
            .border_color(theme.border)
            .rounded_md()
            .flex()
            .items_center()
            .justify_between()
            .child(
                div().flex().flex_col().gap_1()
                    .child(div().text_xs().font_weight(FontWeight::SEMIBOLD).text_color(theme.foreground).child(title))
                    .child(div().text_xs().text_color(theme.muted_foreground).child(desc))
            )
            .child(control)
    }

    fn render_choice_button(
        label: &'static str,
        is_active: bool,
        on_click: impl Fn(&MouseDownEvent, &mut Window, &mut App) + 'static,
    ) -> impl IntoElement {
        div()
            .px_3()
            .py_1()
            .rounded_sm()
            .text_xs()
            .font_weight(if is_active { FontWeight::BOLD } else { FontWeight::NORMAL })
            .bg(if is_active { gpui::rgb(0x007acc) } else { gpui::rgb(0x2d3748) })
            .text_color(if is_active { gpui::rgb(0xffffff) } else { gpui::rgb(0xa0aec0) })
            .hover(|s| s.bg(if is_active { gpui::rgb(0x0062a3) } else { gpui::rgb(0x4a5568) }))
            .cursor_pointer()
            .on_mouse_down(MouseButton::Left, on_click)
            .child(label)
    }
}
