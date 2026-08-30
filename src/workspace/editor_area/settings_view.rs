/// 設定画面（Settings View: Master-Detail カテゴリナビゲーション & 全項目即時反映 & スクロール対応）UI コンポーネント

use gpui::*;
use gpui_component::*;
use gpui_component::theme::ActiveTheme;
use crate::settings::{SettingsGlobal, SettingsStore, SettingsTarget, SettingGroup, SettingType, SettingDefinition};

/// 設定画面のエンティティ
pub struct SettingsView {
    pub current_target: SettingsTarget,
    pub current_group: SettingGroup,
    pub search_query: String,
    pub focus_handle: FocusHandle,
    pub scroll_offset: usize,
    pub editing_key: Option<&'static str>,
    pub editing_text: String,
}

impl SettingsView {
    /// 新規 SettingsView の作成
    pub fn new(cx: &mut Context<Self>) -> Self {
        Self {
            current_target: SettingsTarget::User,
            current_group: SettingGroup::All,
            search_query: String::new(),
            focus_handle: cx.focus_handle(),
            scroll_offset: 0,
            editing_key: None,
            editing_text: String::new(),
        }
    }

    /// 設定ストアから現在のターゲットの値を読み込む
    pub fn get_current_value(&self, key: &str, default: &serde_json::Value, cx: &App) -> serde_json::Value {
        if cx.has_global::<SettingsGlobal>() {
            let store = cx.global::<SettingsGlobal>().0.read().unwrap();
            match self.current_target {
                SettingsTarget::User => store.get_user(key).unwrap_or_else(|| default.clone()),
                SettingsTarget::Workspace => store.get_workspace(key).unwrap_or_else(|| default.clone()),
            }
        } else {
            default.clone()
        }
    }

    /// 設定変更をストアに保存し、UIへ即時反映
    pub fn save_setting(&mut self, key: &'static str, value: serde_json::Value, cx: &mut Context<Self>) {
        if key == "workbench.colorTheme" || key == "theme" {
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
        cx.notify();
    }
}

impl Render for SettingsView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme().clone();
        let is_user = self.current_target == SettingsTarget::User;
        let is_workspace = self.current_target == SettingsTarget::Workspace;

        let all_groups = [
            SettingGroup::All,
            SettingGroup::Appearance,
            SettingGroup::Editor,
            SettingGroup::Files,
            SettingGroup::Terminal,
            SettingGroup::LanguagesAndLsp,
            SettingGroup::Debug,
            SettingGroup::Git,
            SettingGroup::Plugins,
        ];

        // 左側グループナビゲーションサイドバー (絵文字なし・クリーンテキスト)
        let mut group_nav = div()
            .w(gpui::px(220.0))
            .h_full()
            .border_r_1()
            .border_color(theme.border)
            .bg(theme.muted.opacity(0.15))
            .flex()
            .flex_col()
            .gap_1()
            .p_2()
            .overflow_hidden();

        for group in all_groups {
            let is_selected = self.current_group == group;
            let count = SettingsStore::get_items_by_group(group).len();
            let label = group.label();

            group_nav = group_nav.child(
                div()
                    .px_3()
                    .py_2()
                    .rounded_md()
                    .flex()
                    .items_center()
                    .justify_between()
                    .cursor_pointer()
                    .bg(if is_selected { theme.secondary } else { gpui::transparent_black() })
                    .hover(|s| s.bg(theme.secondary))
                    .on_mouse_down(MouseButton::Left, cx.listener(move |this, _, _, cx| {
                        this.current_group = group;
                        this.scroll_offset = 0;
                        cx.notify();
                    }))
                    .child(
                        div()
                            .text_xs()
                            .font_weight(if is_selected { FontWeight::BOLD } else { FontWeight::NORMAL })
                            .text_color(if is_selected { theme.foreground } else { theme.muted_foreground })
                            .child(label)
                    )
                    .child(
                        div()
                            .px_1p5()
                            .py_0p5()
                            .rounded_sm()
                            .text_xs()
                            .bg(theme.muted)
                            .text_color(theme.muted_foreground)
                            .child(format!("{}", count))
                    )
            );
        }

        // 右側設定エディタ一覧の構築
        let items = SettingsStore::get_items_by_group(self.current_group);
        let query_lower = self.search_query.to_lowercase();

