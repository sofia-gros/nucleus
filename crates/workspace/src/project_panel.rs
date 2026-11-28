//! プロジェクトツリー（左ドック）。見えている行だけを描く仮想リスト。

use std::{path::PathBuf, sync::Arc, time::Duration};

use futures::StreamExt as _;
use gpui_kit::{
    App, AppContext as _, Context, EventEmitter, FocusHandle, Focusable, InteractiveElement as _, IntoElement,
    ParentElement, Render, SharedString, StatefulInteractiveElement as _, Styled, Task, UniformListScrollHandle,
    Window,
    assets::IconName,
    component::{
        Icon, Sizable as _,
        button::{Button, ButtonVariants as _},
        dock::{BasePanel, Panel, PanelEvent},
        h_flex,
        input::Input,
        menu::ContextMenuExt as _,
        v_flex,
    },
    div,
    prelude::FluentBuilder as _,
    px, uniform_list,
};
use i18n::Translate as _;
use project::{DirWatcher, EntryKind, ProjectTree, Row};
use theme::{Token, TokenColors as _};

/// ツリーに並べる1行。
#[derive(Clone, Copy)]
enum Slot {
    Row(usize),
    /// 新しく作るものの名前を入れる行（深さ）。
    NewInput(usize),
    /// 入力の誤りを出す行（深さ）。
    Error(usize),
}

pub enum ProjectPanelEvent {
    OpenFile(PathBuf),
    OpenFolderRequested,
    /// 名前を変えた（前, 後）。
    Renamed(PathBuf, PathBuf),
    /// ごみ箱に移した。
    Deleted(PathBuf),
    /// このフォルダでターミナルを開く。
    OpenTerminal(PathBuf),
}

pub struct ProjectPanel {
    focus: FocusHandle,
    tree: Option<ProjectTree>,
    rows: Arc<Vec<Row>>,
    pub(crate) selected: Option<PathBuf>,
    watcher: Option<DirWatcher>,
    scroll: UniformListScrollHandle,
    /// 右クリックした項目（パス, フォルダか）。
    context_target: Option<(PathBuf, bool)>,
    /// ツリーの中で入力している名前（新しいファイル・フォルダ、名前の変更）。
    pub(crate) editing: Option<crate::project_actions::Edit>,
    _watch_task: Option<Task<()>>,
}

impl ProjectPanel {
    pub fn new(cx: &mut Context<Self>) -> Self {
        ProjectPanel {
            focus: cx.focus_handle(),
            tree: None,
            rows: Arc::new(Vec::new()),
            selected: None,
            watcher: None,
            scroll: UniformListScrollHandle::new(),
            context_target: None,
            editing: None,
            _watch_task: None,
        }
    }

    pub fn root(&self) -> Option<PathBuf> {
        self.tree.as_ref().map(|t| t.root().to_path_buf())
    }

    pub fn set_root(&mut self, root: PathBuf, cx: &mut Context<Self>) {
        let (tree, to_load) = ProjectTree::new(root);
        self.rows = tree.rows();
        self.tree = Some(tree);
        self.selected = None;
        match DirWatcher::new() {
            Ok((watcher, mut rx)) => {
                self.watcher = Some(watcher);
                // 変更通知は 100ms まとめてから、該当フォルダだけを読み直す。
                self._watch_task = Some(cx.spawn(async move |this, cx| {
                    while let Some(first) = rx.next().await {
                        let mut dirs = vec![first];
                        cx.background_executor().timer(Duration::from_millis(100)).await;
                        while let Ok(d) = rx.try_recv() {
                            dirs.push(d);
                        }
                        dirs.sort();
                        dirs.dedup();
                        let ok = this.update(cx, |this, cx| {
                            for d in dirs {
                                if this.tree.as_mut().is_some_and(|t| t.invalidate(&d)) {
                                    this.load(d, cx);
                                }
                            }
                        });
                        if ok.is_err() {
                            break;
                        }
                    }
                }));
            }
            Err(e) => log::warn!("ファイルの監視を始められない: {e}"),
        }
        self.load(to_load, cx);
        cx.notify();
    }

