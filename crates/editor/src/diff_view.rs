//! Diff の表示（デザイン 10）。比べる元（HEAD など）と、いまの内容を行単位で並べる。
//! 塊（hunk）ごとに「元に戻す」「ステージする」ができる。ステージは開いた Extension（Git）に任せる。

use std::{ops::Range, path::PathBuf, sync::Arc};

use gpui_kit::component::Disableable as _;
use gpui_kit::{
    App, Context, EventEmitter, FocusHandle, Focusable, HighlightStyle, InteractiveElement as _, IntoElement,
    ParentElement, Render, SharedString, StatefulInteractiveElement as _, Styled, StyledText, UniformListScrollHandle,
    Window,
    assets::IconName,
    component::{
        ActiveTheme as _, Icon, Sizable as _,
        button::{Button, ButtonVariants as _},
        dock::{BasePanel, Panel, PanelEvent},
        h_flex,
        highlighter::SyntaxHighlighter,
        input::Rope,
        v_flex,
    },
    div,
    prelude::FluentBuilder as _,
    px, uniform_list,
};
use i18n::Translate as _;
use theme::{Token, TokenColors as _};

use crate::diff::{self, Hunk, LineKind};

/// これより大きいファイルは色分けしない。
const HIGHLIGHT_LIMIT: usize = 512 * 1024;

pub enum DiffViewEvent {
    /// 塊を元に戻した後の全文（Workspace がエディタかディスクに反映する）。
    Revert { path: PathBuf, text: String },
    /// この塊だけをステージするパッチ（開いた Extension に渡す）。
    StageHunk { path: PathBuf, patch: String },
    /// ファイルを開く（0 始まりの行）。
    Open { path: PathBuf, line: u32 },
}

#[derive(Clone, Copy)]
enum Row {
    Header(usize),
    Line(usize, usize),
}

struct Side {
    text: Arc<String>,
    line_starts: Vec<usize>,
    highlighter: Option<SyntaxHighlighter>,
}

impl Side {
    fn new(text: Arc<String>, language: &str) -> Side {
        let mut line_starts = vec![0];
        line_starts.extend(text.match_indices('\n').map(|(i, _)| i + 1));
        let highlighter = (text.len() <= HIGHLIGHT_LIMIT).then(|| {
            let mut h = SyntaxHighlighter::new(language);
            h.update(None, &Rope::from_str(&text), None);
            h
        });
        Side {
            text,
            line_starts,
            highlighter,
        }
    }

    /// 1 始まりの行の範囲（改行を除く）。
    fn line_range(&self, no: u32) -> Option<Range<usize>> {
        let i = (no as usize).checked_sub(1)?;
        let start = *self.line_starts.get(i)?;
        let end = self.line_starts.get(i + 1).map_or(self.text.len(), |e| e - 1);
        let end = if self.text[start..end].ends_with('\r') {
            end - 1
        } else {
            end
        };
        Some(start..end)
    }

    fn styles(&self, no: u32, cx: &App) -> Vec<(Range<usize>, HighlightStyle)> {
        let (Some(h), Some(range)) = (&self.highlighter, self.line_range(no)) else {
            return Vec::new();
        };
        if range.is_empty() {
            return Vec::new();
        }
        let theme = cx.theme().highlight_theme.clone();
        h.styles(&range, &*theme)
            .into_iter()
            .map(|(r, s)| (r.start - range.start..r.end - range.start, s))
            .collect()
    }
}

pub struct DiffView {
    focus: FocusHandle,
    path: PathBuf,
    /// リポジトリからの相対パス（パッチに使う）。
    rel: String,
    base_label: SharedString,
    language: String,
    old: Side,
    new: Side,
    hunks: Arc<Vec<Hunk>>,
    rows: Arc<Vec<Row>>,
    selected: usize,
    /// ステージを任せる Extension があるか。
    can_stage: bool,
    scroll: UniformListScrollHandle,
}

impl EventEmitter<DiffViewEvent> for DiffView {}
impl EventEmitter<PanelEvent> for DiffView {}

