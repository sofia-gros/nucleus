//! ドックに置く Core のパネル（ようこそ、まだ実装していないパネル）。

use gpui_kit::{
    App, Context, EventEmitter, FocusHandle, Focusable, InteractiveElement as _, IntoElement, ParentElement, Render,
    SharedString, StatefulInteractiveElement as _, Styled, Window,
    assets::IconName,
    component::{
        Icon, Sizable as _,
        dock::{BasePanel, Panel, PanelEvent},
        h_flex, v_flex,
    },
    div,
    prelude::FluentBuilder as _,
    px,
};
use i18n::Translate as _;
use theme::{Token, TokenColors as _};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PanelKind {
    Search,
    Welcome,
    Output,
    Outline,
}

impl PanelKind {
    fn id(self) -> &'static str {
        match self {
            PanelKind::Search => "nucleus.search",
            PanelKind::Welcome => "nucleus.welcome",
            PanelKind::Output => "nucleus.output",
            PanelKind::Outline => "nucleus.outline",
        }
    }

    fn title_key(self) -> &'static str {
        match self {
            PanelKind::Search => "core.panel.search",
            PanelKind::Welcome => "core.welcome.tab",
            PanelKind::Output => "core.panel.output",
            PanelKind::Outline => "core.panel.outline",
        }
    }

    fn icon(self) -> IconName {
        match self {
            PanelKind::Search => IconName::Search,
            PanelKind::Welcome => IconName::Atom,
            PanelKind::Output => IconName::ScrollText,
            PanelKind::Outline => IconName::ListTree,
        }
    }
}

pub struct CorePanel {
    kind: PanelKind,
    focus: FocusHandle,
}

impl CorePanel {
    pub fn new(kind: PanelKind, cx: &mut Context<Self>) -> Self {
        CorePanel {
            kind,
            focus: cx.focus_handle(),
        }
    }
}

impl EventEmitter<PanelEvent> for CorePanel {}

impl Focusable for CorePanel {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}

impl BasePanel for CorePanel {
    fn panel_name(&self) -> &'static str {
        self.kind.id()
    }

    /// ドックの基本パネルは閉じられない（ようこそだけ閉じられる）。
    fn closable(&self, _: &App) -> bool {
        self.kind == PanelKind::Welcome
    }
}

impl Panel for CorePanel {
    fn title(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let color = if self.kind == PanelKind::Welcome {
            cx.token(Token::PrimaryBackground)
        } else {
            cx.token(Token::ListHighlightForeground)
        };
        h_flex()
            .gap_2()
            .child(Icon::new(self.kind.icon()).small().text_color(color))
            .child(cx.t(self.kind.title_key()))
    }

    fn inner_padding(&self, _: &App) -> bool {
        false
    }
}

impl Render for CorePanel {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        match self.kind {
            PanelKind::Welcome => welcome(cx).into_any_element(),
            other => coming_soon(other, cx).into_any_element(),
        }
    }
}

fn coming_soon(kind: PanelKind, cx: &App) -> impl IntoElement {
    let panel = cx.t(kind.title_key());
    let bg = match kind {
        PanelKind::Output => Token::Background,
        _ => Token::SidebarBackground,
    };
    v_flex()
        .size_full()
        .items_center()
        .justify_center()
        .gap_2()
        .bg(cx.token(bg))
        .child(
            Icon::new(kind.icon())
                .size(px(24.))
                .text_color(cx.token(Token::MutedForeground)),
        )
        .child(
            div()
                .text_xs()
                .text_color(cx.token(Token::MutedForeground))
                .child(cx.t_args("core.panel.coming-soon", &[("panel", panel.to_string().into())])),
        )
}

/// キーボードショートカットの表示（Kbd 風）。
fn kbd(keys: &str, cx: &App) -> impl IntoElement {
    div()
        .px_1p5()
        .h(px(20.))
        .flex()
        .items_center()
        .rounded(px(4.))
        .border_1()
        .border_color(cx.token(Token::Border))
        .bg(cx.token(Token::MutedBackground))
        .text_xs()
        .text_color(cx.token(Token::MutedForeground))
        .child(SharedString::from(keys.to_string()))
}