    /// フォルダを開く（閉じていれば。読み込みが要れば読む）。
    pub(crate) fn expand(&mut self, dir: &std::path::Path, cx: &mut Context<Self>) {
        let Some(tree) = self.tree.as_mut() else { return };
        if dir == tree.root() || tree.is_expanded(dir) {
            return;
        }
        if let Some(to_load) = tree.toggle(dir) {
            self.load(to_load, cx);
        }
        if let Some(t) = &self.tree {
            self.rows = t.rows();
        }
    }

    /// 表示する行（ツリーの行と、新しく作るときの入力欄・エラーの行）。
    fn slots(&self) -> Vec<Slot> {
        let mut out: Vec<Slot> = (0..self.rows.len()).map(Slot::Row).collect();
        let Some(edit) = &self.editing else { return out };
        let (at, depth) = match edit.kind {
            crate::project_actions::EditKind::Rename => {
                let Some(ix) = edit
                    .target
                    .as_ref()
                    .and_then(|t| self.rows.iter().position(|r| &r.path == t))
                else {
                    return out;
                };
                (ix + 1, None)
            }
            _ => {
                let parent = self.rows.iter().position(|r| r.path == edit.dir);
                let depth = parent.map_or(1, |p| self.rows[p].depth + 1);
                let at = parent.map_or(0, |p| p + 1);
                out.insert(at, Slot::NewInput(depth));
                (at + 1, Some(depth))
            }
        };
        if edit.error.is_some() {
            let depth = depth.unwrap_or_else(|| self.rows.get(at - 1).map_or(1, |r| r.depth));
            out.insert(at, Slot::Error(depth));
        }
        out
    }

    /// 名前を入れる行（新しく作るとき）。
    fn render_input_row(&self, ix: usize, depth: usize, cx: &mut Context<Self>) -> gpui_kit::AnyElement {
        let Some(edit) = &self.editing else {
            return div().into_any_element();
        };
        let folder = edit.kind == crate::project_actions::EditKind::NewFolder;
        h_flex()
            .id(("new", ix))
            .h(px(24.))
            .w_full()
            .pl(px(8. + depth as f32 * 16.))
            .pr_2()
            .gap_1()
            .child(div().w(px(14.)).flex_none())
            .child(
                Icon::new(if folder { IconName::Folder } else { IconName::FileCode })
                    .small()
                    .text_color(cx.token(if folder {
                        Token::SidebarForeground
                    } else {
                        Token::ListHighlightForeground
                    })),
            )
            .child(self.render_edit_input(cx))
            .into_any_element()
    }

    /// 入力欄そのもの（Esc で取り消す）。
    fn render_edit_input(&self, cx: &mut Context<Self>) -> gpui_kit::AnyElement {
        let Some(edit) = &self.editing else {
            return div().into_any_element();
        };
        div()
            .flex_1()
            .min_w_0()
            .on_key_down(cx.listener(|this, e: &gpui_kit::KeyDownEvent, _, cx| {
                if e.keystroke.key == "escape" {
                    this.cancel_edit(cx);
                    cx.stop_propagation();
                }
            }))
            .child(Input::new(&edit.input).xsmall().when(edit.error.is_some(), |i| {
                i.border_color(cx.token(Token::HighlightError))
            }))
            .into_any_element()
    }

    fn render_error_row(&self, ix: usize, depth: usize, cx: &mut Context<Self>) -> gpui_kit::AnyElement {
        let message = self.editing.as_ref().and_then(|e| e.error.clone()).unwrap_or_default();
        div()
            .id(("error", ix))
            .h(px(24.))
            .w_full()
            .pl(px(8. + depth as f32 * 16. + 34.))
            .pr_2()
            .flex()
            .items_center()
            .text_xs()
            .text_color(cx.token(Token::HighlightError))
            .overflow_hidden()
            .whitespace_nowrap()
            .text_ellipsis()
            .child(message)
            .into_any_element()
    }

    /// フォルダを読み直す（右クリックの操作の後。ファイルの監視より早く見せるため）。
    pub(crate) fn refresh_dir(&mut self, dir: &std::path::Path, cx: &mut Context<Self>) {
        if self.tree.as_mut().is_some_and(|t| t.invalidate(dir)) {
            self.load(dir.to_path_buf(), cx);
        }
    }

