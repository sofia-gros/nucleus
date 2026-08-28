/// アクティビティバー（左端の縦アイコンバー）の描画コンポーネント (VSCode 精密再現版)

use gpui::*;
use gpui_component::*;
use gpui_component::theme::ActiveTheme;

pub struct ActivityBar;

impl ActivityBar {
    pub fn new() -> Self {
        Self
    }
}

impl Render for ActivityBar {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let mut top_items = Vec::new();

        // 1. Explorer アイコン
        let is_explorer_active = true; // デフォルトまたは選択状態
        let explorer_icon = div()
            .relative()
            .w_full()
            .h(gpui::px(48.0))
            .flex()
            .items_center()
            .justify_center()
            .cursor_pointer()
            .hover(|s| s.bg(cx.theme().muted.opacity(0.3)))
            .on_mouse_down(MouseButton::Left, cx.listener(|_bar, _event, _window, cx| {
                if cx.has_global::<crate::plugin_manager::PluginManagerGlobal>() {
                    let pm_global = cx.global::<crate::plugin_manager::PluginManagerGlobal>().0.clone();
                    pm_global.update(cx, |pm, _| {
                        pm.dispatch_action(crate::plugin_manager::action::PluginAction::OpenPanel { id: "explorer".to_string() });
                    });
                }
            }))
            // 左端のアクティブインジケータ (2px 白線)
            .children(if is_explorer_active {
                Some(div().absolute().left_0().top_0().bottom_0().w(gpui::px(2.0)).bg(cx.theme().foreground))
            } else {
                None
            })
            .child(
                Icon::new(IconName::File)
                    .text_color(if is_explorer_active { cx.theme().foreground } else { cx.theme().muted_foreground })
            );

        top_items.push(explorer_icon);

        // 2. プラグインアイテム (Source Control 等)
        if cx.has_global::<crate::plugin_manager::PluginManagerGlobal>() {
            let pm_global = cx.global::<crate::plugin_manager::PluginManagerGlobal>().0.clone();
            let pm = pm_global.read(cx);

            // Git 変更件数の取得
            let mut git_change_count = 0;
            if cx.has_global::<crate::settings::SettingsGlobal>() {
                let settings = cx.global::<crate::settings::SettingsGlobal>().0.read().unwrap();
                if let Some(serde_json::Value::Object(git_stats)) = settings.get("git.status") {
                    git_change_count = git_stats.len();
                }
            }

            for item in &pm.ui_registry.activity_bar_items {
                let cmd = item.command.clone();
                let is_git = item.id == "git_sidebar" || item.icon == "source_control" || item.icon == "git";

                let item_el = div()
                    .relative()
                    .w_full()
                    .h(gpui::px(48.0))
                    .flex()
                    .items_center()
                    .justify_center()
                    .cursor_pointer()
                    .hover(|s| s.bg(cx.theme().muted.opacity(0.3)))
                    .on_mouse_down(MouseButton::Left, cx.listener(move |_bar, _event, _window, cx| {
                        if !cmd.is_empty() {
                            if cx.has_global::<crate::plugin_manager::PluginManagerGlobal>() {
                                let pm_global = cx.global::<crate::plugin_manager::PluginManagerGlobal>().0.clone();
                                pm_global.update(cx, |pm, _| {
                                    pm.dispatch_event(crate::plugin_manager::event::PluginEvent::CommandExecuted { command: cmd.clone() });
                                });
                            }
                        }
                    }))
                    .child(
                        if is_git {
                            // Source Control (Git branch 分岐アイコン風)
                            div().text_sm().font_weight(FontWeight::BOLD).text_color(cx.theme().muted_foreground).child("⑂").into_any_element()
                        } else {
                            div().text_xs().font_weight(FontWeight::BOLD).text_color(cx.theme().muted_foreground).child(item.tooltip.chars().take(2).collect::<String>().to_uppercase()).into_any_element()
                        }
                    )
                    // Git 変更件数バッジ
                    .children(if is_git && git_change_count > 0 {
                        Some(
                            div()
                                .absolute()
                                .top(gpui::px(8.0))
                                .right(gpui::px(6.0))
                                .px_1()
                                .rounded_full()
                                .bg(gpui::rgb(0x007acc))
                                .text_xs()
                                .font_weight(FontWeight::BOLD)
                                .text_color(gpui::rgb(0xffffff))
                                .child(format!("{}", git_change_count))
                        )
                    } else {
                        None
                    });

                top_items.push(item_el);
            }
        }

        // 下部のアカウント & 設定アイコン
        let bottom_items = div()
            .flex()
            .flex_col()
            .w_full()
            .items_center()
            .child(
                div()
                    .w_full()
                    .h(gpui::px(40.0))
                    .flex()
                    .items_center()
                    .justify_center()
                    .cursor_pointer()
                    .hover(|s| s.bg(cx.theme().muted.opacity(0.3)))
                    .child(Icon::new(IconName::Settings).text_color(cx.theme().muted_foreground))
            );

        div()
            .w(gpui::px(48.0))
            .h_full()
            .bg(cx.theme().background)
            .border_r_1()
            .border_color(cx.theme().border)
            .flex()
            .flex_col()
            .justify_between()
            .child(div().flex().flex_col().w_full().items_center().children(top_items))
            .child(bottom_items)
    }
}
