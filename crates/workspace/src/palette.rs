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
    div,
    prelude::FluentBuilder as _,
    px,
};
use i18n::Translate as _;
use theme::{Token, TokenColors as _};

use crate::{Workspace, commands, finder};

const MAX_RESULTS: usize = 60;

thread_local! {
    static PALETTE_SEQ: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
}

/// テーマ・表示言語の選択肢: (設定に書く値, 表示名, 使用中か)。
pub(crate) fn choices(kind: PaletteKind, cx: &gpui_kit::App) -> Vec<(String, String, bool)> {
    match kind {
        PaletteKind::Themes => {
            let current = settings::ActiveSettings::get(cx).theme.clone();
            crate::settings_sync::available_themes(cx)
                .into_iter()
                .map(|t| {
                    let is_current = t.name == current;
                    (t.name.clone(), t.name, is_current)
                })
                .collect()
        }
        PaletteKind::Languages => {
            let current = cx
                .global::<i18n::I18n>()
                .localizer
                .language()
                .language
                .as_str()
                .to_string();
            let packs: Vec<(String, String)> = extension_host::ExtensionStore::get(cx)
                .map(|s| {
                    s.enabled()
                        .filter_map(|e| {
                            e.manifest
                                .locale
                                .as_ref()
                                .map(|l| (l.language.clone(), l.label.clone()))
                        })
                        .collect()
                })
                .unwrap_or_default();
            cx.global::<i18n::I18n>()
                .catalog
                .languages()
                .into_iter()
                .map(|l| {
                    let tag = l.to_string();
                    let label = match tag.as_str() {
                        "en" => "English".to_string(),
                        "ja" => "日本語".to_string(),
                        t => packs
                            .iter()
                            .find(|(p, _)| p == t)
                            .map(|(_, label)| label.clone())
                            .unwrap_or_else(|| t.to_string()),
                    };
                    let is_current = l.language.as_str() == current;
                    (tag, label, is_current)
                })
                .collect()
        }
        _ => Vec::new(),
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PaletteKind {
    Commands,
    Files,
    Themes,
    Languages,
}

pub struct Palette {
    /// パレットを開くたびに変わる番号。確定の処理が、別のパレットに効かないようにする。
    id: u64,
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
        // パレットから別のパレットを開いた場合は、最初に開く前のフォーカスを引き継ぐ。
        let prev_focus = match self.palette.take() {
            Some(old) => old.prev_focus,
            None => window.focused(cx),
        };
        let state = cx.new(|cx| CommandState::new(window, cx));
        state.update(cx, |s, cx| s.focus(window, cx));
        PALETTE_SEQ.with(|n| n.set(n.get() + 1));
        self.palette = Some(Palette {
            id: PALETTE_SEQ.with(|n| n.get()),
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
                let titles: Vec<String> = commands::all(cx).iter().map(|c| c.title.to_string()).collect();
                palette.matches = if query.trim().is_empty() {
                    (0..titles.len()).collect()
                } else {
                    finder::fuzzy_top(&titles, &query, MAX_RESULTS)
                };
                cx.notify();
            }
            PaletteKind::Themes | PaletteKind::Languages => {
                let labels: Vec<String> = choices(palette.kind, cx).into_iter().map(|c| c.1).collect();
                palette.matches = if query.trim().is_empty() {
                    (0..labels.len()).collect()
                } else {
                    finder::fuzzy_top(&labels, &query, MAX_RESULTS)
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

    fn confirm_palette(&mut self, id: u64, row: usize, window: &mut Window, cx: &mut Context<Self>) {
        // コマンドの実行で別のパレットが開いた場合は、そちらを閉じない。
        let Some(p) = self.palette.as_ref().filter(|p| p.id == id) else {
            return;
        };
        let kind = p.kind;
        let Some(&ix) = p.matches.get(row) else { return };
        self.close_palette(window, cx);
        if matches!(kind, PaletteKind::Themes | PaletteKind::Languages) {
            if let Some((value, _, _)) = choices(kind, cx).into_iter().nth(ix) {
                let key = if kind == PaletteKind::Themes {
                    "theme"
                } else {
                    "language"
                };
                self.choose_setting(key, serde_json::json!(value), window, cx);
            }
            return;
        }
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
                let all = commands::all(cx);
                p.matches
                    .iter()
                    .filter_map(|&i| all.get(i))
                    .map(|c| {
                        CommandItem::new()
                            .label(c.title.clone())
                            .icon(Icon::new(c.icon).text_color(cx.token(Token::MutedForeground)))
                            .action((c.action)())
                    })
                    .collect()
            }
            PaletteKind::Themes | PaletteKind::Languages => {
                let all = choices(p.kind, cx);
                let icon = if p.kind == PaletteKind::Themes {
                    IconName::Palette
                } else {
                    IconName::Languages
                };
                p.matches
                    .iter()
                    .filter_map(|&i| all.get(i).cloned())
                    .map(|(_, label, current)| {
                        let current_label = cx.t("core.palette.current");
                        CommandItem::new()
                            .label(SharedString::from(label.clone()))
                            .child(move |_, cx| {
                                h_flex()
                                    .w_full()
                                    .gap_2()
                                    .child(Icon::new(icon).small().text_color(cx.token(Token::MutedForeground)))
                                    .child(
                                        div()
                                            .flex_1()
                                            .text_sm()
                                            .text_color(cx.token(Token::PopoverForeground))
                                            .child(label.clone()),
                                    )
                                    .when(current, |d| {
                                        d.child(
                                            div()
                                                .text_xs()
                                                .text_color(cx.token(Token::ListHighlightForeground))
                                                .child(current_label.clone()),
                                        )
                                    })
                            })
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
            PaletteKind::Themes => cx.t("core.palette.themes-placeholder"),
            PaletteKind::Languages => cx.t("core.palette.languages-placeholder"),
        };
        let empty_key = match (p.kind, self.project.read(cx).root().is_some(), self.indexing) {
            (PaletteKind::Files, false, _) => "core.palette.no-project",
            (PaletteKind::Files, true, true) => "core.palette.indexing",
            _ => "core.palette.no-results",
        };
        let empty: SharedString = cx.t(empty_key);
        let (ws1, ws2, ws3) = (ws.clone(), ws.clone(), ws);
        let palette_id = p.id;
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
                ws2.update(cx, |ws, cx| ws.confirm_palette(palette_id, index.row, window, cx))
                    .ok();
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