    /// フォルダの直下をバックグラウンドで読み、結果を反映する。
    fn load(&mut self, dir: PathBuf, cx: &mut Context<Self>) {
        let read = {
            let dir = dir.clone();
            cx.background_spawn(async move { project::read_dir(&dir) })
        };
        cx.spawn(async move |this, cx| {
            let result = read.await;
            this.update(cx, |this, cx| {
                let ok = result.is_ok();
                if let Some(tree) = this.tree.as_mut() {
                    tree.set_children(&dir, result);
                    this.rows = tree.rows();
                }
                if ok && let Some(w) = this.watcher.as_mut() {
                    w.watch(&dir);
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    fn click(&mut self, ix: usize, cx: &mut Context<Self>) {
        let Some(row) = self.rows.get(ix).cloned() else { return };
        self.selected = Some(row.path.clone());
        match row.kind {
            EntryKind::Dir => {
                if let Some(to_load) = self.tree.as_mut().and_then(|t| t.toggle(&row.path)) {
                    self.load(to_load, cx);
                }
                if let Some(t) = &self.tree {
                    self.rows = t.rows();
                }
            }
            EntryKind::File => cx.emit(ProjectPanelEvent::OpenFile(row.path)),
        }
        cx.notify();
    }

    fn render_row(&self, ix: usize, row: &Row, cx: &mut Context<Self>) -> gpui_kit::AnyElement {
        let selected = self.selected.as_ref() == Some(&row.path);
        let is_dir = row.kind == EntryKind::Dir;
        let chevron = if is_dir {
            Some(if row.expanded {
                IconName::ChevronDown
            } else {
                IconName::ChevronRight
            })
        } else {
            None
        };
        let icon = match (is_dir, row.expanded) {
            (true, true) => IconName::FolderOpen,
            (true, false) => IconName::Folder,
            _ => IconName::FileCode,
        };
        let icon_color = if is_dir {
            Token::SidebarForeground
        } else {
            Token::ListHighlightForeground
        };
        h_flex()
            .id(ix)
            .h(px(24.))
            .w_full()
            .pl(px(8. + row.depth as f32 * 16.))
            .pr_2()
            .gap_1()
            .border_l_2()
            .border_color(if selected {
                cx.token(Token::ListActiveBorder)
            } else {
                cx.token(Token::TabBackground)
            })
            .when(selected, |d| d.bg(cx.token(Token::ListActiveBackground)))
            .when(!selected, |d| d.hover(|d| d.bg(cx.token(Token::ListHoverBackground))))
            .child(div().w(px(14.)).flex_none().when_some(chevron, |d, c| {
                d.child(Icon::new(c).size(px(14.)).text_color(cx.token(Token::MutedForeground)))
            }))
            .child(Icon::new(icon).small().text_color(cx.token(icon_color)))
            .map(|d| {
                // 名前を変えている行は、名前の代わりに入力欄を出す。
                let renaming = self.editing.as_ref().is_some_and(|e| {
                    e.kind == crate::project_actions::EditKind::Rename && e.target.as_ref() == Some(&row.path)
                });
                if renaming {
                    return d.child(self.render_edit_input(cx));
                }
                d.child(
                    div()
                        .flex_1()
                        .overflow_hidden()
                        .whitespace_nowrap()
                        .text_ellipsis()
                        .text_sm()
                        .when(row.depth == 0, |d| d.font_weight(gpui_kit::FontWeight::MEDIUM))
                        .text_color(if selected {
                            cx.token(Token::SidebarAccentForeground)
                        } else {
                            cx.token(Token::SidebarForeground)
                        })
                        .child(SharedString::from(row.name.clone())),
                )
            })
            .when(row.loading, |d| {
                d.child(
                    div()
                        .text_xs()
                        .text_color(cx.token(Token::MutedForeground))
                        .child(cx.t("core.project.loading")),
                )
            })
            .on_click(cx.listener(move |this, _, _, cx| this.click(ix, cx)))
            .on_mouse_down(gpui_kit::MouseButton::Right, {
                let path = row.path.clone();
                cx.listener(move |this, _, _, _| this.context_target = Some((path.clone(), is_dir)))
            })
            .into_any_element()
    }

    fn render_empty(&self, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .size_full()
            .items_center()
            .justify_center()
            .gap_3()
            .p_6()
            .child(
                Icon::new(IconName::Folder)
                    .size(px(32.))
                    .text_color(cx.token(Token::MutedForeground)),
            )
            .child(
                div()
                    .text_sm()
                    .font_weight(gpui_kit::FontWeight::MEDIUM)
                    .text_color(cx.token(Token::SidebarForeground))
                    .child(cx.t("core.project.empty-title")),
            )
            .child(
                div()
                    .text_xs()
                    .text_center()
                    .text_color(cx.token(Token::MutedForeground))
                    .child(cx.t("core.project.empty-body")),
            )
            .child(
                Button::new("open-folder")
                    .primary()
                    .label(cx.t("core.project.open-folder"))
                    .on_click(cx.listener(|_, _, _, cx| cx.emit(ProjectPanelEvent::OpenFolderRequested))),
            )
    }
}

impl EventEmitter<PanelEvent> for ProjectPanel {}
impl EventEmitter<ProjectPanelEvent> for ProjectPanel {}

impl Focusable for ProjectPanel {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}

impl BasePanel for ProjectPanel {
    fn panel_name(&self) -> &'static str {
        "nucleus.project"
    }

    fn closable(&self, _: &App) -> bool {
        false
    }
}

impl Panel for ProjectPanel {
    fn title(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        h_flex()
            .gap_2()
            .child(
                Icon::new(IconName::ListTree)
                    .small()
                    .text_color(cx.token(Token::ListHighlightForeground)),
            )
            .child(cx.t("core.panel.project"))
    }

    fn inner_padding(&self, _: &App) -> bool {
        false
    }
}

impl Render for ProjectPanel {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let body = if self.tree.is_none() {
            self.render_empty(cx).into_any_element()
        } else {
            let rows = self.rows.clone();
            let slots = self.slots();
            uniform_list(
                "project-tree",
                slots.len(),
                cx.processor(move |this, range: std::ops::Range<usize>, _, cx| {
                    let mut items = Vec::with_capacity(range.len());
                    for ix in range {
                        items.push(match slots[ix] {
                            Slot::Row(r) => this.render_row(r, &rows[r], cx),
                            Slot::NewInput(depth) => this.render_input_row(ix, depth, cx),
                            Slot::Error(depth) => this.render_error_row(ix, depth, cx),
                        });
                    }
                    items
                }),
            )
            .track_scroll(&self.scroll)
            .size_full()
            .pt_1()
            .into_any_element()
        };
        // 右クリックメニューはパネル全体に1つ付け、どの行で押したかを覚えておく
        // （仮想リストの行の中に付けると、メニューのクリックが届かない）。
        let this = cx.entity().downgrade();
        let focus = self.focus.clone();
        div()
            .id("project-panel")
            .track_focus(&self.focus)
            .on_action(
                cx.listener(|this, a: &crate::project_actions::TreeAction, window, cx| this.tree_action(a, window, cx)),
            )
            // 行より先に呼ばれるので、空いているところで押したらルートを対象にする。
            .capture_any_mouse_down(cx.listener(|this, e: &gpui_kit::MouseDownEvent, _, _| {
                if e.button == gpui_kit::MouseButton::Right {
                    this.context_target = this.root().map(|r| (r, true));
                }
            }))
            .size_full()
            .bg(cx.token(Token::SidebarBackground))
            .child(body)
            .context_menu(move |m, _, cx| {
                let target = this.upgrade().and_then(|p| p.read(cx).context_target.clone());
                match target {
                    Some((path, is_dir)) => {
                        let is_root = this.upgrade().and_then(|p| p.read(cx).root()).as_ref() == Some(&path);
                        crate::project_actions::menu(m.action_context(focus.clone()), &path, is_dir, is_root, cx)
                    }
                    None => m,
                }
            })
    }
}
