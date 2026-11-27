//! コマンドパレット（Ctrl+Shift+P）とファイル検索（Ctrl+P）。gpui-kit の Command 部品で描く。
//! 絞り込みは自前のあいまい検索（ファイルはバックグラウンド）で行い、結果だけを渡す。

use std::sync::Arc;

use gpui_kit::{
    AppContext as _, Context, Entity, FocusHandle, Focusable as _, InteractiveElement as _, IntoElement, ParentElement,
    SharedString, Styled, Task, Window,
    assets::IconName,
    component::{
        Icon, Sizable as _,
        command::{Command, CommandItem, CommandState},
        h_flex,
    },
    div, px,
};
use i18n::Translate as _;
use theme::{Token, TokenColors as _};

use crate::{Workspace, commands, finder};

const MAX_RESULTS: usize = 60;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PaletteKind {
    Commands,
    Files,
}

pub struct Palette {
    pub kind: PaletteKind,
    state: Entity<CommandState>,
    prev_focus: Option<FocusHandle>,
    query: String,
    /// Commands: commands::all() の添字 / Files: file_index の添字
    matches: Vec<usize>,
    _match_task: Option<Task<()>>,
}

impl Workspace {
    pub(crate) fn toggle_palette(&mut self, kind: PaletteKind, window: &mut Window, cx: &mut Context<Self>) {
        if self.palette.as_ref().is_some_and(|p| p.kind == kind) {
            self.close_palette(window, cx);
            return;
        }
        let state = cx.new(|cx| CommandState::new(window, cx));
        state.update(cx, |s, cx| s.focus(window, cx));
        let prev_focus = window.focused(cx);
        self.palette = Some(Palette {
            kind,
            state,
            prev_focus,
            query: String::new(),
            matches: Vec::new(),
            _match_task: None,
        });
        if kind == PaletteKind::Files {
            self.refresh_file_index(cx);
        }
        self.update_palette_query(String::new(), cx);
        cx.notify();
    }

    pub(crate) fn close_palette(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(p) = self.palette.take() {
            // アクションがフォーカスを移していなければ、開く前の場所に戻す。
            let palette_focused = p.state.read(cx).focus_handle(cx).contains_focused(window, cx);
            if palette_focused || window.focused(cx).is_none() {
                match p.prev_focus {
                    Some(f) => f.focus(window, cx),
                    None => self.focus.focus(window, cx),
                }
            }
            cx.notify();
        }
    }

    /// プロジェクト内のファイル一覧をバックグラウンドで作り直す。
    pub(crate) fn refresh_file_index(&mut self, cx: &mut Context<Self>) {
        let Some(root) = self.project.read(cx).root() else {
            return;
        };
        self.indexing = true;
        let list = cx.background_spawn(async move { finder::list_files(&root) });
        cx.spawn(async move |this, cx| {
            let files = list.await;
            this.update(cx, |this, cx| {
                this.indexing = false;
                this.file_index = Some(Arc::new(files));
                let q = this.palette.as_ref().map(|p| p.query.clone()).unwrap_or_default();
                this.update_palette_query(q, cx);
            })
            .ok();
        })
        .detach();
    }

    fn update_palette_query(&mut self, query: String, cx: &mut Context<Self>) {
        let Some(palette) = self.palette.as_mut() else { return };
        palette.query = query.clone();
        match palette.kind {
            PaletteKind::Commands => {
                let titles: Vec<String> = commands::all().iter().map(|c| cx.t(c.title_key).to_string()).collect();
                palette.matches = if query.trim().is_empty() {
                    (0..titles.len()).collect()
                } else {
                    finder::fuzzy_top(&titles, &query, MAX_RESULTS)
                };
                cx.notify();
            }
            PaletteKind::Files => {
                let Some(index) = self.file_index.clone() else { return };
                let task = cx.background_spawn(async move { finder::fuzzy_top(&index, &query, MAX_RESULTS) });
                palette._match_task = Some(cx.spawn(async move |this, cx| {
                    let matches = task.await;
                    this.update(cx, |this, cx| {
                        if let Some(p) = this.palette.as_mut() {
                            p.matches = matches;
                            cx.notify();
                        }
                    })
                    .ok();
                }));
            }
        }
    }