impl DiffView {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        path: PathBuf,
        rel: String,
        base_label: String,
        base: String,
        current: String,
        language: String,
        can_stage: bool,
        cx: &mut Context<Self>,
    ) -> Self {
        let old = Side::new(Arc::new(base), &language);
        let new = Side::new(Arc::new(current), &language);
        let mut this = DiffView {
            focus: cx.focus_handle(),
            path,
            rel,
            base_label: base_label.into(),
            language,
            old,
            new,
            hunks: Arc::new(Vec::new()),
            rows: Arc::new(Vec::new()),
            selected: 0,
            can_stage,
            scroll: UniformListScrollHandle::new(),
        };
        this.recompute();
        this
    }

    pub fn path(&self) -> &std::path::Path {
        &self.path
    }

    /// いまの内容が変わった（エディタで編集・保存した）。
    pub fn set_current(&mut self, text: String, cx: &mut Context<Self>) {
        if *self.new.text == text {
            return;
        }
        self.new = Side::new(Arc::new(text), &self.language);
        self.recompute();
        cx.notify();
    }

    /// 比べる元が変わった（ステージした、コミットした）。
    pub fn set_base(&mut self, label: String, text: String, cx: &mut Context<Self>) {
        self.base_label = label.into();
        if *self.old.text != text {
            self.old = Side::new(Arc::new(text), &self.language);
            self.recompute();
        }
        cx.notify();
    }

    fn recompute(&mut self) {
        let hunks = diff::compute(&self.old.text, &self.new.text, 3);
        let mut rows = Vec::new();
        for (hi, h) in hunks.iter().enumerate() {
            rows.push(Row::Header(hi));
            rows.extend((0..h.lines.len()).map(|li| Row::Line(hi, li)));
        }
        self.selected = self.selected.min(hunks.len().saturating_sub(1));
        self.hunks = Arc::new(hunks);
        self.rows = Arc::new(rows);
    }

    fn select(&mut self, hunk: usize, scroll: bool, cx: &mut Context<Self>) {
        if hunk >= self.hunks.len() {
            return;
        }
        self.selected = hunk;
        if scroll && let Some(ix) = self.rows.iter().position(|r| matches!(r, Row::Header(h) if *h == hunk)) {
            self.scroll.scroll_to_item(ix, gpui_kit::ScrollStrategy::Top);
        }
        cx.notify();
    }

    fn revert(&mut self, cx: &mut Context<Self>) {
        let Some(h) = self.hunks.get(self.selected) else { return };
        let text = diff::revert_hunk(&self.new.text, &self.old.text, h);
        cx.emit(DiffViewEvent::Revert {
            path: self.path.clone(),
            text,
        });
    }

    fn stage(&mut self, cx: &mut Context<Self>) {
        let Some(h) = self.hunks.get(self.selected) else { return };
        let patch = diff::hunk_patch(
            &self.rel,
            h,
            self.old.text.is_empty() || self.old.text.ends_with('\n'),
            self.new.text.is_empty() || self.new.text.ends_with('\n'),
        );
        cx.emit(DiffViewEvent::StageHunk {
            path: self.path.clone(),
            patch,
        });
    }

    fn open_at_selected(&mut self, cx: &mut Context<Self>) {
        let line = self.hunks.get(self.selected).map_or(0, |h| h.new_start as u32);
        cx.emit(DiffViewEvent::Open {
            path: self.path.clone(),
            line,
        });
    }

    fn render_row(&self, ix: usize, row: Row, cx: &mut Context<Self>) -> gpui_kit::AnyElement {
        match row {
            Row::Header(hi) => {
                let h = &self.hunks[hi];
                h_flex()
                    .id(ix)
                    .h(px(20.))
                    .w_full()
                    .px_2()
                    .bg(cx.token(Token::MutedBackground))
                    .text_color(cx.token(Token::HighlightInfo))
                    .cursor_pointer()
                    .child(h.header())
                    .on_click(cx.listener(move |this, _, _, cx| this.select(hi, false, cx)))
                    .into_any_element()
            }
            Row::Line(hi, li) => {
                let l = &self.hunks[hi].lines[li];
                let (bg, sign, sign_color) = match l.kind {
                    LineKind::Added => (Some(Token::HighlightCreatedBackground), "+", Token::HighlightCreated),
                    LineKind::Removed => (Some(Token::HighlightDeletedBackground), "-", Token::HighlightDeleted),
                    LineKind::Context => (None, " ", Token::MutedForeground),
                };
                let (side, no) = match l.kind {
                    LineKind::Removed => (&self.old, l.old_no),
                    _ => (&self.new, l.new_no),
                };
                let styles = no.map(|n| side.styles(n, cx)).unwrap_or_default();
                let text = StyledText::new(SharedString::from(l.text.clone())).with_highlights(styles);
                let selected = hi == self.selected;
                h_flex()
                    .id(ix)
                    .h(px(20.))
                    .w_full()
                    .when_some(bg, |d, t| d.bg(cx.token(t)))
                    .map(|d| {
                        // 選んでいる塊の変更行には、左に色の線を引く。
                        if selected && l.kind != LineKind::Context {
                            d.border_l_2().border_color(cx.token(sign_color))
                        } else {
                            d.pl(px(2.))
                        }
                    })
                    .child(
                        div()
                            .w(px(44.))
                            .pr_2()
                            .flex_shrink_0()
                            .text_right()
                            .text_color(cx.token(Token::HighlightEditorLineNumber))
                            .child(no.map(|n| n.to_string()).unwrap_or_default()),
                    )
                    .child(
                        div()
                            .w(px(16.))
                            .flex_shrink_0()
                            .text_color(cx.token(sign_color))
                            .child(sign),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .overflow_hidden()
                            .whitespace_nowrap()
                            .text_color(cx.token(Token::HighlightEditorForeground))
                            .child(text),
                    )
                    .on_click(cx.listener(move |this, _, _, cx| this.select(hi, false, cx)))
                    .into_any_element()
            }
        }
    }
}

