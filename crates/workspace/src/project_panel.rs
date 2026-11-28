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
    selected: Option<PathBuf>,
    watcher: Option<DirWatcher>,
    scroll: UniformListScrollHandle,
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
            .child(
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
            .when(row.loading, |d| {
                d.child(
                    div()
                        .text_xs()
                        .text_color(cx.token(Token::MutedForeground))
                        .child(cx.t("core.project.loading")),
                )
            })
            .on_click(cx.listener(move |this, _, _, cx| this.click(ix, cx)))
            .context_menu({
                let (path, focus) = (row.path.clone(), self.focus.clone());
                move |m, _, cx| crate::project_actions::menu(m.action_context(focus.clone()), &path, is_dir, cx)
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
            uniform_list(
                "project-tree",
                rows.len(),
                cx.processor(move |this, range: std::ops::Range<usize>, _, cx| {
                    let mut items = Vec::with_capacity(range.len());
                    for ix in range {
                        items.push(this.render_row(ix, &rows[ix], cx));
                    }
                    items
                }),
            )
            .track_scroll(&self.scroll)
            .size_full()
            .pt_1()
            .into_any_element()
        };
        div()
            .track_focus(&self.focus)
            .on_action(cx.listener(|this, a: &crate::project_actions::TreeAction, window, cx| {
                this.tree_action(a, window, cx)
            }))
            .size_full()
            .bg(cx.token(Token::SidebarBackground))
            .child(body)
    }
}
