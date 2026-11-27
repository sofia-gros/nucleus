//! ワークスペース（メインウィンドウ）の骨格: TitleBar / DockArea / StatusBar。

mod assets;
mod panels;

use gpui_kit::{
    App, AppContext as _, Context, Entity, InteractiveElement as _, IntoElement, KeyBinding, ParentElement, Render,
    Styled, Window, WindowBounds, WindowOptions, actions,
    assets::IconName,
    component::{
        Icon, Sizable as _, TitleBar,
        button::{Button, ButtonVariants as _},
        dock::{DockArea, DockLayout, DockPlacement, DockSkin, panel_handle},
        h_flex,
        status_bar::StatusBar,
        v_flex,
    },
    div, px, size,
};
use i18n::Translate as _;
use theme::{Token, TokenColors as _};

pub use assets::{Assets, fonts};
use panels::{CorePanel, PanelKind};

actions!(workspace, [ToggleLeftDock, ToggleBottomDock, ToggleRightDock, Quit]);

const CONTEXT: &str = "Workspace";

pub fn init(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("ctrl-b", ToggleLeftDock, Some(CONTEXT)),
        KeyBinding::new("ctrl-j", ToggleBottomDock, Some(CONTEXT)),
        KeyBinding::new("ctrl-alt-b", ToggleRightDock, Some(CONTEXT)),
        KeyBinding::new("ctrl-q", Quit, None),
    ]);
    cx.on_action(|_: &Quit, cx| cx.quit());
}

/// メインウィンドウを開く。
pub fn open_main_window(cx: &mut App) -> anyhow::Result<()> {
    let options = WindowOptions {
        window_bounds: Some(WindowBounds::centered(None, size(px(1440.), px(900.)), cx)),
        window_min_size: Some(size(px(800.), px(500.))),
        ..TitleBar::window_options()
    };
    gpui_kit::open_window(options, cx, |window, cx| cx.new(|cx| Workspace::new(window, cx)))?;
    Ok(())
}

pub struct Workspace {
    dock_area: Entity<DockArea>,
}