    fn confirm_palette(&mut self, row: usize, window: &mut Window, cx: &mut Context<Self>) {
        let Some(p) = self.palette.as_ref() else { return };
        let kind = p.kind;
        let Some(&ix) = p.matches.get(row) else { return };
        self.close_palette(window, cx);
        if kind == PaletteKind::Files
            && let (Some(index), Some(root)) = (self.file_index.clone(), self.project.read(cx).root())
            && let Some(rel) = index.get(ix)
        {
            self.open_file(root.join(rel), window, cx);
        }
    }

    pub(crate) fn render_palette(&self, cx: &mut Context<Self>) -> Option<impl IntoElement> {
        let p = self.palette.as_ref()?;
        let ws = cx.entity().downgrade();
        let items: Vec<CommandItem> = match p.kind {
            PaletteKind::Commands => {
                let all = commands::all();
                p.matches
                    .iter()
                    .filter_map(|&i| all.get(i))
                    .map(|c| {
                        CommandItem::new()
                            .label(cx.t(c.title_key))
                            .icon(Icon::new(c.icon).text_color(cx.token(Token::MutedForeground)))
                            .action((c.action)())
                    })
                    .collect()
            }
            PaletteKind::Files => {
                let index = self.file_index.clone().unwrap_or_default();
                p.matches
                    .iter()
                    .filter_map(|&i| index.get(i).cloned())
                    .map(|rel| {
                        let (dir, name) = match rel.rsplit_once('/') {
                            Some((d, n)) => (d.to_string(), n.to_string()),
                            None => (String::new(), rel.clone()),
                        };
                        CommandItem::new()
                            .label(SharedString::from(rel.clone()))
                            .child(move |_, cx| {
                                h_flex()
                                    .w_full()
                                    .gap_2()
                                    .child(
                                        Icon::new(IconName::FileCode)
                                            .small()
                                            .text_color(cx.token(Token::ListHighlightForeground)),
                                    )
                                    .child(
                                        div()
                                            .text_sm()
                                            .text_color(cx.token(Token::PopoverForeground))
                                            .child(name.clone()),
                                    )
                                    .child(
                                        div()
                                            .flex_1()
                                            .overflow_hidden()
                                            .whitespace_nowrap()
                                            .text_ellipsis()
                                            .text_xs()
                                            .text_color(cx.token(Token::MutedForeground))
                                            .child(dir.clone()),
                                    )
                            })
                    })
                    .collect()
            }
        };
        let placeholder = match p.kind {
            PaletteKind::Commands => cx.t("core.palette.commands-placeholder"),
            PaletteKind::Files => cx.t("core.palette.files-placeholder"),
        };
        let empty_key = match (p.kind, self.project.read(cx).root().is_some(), self.indexing) {
            (PaletteKind::Files, false, _) => "core.palette.no-project",
            (PaletteKind::Files, true, true) => "core.palette.indexing",
            _ => "core.palette.no-results",
        };
        let empty: SharedString = cx.t(empty_key);
        let (ws1, ws2, ws3) = (ws.clone(), ws.clone(), ws);
        let command = Command::new(&p.state)
            .items(items)
            .filterable(false)
            .highlight_matches(true)
            .placeholder(placeholder)
            .max_h(px(420.))
            .bordered(false)
            .empty(move |_, _, cx| {
                div()
                    .py_6()
                    .text_center()
                    .text_sm()
                    .text_color(cx.token(Token::MutedForeground))
                    .child(empty.clone())
            })
            .on_query(move |q, _, cx| {
                let q = q.to_string();
                ws1.update(cx, |ws, cx| ws.update_palette_query(q, cx)).ok();
            })
            .on_confirm(move |index, window, cx| {
                ws2.update(cx, |ws, cx| ws.confirm_palette(index.row, window, cx)).ok();
            })
            .on_cancel(move |window, cx| {
                ws3.update(cx, |ws, cx| ws.close_palette(window, cx)).ok();
            });

        Some(
            div()
                .absolute()
                .top(px(40.))
                .left_0()
                .right_0()
                .flex()
                .justify_center()
                .child(
                    div()
                        .id("palette")
                        .occlude()
                        .w(px(640.))
                        .rounded(px(8.))
                        .border_1()
                        .border_color(cx.token(Token::Border))
                        .bg(cx.token(Token::PopoverBackground))
                        .shadow_lg()
                        .overflow_hidden()
                        .child(command),
                ),
        )
    }
}