impl Focusable for DiffView {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}

impl BasePanel for DiffView {
    fn panel_name(&self) -> &'static str {
        "nucleus.diff"
    }
}

impl Panel for DiffView {
    fn title(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let name = self
            .path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        h_flex()
            .gap_2()
            .child(
                Icon::new(IconName::GitCompare)
                    .small()
                    .text_color(cx.token(Token::HighlightModified)),
            )
            .child(format!("{name} ↔ {}", self.base_label))
    }

    fn inner_padding(&self, _: &App) -> bool {
        false
    }
}

impl Render for DiffView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let rows = self.rows.clone();
        let (added, removed) = self
            .hunks
            .iter()
            .fold((0, 0), |(a, r), h| (a + h.added(), r + h.removed()));
        let has_hunks = !self.hunks.is_empty();
        let selected = self.selected;
        let nav = |id: &'static str, icon: IconName, tip: &str, cx: &mut Context<Self>| {
            Button::new(id)
                .ghost()
                .xsmall()
                .icon(Icon::new(icon).text_color(cx.token(Token::MutedForeground)))
                .tooltip(cx.t(tip))
                .disabled(!has_hunks)
        };
        let body = if has_hunks {
            uniform_list(
                "diff-rows",
                rows.len(),
                cx.processor(move |this, range: Range<usize>, _, cx| {
                    range.map(|ix| this.render_row(ix, rows[ix], cx)).collect::<Vec<_>>()
                }),
            )
            .track_scroll(&self.scroll)
            .flex_1()
            .min_h_0()
            .font_family("JetBrains Mono")
            .text_size(px(13.))
            .into_any_element()
        } else {
            div()
                .flex_1()
                .p_4()
                .text_sm()
                .text_color(cx.token(Token::MutedForeground))
                .child(cx.t("core.diff.no-changes"))
                .into_any_element()
        };
        v_flex()
            .track_focus(&self.focus)
            .size_full()
            .bg(cx.token(Token::HighlightEditorBackground))
            .child(
                h_flex()
                    .h(px(28.))
                    .px_2()
                    .gap_1()
                    .justify_end()
                    .border_b_1()
                    .border_color(cx.token(Token::Border))
                    .child(
                        nav("prev", IconName::ArrowUp, "core.diff.previous-hunk", cx).on_click(
                            cx.listener(move |this, _, _, cx| this.select(selected.saturating_sub(1), true, cx)),
                        ),
                    )
                    .child(
                        nav("next", IconName::ArrowDown, "core.diff.next-hunk", cx)
                            .on_click(cx.listener(move |this, _, _, cx| this.select(selected + 1, true, cx))),
                    )
                    .child(
                        nav("open", IconName::FileCode, "core.diff.open-file", cx)
                            .on_click(cx.listener(|this, _, _, cx| this.open_at_selected(cx))),
                    ),
            )
            .child(body)
            .child(
                h_flex()
                    .h(px(32.))
                    .px_2()
                    .gap_2()
                    .border_t_1()
                    .border_color(cx.token(Token::Border))
                    .text_xs()
                    .child(
                        div()
                            .text_color(cx.token(Token::MutedForeground))
                            .child(cx.t_args("core.diff.hunks", &[("count", self.hunks.len().into())])),
                    )
                    .child(
                        div()
                            .text_color(cx.token(Token::HighlightCreated))
                            .child(format!("+{added}")),
                    )
                    .child(
                        div()
                            .text_color(cx.token(Token::HighlightDeleted))
                            .child(format!("-{removed}")),
                    )
                    .child(div().flex_1())
                    .when(self.can_stage, |d| {
                        d.child(
                            Button::new("stage-hunk")
                                .small()
                                .outline()
                                .label(cx.t("core.diff.stage-hunk"))
                                .disabled(!has_hunks)
                                .on_click(cx.listener(|this, _, _, cx| this.stage(cx))),
                        )
                    })
                    .child(
                        Button::new("revert-hunk")
                            .small()
                            .outline()
                            .label(cx.t("core.diff.revert-hunk"))
                            .disabled(!has_hunks)
                            .on_click(cx.listener(|this, _, _, cx| this.revert(cx))),
                    ),
            )
    }
}