impl Workspace {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let panel = |kind, window: &mut Window, cx: &mut Context<DockArea>| {
            let _ = window;
            panel_handle(cx.new(|cx| CorePanel::new(kind, cx)))
        };
        let dock_area = cx.new(|cx| {
            let mut area = DockArea::new("nucleus.workspace", Some(1), window, cx).with_renderer(DockSkin::new(cx));
            let project = panel(PanelKind::Project, window, cx);
            let search = panel(PanelKind::Search, window, cx);
            let welcome = panel(PanelKind::Welcome, window, cx);
            let terminal = panel(PanelKind::Terminal, window, cx);
            let problems = panel(PanelKind::Problems, window, cx);
            let output = panel(PanelKind::Output, window, cx);
            let git = panel(PanelKind::Git, window, cx);
            let outline = panel(PanelKind::Outline, window, cx);

            area.set_center(DockLayout::tabs().panel_view(welcome, cx), window, cx);
            area.set_dock(
                DockPlacement::Left,
                DockLayout::tabs().panel_view(project, cx).panel_view(search, cx),
                window,
                cx,
            );
            area.set_dock(
                DockPlacement::Bottom,
                DockLayout::tabs()
                    .panel_view(terminal, cx)
                    .panel_view(problems, cx)
                    .panel_view(output, cx),
                window,
                cx,
            );
            area.set_dock(
                DockPlacement::Right,
                DockLayout::tabs().panel_view(git, cx).panel_view(outline, cx),
                window,
                cx,
            );
            area.set_dock_size(DockPlacement::Left, px(280.), window, cx);
            area.set_dock_size(DockPlacement::Bottom, px(240.), window, cx);
            area.set_dock_size(DockPlacement::Right, px(320.), window, cx);
            // デザイン（02 Editor — main）と同じく、右ドックは閉じた状態で始める。
            area.toggle_dock(DockPlacement::Right, window, cx);
            area
        });
        Workspace { dock_area }
    }

    fn toggle(&mut self, placement: DockPlacement, window: &mut Window, cx: &mut Context<Self>) {
        self.dock_area
            .update(cx, |area, cx| area.toggle_dock(placement, window, cx));
    }

    fn render_title_bar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let menus = ["file", "edit", "view", "selection", "find", "extensions", "help"];
        let dock_button =
            |id: &'static str, icon: IconName, key: &str, placement: DockPlacement, cx: &mut Context<Self>| {
                Button::new(id)
                    .ghost()
                    .small()
                    .icon(Icon::new(icon).text_color(cx.token(Token::MutedForeground)))
                    .tooltip(cx.t(key))
                    .on_click(cx.listener(move |this, _, window, cx| this.toggle(placement, window, cx)))
            };
        TitleBar::new()
            .child(
                h_flex()
                    .gap_0p5()
                    .child(
                        div().pr_2().child(
                            Icon::new(IconName::Atom)
                                .size(px(18.))
                                .text_color(cx.token(Token::PrimaryBackground)),
                        ),
                    )
                    .children(menus.map(|m| Button::new(m).ghost().small().label(cx.t(&format!("core.menu.{m}"))))),
            )
            .child(
                div()
                    .flex_1()
                    .flex()
                    .justify_center()
                    .text_sm()
                    .text_color(cx.token(Token::MutedForeground))
                    .child(cx.t("core.window-title-empty")),
            )
            .child(
                h_flex()
                    .gap_1()
                    .pr_2()
                    .child(dock_button(
                        "toggle-left",
                        IconName::PanelLeft,
                        "core.dock.toggle-left",
                        DockPlacement::Left,
                        cx,
                    ))
                    .child(dock_button(
                        "toggle-bottom",
                        IconName::PanelBottom,
                        "core.dock.toggle-bottom",
                        DockPlacement::Bottom,
                        cx,
                    ))
                    .child(dock_button(
                        "toggle-right",
                        IconName::PanelRight,
                        "core.dock.toggle-right",
                        DockPlacement::Right,
                        cx,
                    )),
            )
    }

    fn render_status_bar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let item = |icon: Option<IconName>, label: String, cx: &App| {
            h_flex()
                .gap_1()
                .px_2()
                .text_xs()
                .text_color(cx.token(Token::MutedForeground))
                .children(icon.map(|i| Icon::new(i).xsmall().text_color(cx.token(Token::MutedForeground))))
                .child(label)
        };
        StatusBar::new()
            .left(item(
                Some(IconName::CircleX),
                cx.t_args("core.status.errors", &[("count", 0.into())]).to_string(),
                cx,
            ))
            .left(item(
                Some(IconName::TriangleAlert),
                cx.t_args("core.status.warnings", &[("count", 0.into())]).to_string(),
                cx,
            ))
            .right(item(Some(IconName::Bell), String::new(), cx))
    }
}

impl Render for Workspace {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .key_context(CONTEXT)
            .on_action(cx.listener(|this, _: &ToggleLeftDock, window, cx| this.toggle(DockPlacement::Left, window, cx)))
            .on_action(
                cx.listener(|this, _: &ToggleBottomDock, window, cx| this.toggle(DockPlacement::Bottom, window, cx)),
            )
            .on_action(
                cx.listener(|this, _: &ToggleRightDock, window, cx| this.toggle(DockPlacement::Right, window, cx)),
            )
            .size_full()
            .bg(cx.token(Token::Background))
            .text_color(cx.token(Token::Foreground))
            .font_family(cx.global::<theme::ActiveTheme>().resolved.fonts.ui.clone())
            .child(self.render_title_bar(cx))
            .child(div().flex_1().min_h_0().child(self.dock_area.clone()))
            .child(self.render_status_bar(cx))
    }
}