        let filtered_items: Vec<SettingDefinition> = items.into_iter().filter(|item| {
            if query_lower.is_empty() {
                true
            } else {
                item.key.to_lowercase().contains(&query_lower)
                    || item.label.to_lowercase().contains(&query_lower)
                    || item.description.to_lowercase().contains(&query_lower)
            }
        }).collect();

        let total_items = filtered_items.len();
        let page_capacity = 20;
        let max_scroll = total_items.saturating_sub(page_capacity);
        let effective_scroll = self.scroll_offset.min(max_scroll);
        let visible_items = if total_items > 0 {
            let end = (effective_scroll + page_capacity).min(total_items);
            &filtered_items[effective_scroll..end]
        } else {
            &[]
        };

        let mut settings_list = div()
            .flex_1()
            .min_w_0()
            .h_full()
            .flex()
            .flex_col()
            .gap_4()
            .p_6()
            .overflow_hidden()
            .on_scroll_wheel(cx.listener(|this, event: &ScrollWheelEvent, _, cx| {
                let delta = match event.delta {
                    ScrollDelta::Pixels(p) => (f32::from(p.y) / 20.0).round() as i32,
                    ScrollDelta::Lines(l) => l.y.round() as i32,
                };
                if delta < 0 {
                    this.scroll_offset = this.scroll_offset.saturating_add((-delta) as usize);
                } else if delta > 0 {
                    this.scroll_offset = this.scroll_offset.saturating_sub(delta as usize);
                }
                cx.notify();
            }));

