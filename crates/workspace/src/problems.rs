//! Problems パネル（下ドック）: 言語サーバーの診断をファイルごとに並べる。

use std::path::PathBuf;

use gpui_kit::{
    App, Context, EventEmitter, FocusHandle, Focusable, InteractiveElement as _, IntoElement, ParentElement, Render,
    SharedString, StatefulInteractiveElement as _, Styled, Window, div, prelude::FluentBuilder as _, px,
    assets::IconName,
    component::{
        Icon, Sizable as _,
        dock::{BasePanel, Panel, PanelEvent},
        h_flex, v_flex,
    },
};
use i18n::Translate as _;
use lsp_types::{Diagnostic, DiagnosticSeverity, Position};
use theme::{Token, TokenColors as _};

pub enum ProblemsEvent {
    Open(PathBuf, Position),
}

pub struct ProblemsPanel {
    focus: FocusHandle,
    root: Option<PathBuf>,
    items: Vec<(PathBuf, Vec<Diagnostic>)>,
}

impl ProblemsPanel {
    pub fn new(cx: &mut Context<Self>) -> Self {
        ProblemsPanel { focus: cx.focus_handle(), root: None, items: Vec::new() }
    }

    pub fn set(&mut self, root: Option<PathBuf>, items: Vec<(PathBuf, Vec<Diagnostic>)>, cx: &mut Context<Self>) {
        self.root = root;
        self.items = items;
        cx.notify();
    }

    fn count(&self) -> usize {
        self.items.iter().map(|(_, d)| d.len()).sum()
    }
}

impl EventEmitter<PanelEvent> for ProblemsPanel {}
impl EventEmitter<ProblemsEvent> for ProblemsPanel {}

impl Focusable for ProblemsPanel {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}

impl BasePanel for ProblemsPanel {
    fn panel_name(&self) -> &'static str {
        "nucleus.problems"
    }

    fn closable(&self, _: &App) -> bool {
        false
    }
}

impl Panel for ProblemsPanel {
    fn title(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let n = self.count();
        h_flex()
            .gap_2()
            .child(Icon::new(IconName::TriangleAlert).small().text_color(cx.token(Token::ListHighlightForeground)))
            .child(cx.t("core.panel.problems"))
            .when(n > 0, |d| {
                d.child(
                    div()
                        .px_1p5()
                        .rounded(px(8.))
                        .bg(cx.token(Token::SecondaryBackground))
                        .text_xs()
                        .text_color(cx.token(Token::SecondaryForeground))
                        .child(SharedString::from(n.to_string())),
                )
            })
    }

    fn inner_padding(&self, _: &App) -> bool {
        false
    }
}

impl Render for ProblemsPanel {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let mut list = v_flex().w_full().py_1();
        if self.items.is_empty() {
            list = list.child(
                div().p_3().text_sm().text_color(cx.token(Token::MutedForeground)).child(cx.t("core.problems.none")),
            );
        }
        let mut row_ix = 0usize;
        for (path, diags) in &self.items {
            let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
            let dir = path
                .parent()
                .map(|p| self.root.as_deref().and_then(|r| p.strip_prefix(r).ok()).unwrap_or(p).display().to_string())
                .unwrap_or_default();
            list = list.child(
                h_flex()
                    .h(px(24.))
                    .px_3()
                    .gap_2()
                    .child(Icon::new(IconName::ChevronDown).size(px(14.)).text_color(cx.token(Token::MutedForeground)))
                    .child(Icon::new(IconName::FileCode).small().text_color(cx.token(Token::ListHighlightForeground)))
                    .child(div().text_sm().font_weight(gpui_kit::FontWeight::MEDIUM).text_color(cx.token(Token::Foreground)).child(name))
                    .child(div().text_xs().text_color(cx.token(Token::MutedForeground)).child(dir)),
            );
            for d in diags {
                let (icon, color) = match d.severity {
                    Some(DiagnosticSeverity::WARNING) => (IconName::TriangleAlert, Token::HighlightWarning),
                    Some(DiagnosticSeverity::INFORMATION) | Some(DiagnosticSeverity::HINT) => {
                        (IconName::Info, Token::HighlightInfo)
                    }
                    _ => (IconName::CircleX, Token::HighlightError),
                };
                let message = d.message.lines().next().unwrap_or_default().to_string();
                let source = d.source.clone().unwrap_or_default();
                let at = format!("[{}, {}]", d.range.start.line + 1, d.range.start.character + 1);
                let target = (path.clone(), d.range.start);
                list = list.child(
                    h_flex()
                        .id(("problem", row_ix))
                        .h(px(24.))
                        .pl(px(32.))
                        .pr_3()
                        .gap_2()
                        .hover(|s| s.bg(cx.token(Token::ListHoverBackground)))
                        .child(Icon::new(icon).small().text_color(cx.token(color)))
                        .child(
                            div()
                                .flex_shrink()
                                .min_w_0()
                                .overflow_hidden()
                                .whitespace_nowrap()
                                .text_ellipsis()
                                .text_sm()
                                .text_color(cx.token(Token::Foreground))
                                .child(message),
                        )
                        .child(div().text_xs().text_color(cx.token(Token::MutedForeground)).child(source))
                        .child(div().text_xs().text_color(cx.token(Token::MutedForeground)).child(at))
                        .on_click(cx.listener(move |_, _, _, cx| cx.emit(ProblemsEvent::Open(target.0.clone(), target.1)))),
                );
                row_ix += 1;
            }
        }
        div()
            .id("problems")
            .track_focus(&self.focus)
            .size_full()
            .overflow_y_scroll()
            .bg(cx.token(Token::Background))
            .child(list)
    }
}