fn card(title_key: &str, cx: &App) -> gpui_kit::Div {
    v_flex()
        .w(px(360.))
        .p_4()
        .gap_1()
        .rounded(px(8.))
        .border_1()
        .border_color(cx.token(Token::Border))
        .bg(cx.token(Token::SidebarBackground))
        .child(
            div()
                .pb_1()
                .text_xs()
                .font_weight(gpui_kit::FontWeight::MEDIUM)
                .text_color(cx.token(Token::MutedForeground))
                .child(cx.t(title_key).to_uppercase()),
        )
}

fn action_row(
    id: &'static str,
    icon: IconName,
    label_key: &str,
    keys: Option<&str>,
    highlighted: bool,
    cx: &App,
) -> impl IntoElement {
    h_flex()
        .id(id)
        .h(px(32.))
        .px_2()
        .gap(px(10.))
        .rounded(px(6.))
        .when(highlighted, |d| d.bg(cx.token(Token::AccentBackground)))
        .hover(|d| d.bg(cx.token(Token::ListHoverBackground)))
        .child(Icon::new(icon).text_color(cx.token(Token::ListHighlightForeground)))
        .child(
            div()
                .flex_1()
                .text_sm()
                .text_color(cx.token(Token::Foreground))
                .child(cx.t(label_key)),
        )
        .when_some(keys, |d, k| d.child(kbd(k, cx)))
        .when(id == "w-open", |d| {
            d.on_click(|_, window, cx| window.dispatch_action(Box::new(crate::OpenFolder), cx))
        })
}

fn welcome(cx: &App) -> impl IntoElement {
    let start = card("core.welcome.start", cx)
        .child(action_row(
            "w-open",
            IconName::FolderOpen,
            "core.welcome.open-folder",
            Some("Ctrl+O"),
            true,
            cx,
        ))
        .child(action_row(
            "w-new",
            IconName::FileCode,
            "core.welcome.new-file",
            Some("Ctrl+N"),
            false,
            cx,
        ))
        .child(action_row(
            "w-clone",
            IconName::GitPullRequest,
            "core.welcome.clone",
            None,
            false,
            cx,
        ))
        .child(action_row(
            "w-palette",
            IconName::Command,
            "core.welcome.command-palette",
            Some("Ctrl+Shift+P"),
            false,
            cx,
        ));
    let recent = card("core.welcome.recent", cx).child(
        div()
            .py_2()
            .px_2()
            .text_sm()
            .text_color(cx.token(Token::MutedForeground))
            .child(cx.t("core.welcome.no-recent")),
    );
    let tip = |keys: &str, key: &str| {
        h_flex().gap_2().child(kbd(keys, cx)).child(
            div()
                .text_xs()
                .text_color(cx.token(Token::MutedForeground))
                .child(cx.t(key)),
        )
    };

    v_flex()
        .size_full()
        .items_center()
        .pt(px(72.))
        .gap_8()
        .bg(cx.token(Token::Background))
        .child(
            v_flex()
                .items_center()
                .gap_2()
                .child(
                    Icon::new(IconName::Atom)
                        .size(px(72.))
                        .text_color(cx.token(Token::PrimaryBackground)),
                )
                .child(
                    div()
                        .text_2xl()
                        .font_weight(gpui_kit::FontWeight::SEMIBOLD)
                        .text_color(cx.token(Token::AccentForeground))
                        .child(cx.t("core.app-name")),
                )
                .child(
                    div()
                        .text_sm()
                        .text_color(cx.token(Token::MutedForeground))
                        .child(cx.t("core.welcome.tagline")),
                ),
        )
        .child(h_flex().gap_6().items_start().child(start).child(recent))
        .child(
            h_flex()
                .gap_6()
                .child(tip("Ctrl+P", "core.welcome.tip-find-file"))
                .child(tip("Ctrl+Shift+F", "core.welcome.tip-search"))
                .child(tip("Ctrl+`", "core.welcome.tip-terminal"))
                .child(tip("Ctrl+,", "core.welcome.tip-settings")),
        )
}