        for item in visible_items {
            let key = item.key;
            let current_val = self.get_current_value(key, &item.default_value, cx);

            let control_element = match &item.setting_type {
                SettingType::Bool => {
                    let is_checked = current_val.as_bool().unwrap_or(false);
                    let active_blue: gpui::Hsla = gpui::rgb(0x007acc).into();
                    div()
                        .flex()
                        .items_center()
                        .gap_2()
                        .cursor_pointer()
                        .on_mouse_down(MouseButton::Left, cx.listener(move |this, _, _, cx| {
                            this.save_setting(key, serde_json::json!(!is_checked), cx);
                        }))
                        .child(
                            div()
                                .w(gpui::px(18.0))
                                .h(gpui::px(18.0))
                                .border_1()
                                .border_color(if is_checked { active_blue } else { theme.border })
                                .bg(if is_checked { active_blue } else { gpui::transparent_black() })
                                .rounded_sm()
                                .flex()
                                .items_center()
                                .justify_center()
                                .text_xs()
                                .text_color(gpui::rgb(0xffffff))
                                .child(if is_checked { "✓" } else { "" })
                        )
                        .child(
                            div().text_xs().text_color(theme.muted_foreground).child(if is_checked { "Enabled" } else { "Disabled" })
                        )
                        .into_any_element()
                }
                SettingType::Select(options) => {
                    let curr_str = current_val.as_str().unwrap_or("").to_string();
                    let options_vec: Vec<&'static str> = options.clone();
                    let active_blue: gpui::Hsla = gpui::rgb(0x007acc).into();
                    let active_white: gpui::Hsla = gpui::rgb(0xffffff).into();

                    let mut opt_row = div().flex().items_center().gap_1p5();
                    for opt in options_vec {
                        let is_opt_active = curr_str == opt;
                        let opt_str = opt;
                        opt_row = opt_row.child(
                            div()
                                .px_2p5()
                                .py_1()
                                .rounded_md()
                                .text_xs()
                                .cursor_pointer()
                                .bg(if is_opt_active { active_blue } else { theme.muted })
                                .text_color(if is_opt_active { active_white } else { theme.muted_foreground })
                                .hover(|s| s.bg(gpui::rgb(0x007acc)).text_color(gpui::rgb(0xffffff)))
                                .on_mouse_down(MouseButton::Left, cx.listener(move |this, _, _, cx| {
                                    this.save_setting(key, serde_json::json!(opt_str), cx);
                                }))
                                .child(opt_str.to_string())
                        );
                    }
                    opt_row.into_any_element()
                }
                SettingType::Number { step, min: _, max: _ } => {
                    let num_val = current_val.as_f64().unwrap_or(0.0);
                    let step_val = *step;
                    div()
                        .flex()
                        .items_center()
                        .gap_2()
                        .child(
                            div()
                                .px_3()
                                .py_1()
                                .bg(theme.muted.opacity(0.3))
                                .border_1()
                                .border_color(theme.border)
                                .rounded_sm()
                                .text_xs()
                                .font_weight(FontWeight::BOLD)
                                .text_color(theme.foreground)
                                .child(format!("{}", num_val))
                        )
                        .child(
                            div()
                                .px_2()
                                .py_0p5()
                                .bg(theme.muted)
                                .rounded_sm()
                                .text_xs()
                                .cursor_pointer()
                                .hover(|s| s.bg(theme.secondary))
                                .on_mouse_down(MouseButton::Left, cx.listener(move |this, _, _, cx| {
                                    this.save_setting(key, serde_json::json!((num_val - step_val).max(0.0)), cx);
                                }))
                                .child("—")
                        )
                        .child(
                            div()
                                .px_2()
                                .py_0p5()
                                .bg(theme.muted)
                                .rounded_sm()
                                .text_xs()
                                .cursor_pointer()
                                .hover(|s| s.bg(theme.secondary))
                                .on_mouse_down(MouseButton::Left, cx.listener(move |this, _, _, cx| {
                                    this.save_setting(key, serde_json::json!(num_val + step_val), cx);
                                }))
                                .child("+")
                        )
                        .into_any_element()
                }
                SettingType::String => {
                    let str_val = current_val.as_str().unwrap_or("").to_string();
                    let is_editing = self.editing_key == Some(key);

                    if is_editing {
                        let text_val = self.editing_text.clone();
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(
                                div()
                                    .px_3()
                                    .py_1p5()
                                    .w(gpui::px(320.0))
                                    .bg(theme.muted.opacity(0.3))
                                    .border_1()
                                    .border_color(gpui::rgb(0x007acc))
                                    .rounded_sm()
                                    .text_xs()
                                    .text_color(theme.foreground)
                                    .cursor(CursorStyle::IBeam)
                                    .on_key_down(cx.listener(move |this, event: &KeyDownEvent, _, cx| {
                                        let k = &event.keystroke.key;
                                        if k == "enter" {
                                            let final_val = this.editing_text.clone();
                                            this.editing_key = None;
                                            this.save_setting(key, serde_json::json!(final_val), cx);
                                        } else if k == "escape" {
                                            this.editing_key = None;
                                            cx.notify();
                                        } else if k == "backspace" {
                                            this.editing_text.pop();
                                            cx.notify();
                                        } else if !event.keystroke.modifiers.control && !event.keystroke.modifiers.alt {
                                            if k.len() == 1 {
                                                this.editing_text.push_str(k);
                                                cx.notify();
                                            } else if k == "space" {
                                                this.editing_text.push(' ');
                                                cx.notify();
                                            }
                                        }
                                    }))
                                    .child(if text_val.is_empty() { "_".to_string() } else { format!("{}_", text_val) })
                            )
                            .child(
                                div()
                                    .px_2()
                                    .py_1()
                                    .bg(gpui::rgb(0x007acc))
                                    .text_color(gpui::rgb(0xffffff))
                                    .rounded_sm()
                                    .text_xs()
                                    .cursor_pointer()
                                    .on_mouse_down(MouseButton::Left, cx.listener(move |this, _, _, cx| {
                                        let final_val = this.editing_text.clone();
                                        this.editing_key = None;
                                        this.save_setting(key, serde_json::json!(final_val), cx);
                                    }))
                                    .child("Save")
                            )
                            .into_any_element()
                    } else {
                        let init_str = str_val.clone();
                        div()
                            .px_3()
                            .py_1p5()
                            .w(gpui::px(320.0))
                            .bg(theme.muted.opacity(0.3))
                            .border_1()
                            .border_color(theme.border)
                            .rounded_sm()
                            .text_xs()
                            .text_color(if str_val.is_empty() { theme.muted_foreground } else { theme.foreground })
                            .cursor_pointer()
                            .hover(|s| s.border_color(gpui::rgb(0x007acc)))
                            .on_mouse_down(MouseButton::Left, cx.listener(move |this, _, _, cx| {
                                this.editing_key = Some(key);
                                this.editing_text = init_str.clone();
                                cx.notify();
                            }))
                            .child(if str_val.is_empty() { "Click to edit...".to_string() } else { str_val })
                            .into_any_element()
                    }
                }
            };

            settings_list = settings_list.child(
                div()
                    .flex()
                    .flex_col()
                    .gap_1p5()
                    .pb_4()
                    .border_b_1()
                    .border_color(theme.border.opacity(0.5))
                    .child(
                        div().text_sm().font_weight(FontWeight::SEMIBOLD).text_color(theme.foreground).child(item.label)
                    )
                    .child(
                        div().text_xs().text_color(theme.muted_foreground).child(item.description)
                    )
                    .child(
                        div().mt_1().child(control_element)
                    )
            );
        }

