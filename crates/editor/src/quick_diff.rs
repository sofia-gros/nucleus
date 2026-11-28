//! 行番号の横の変更の印（クイック Diff）。元の内容（Git のインデックスなど）といまの内容を比べ、
//! 追加・変更・削除をガターとスクロールバーに出す。元の内容は Workspace が Extension の指定で取ってきて渡す。

use std::{sync::Arc, time::Duration};

use gpui_kit::{
    AppContext as _, Context, Hsla,
    component::input::{
        GutterDecoration, GutterDecorationCollection, OverviewRulerDecoration, OverviewRulerDecorationCollection,
        OverviewRulerLane, RopeExt as _,
    },
};
use theme::{Token, TokenColors as _};

use crate::{
    EditorPanel,
    diff::{LineChange, line_changes},
};

/// 打っている間は少し待ってから比べ直す。
const DEBOUNCE: Duration = Duration::from_millis(150);

#[derive(Default)]
pub(crate) struct QuickDiff {
    base: Option<Arc<String>>,
    gutter: Option<GutterDecorationCollection>,
    ruler: Option<OverviewRulerDecorationCollection>,
    task: Option<gpui_kit::Task<()>>,
}

impl EditorPanel {
    /// 比べる元を設定する（`None` なら印を消す）。
    pub fn set_diff_base(&mut self, base: Option<String>, cx: &mut Context<Self>) {
        let base = base.map(Arc::new);
        if self.quick_diff.base.as_deref() == base.as_deref() {
            return;
        }
        self.quick_diff.base = base;
        self.update_quick_diff(Duration::ZERO, cx);
    }

    /// 内容が変わったときに呼ぶ。
    pub(crate) fn quick_diff_changed(&mut self, cx: &mut Context<Self>) {
        if self.quick_diff.base.is_some() {
            self.update_quick_diff(DEBOUNCE, cx);
        }
    }

    fn update_quick_diff(&mut self, delay: Duration, cx: &mut Context<Self>) {
        let Some(base) = self.quick_diff.base.clone() else {
            self.apply_marks(Vec::new(), cx);
            return;
        };
        let text = self.state.read(cx).value().to_string();
        self.quick_diff.task = Some(cx.spawn(async move |this, cx| {
            if !delay.is_zero() {
                cx.background_executor().timer(delay).await;
            }
            let changes = cx.background_spawn(async move { line_changes(&base, &text) }).await;
            this.update(cx, |this, cx| this.apply_marks(changes, cx)).ok();
        }));
    }

    fn apply_marks(&mut self, changes: Vec<(u32, u32, LineChange)>, cx: &mut Context<Self>) {
        let color = |c: LineChange, cx: &gpui_kit::App| -> Hsla {
            cx.token(match c {
                LineChange::Added => Token::HighlightCreated,
                LineChange::Modified => Token::HighlightModified,
                LineChange::Deleted => Token::HighlightDeleted,
            })
        };
        let (gutter, ruler): (Vec<_>, Vec<_>) = {
            let state = self.state.read(cx);
            let text = state.text();
            let lines = text.lines_len();
            changes
                .iter()
                .map(|&(line, count, change)| {
                    let line = (line as usize).min(lines.saturating_sub(1));
                    let start = text.line_start_offset(line);
                    let end = if count == 0 {
                        start
                    } else {
                        text.line_end_offset((line + count as usize - 1).min(lines.saturating_sub(1)))
                    };
                    let c = color(change, cx);
                    (
                        GutterDecoration::new(start..end).with_color(c),
                        OverviewRulerDecoration::new(start..end)
                            .with_lane(OverviewRulerLane::Left)
                            .with_color(c),
                    )
                })
                .unzip()
        };
        match (&self.quick_diff.gutter, &self.quick_diff.ruler) {
            (Some(g), Some(r)) => {
                let (g, r) = (g.clone(), r.clone());
                g.set(gutter, cx);
                r.set(ruler, cx);
            }
            _ => {
                let (g, r) = self.state.update(cx, |s, cx| {
                    (
                        s.create_gutter_decorations_collection(gutter, cx),
                        s.create_overview_ruler_decorations_collection(ruler, cx),
                    )
                });
                self.quick_diff.gutter = Some(g);
                self.quick_diff.ruler = Some(r);
            }
        }
    }
}