        div()
            .size_full()
            .bg(theme.background)
            .flex()
            .flex_col()
            .overflow_hidden()
            .child(
                // ヘッダー: タイトル + User/Workspace タブ + リアルタイム検索バー
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .border_b_1()
                    .border_color(theme.border)
                    .px_6()
                    .py_3()
                    .child(
                        div().flex().items_center().gap_3()
                            .child(Icon::new(IconName::Settings).size(gpui::px(18.0)).text_color(theme.foreground))
                            .child(div().text_base().font_weight(FontWeight::BOLD).text_color(theme.foreground).child("Settings"))
                    )
                    .child(
                        div().flex().items_center().gap_4()
                            // User / Workspace 切替タブ
                            .child(
                                div().flex().bg(theme.muted).p_0p5().rounded_md()
                                    .child(
                                        div()
                                            .px_3()
                                            .py_1()
                                            .rounded_sm()
                                            .text_xs()
                                            .font_weight(if is_user { FontWeight::BOLD } else { FontWeight::NORMAL })
                                            .bg(if is_user { theme.background } else { gpui::transparent_black() })
                                            .text_color(if is_user { theme.foreground } else { theme.muted_foreground })
                                            .cursor_pointer()
                                            .on_mouse_down(MouseButton::Left, cx.listener(|this, _, _, cx| {
                                                this.current_target = SettingsTarget::User;
                                                cx.notify();
                                            }))
                                            .child("User")
                                    )
                                    .child(
                                        div()
                                            .px_3()
                                            .py_1()
                                            .rounded_sm()
                                            .text_xs()
                                            .font_weight(if is_workspace { FontWeight::BOLD } else { FontWeight::NORMAL })
                                            .bg(if is_workspace { theme.background } else { gpui::transparent_black() })
                                            .text_color(if is_workspace { theme.foreground } else { theme.muted_foreground })
                                            .cursor_pointer()
                                            .on_mouse_down(MouseButton::Left, cx.listener(|this, _, _, cx| {
                                                this.current_target = SettingsTarget::Workspace;
                                                cx.notify();
                                            }))
                                            .child("Workspace")
                                    )
                            )
                            // リアルタイム検索バー (IME / タイピング対応)
                            .child(
                                div()
                                    .w(gpui::px(260.0))
                                    .px_3()
                                    .py_1()
                                    .bg(theme.muted.opacity(0.3))
                                    .border_1()
                                    .border_color(theme.border)
                                    .rounded_md()
                                    .flex()
                                    .items_center()
                                    .track_focus(&self.focus_handle)
                                    .cursor(CursorStyle::IBeam)
                                    .on_mouse_down(MouseButton::Left, cx.listener(|this, _, window, cx| {
                                        this.focus_handle.focus(window, cx);
                                    }))
                                    .on_key_down(cx.listener(|this, event: &KeyDownEvent, _, cx| {
                                        let key = &event.keystroke.key;
                                        if key == "backspace" {
                                            this.search_query.pop();
                                            this.scroll_offset = 0;
                                            cx.notify();
                                        } else if key == "escape" {
                                            this.search_query.clear();
                                            this.scroll_offset = 0;
                                            cx.notify();
                                        } else if !event.keystroke.modifiers.control && !event.keystroke.modifiers.alt {
                                            if key.len() == 1 {
                                                this.search_query.push_str(key);
                                                this.scroll_offset = 0;
                                                cx.notify();
                                            } else if key == "space" {
                                                this.search_query.push(' ');
                                                this.scroll_offset = 0;
                                                cx.notify();
                                            }
                                        }
                                    }))
                                    .child(
                                        div().text_xs().text_color(if self.search_query.is_empty() { theme.muted_foreground } else { theme.foreground })
                                            .child(if self.search_query.is_empty() { "Search settings...".to_string() } else { format!("{}_", self.search_query) })
                                    )
                            )
                    )
            )
            .child(
                // Master-Detail ボディ（左側グループ + 右側設定項目）
                div()
                    .flex_1()
                    .flex()
                    .flex_row()
                    .overflow_hidden()
                    .child(group_nav)
                    .child(settings_list)
            )
    }
}
