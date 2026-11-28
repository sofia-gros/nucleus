//! プロジェクト全体の検索と置換（左ドックの Search、デザイン 07）。
//! 検索はバックグラウンドで行い、入力のたびに前の検索を取り消す。結果は見えている行だけを描く。

use std::{
    collections::HashSet,
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

use gpui_kit::component::Disableable as _;
use gpui_kit::{
    App, AppContext as _, Context, Entity, EventEmitter, FocusHandle, Focusable, HighlightStyle,
    InteractiveElement as _, IntoElement, ParentElement, Render, SharedString, StatefulInteractiveElement as _, Styled,
    StyledText, Subscription, Task, UniformListScrollHandle, Window,
    assets::IconName,
    component::{
        Icon, Sizable as _,
        button::{Button, ButtonVariants as _},
        dock::{BasePanel, Panel, PanelEvent},
        h_flex,
        input::{Input, InputEvent, InputState},
        v_flex,
    },
    div,
    prelude::FluentBuilder as _,
    px, uniform_list,
};
use i18n::Translate as _;
use project::search::{self, FileMatches, SearchOptions, SearchResult};
use theme::{Token, TokenColors as _};

/// 一致の数の上限（これを超えたら止める）。
const LIMIT: usize = 5000;
const DEBOUNCE: Duration = Duration::from_millis(150);

pub enum SearchPanelEvent {
    /// 一致した場所を開く（0 始まりの行・列）。
    Open { path: PathBuf, line: u32, column: u32 },
    /// すべて置き換える（Workspace が確認してから行う）。
    ReplaceAll {
        options: SearchOptions,
        replacement: String,
        files: Vec<PathBuf>,
        count: usize,
    },
}

#[derive(Clone, Copy)]
enum RowKind {
    File(usize),
    Match(usize, usize),
}

pub struct SearchPanel {
    focus: FocusHandle,
    root: Option<PathBuf>,
    query: Entity<InputState>,
    replace: Entity<InputState>,
    include: Entity<InputState>,
    regex: bool,
    case_sensitive: bool,
    whole_word: bool,
    result: Arc<SearchResult>,
    error: Option<String>,
    searching: bool,
    collapsed: HashSet<usize>,
    rows: Arc<Vec<RowKind>>,
    selected: Option<(usize, usize)>,
    scroll: UniformListScrollHandle,
    cancel: Arc<AtomicBool>,
    _search: Option<Task<()>>,
    _subscriptions: Vec<Subscription>,
}

impl EventEmitter<SearchPanelEvent> for SearchPanel {}
impl EventEmitter<PanelEvent> for SearchPanel {}

impl SearchPanel {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let query = cx.new(|cx| InputState::new(window, cx).placeholder(cx.t("core.search.placeholder")));
        let replace = cx.new(|cx| InputState::new(window, cx).placeholder(cx.t("core.search.replace-placeholder")));
        let include = cx.new(|cx| InputState::new(window, cx).placeholder(cx.t("core.search.include-placeholder")));
        let subs = vec![
            cx.subscribe_in(&query, window, |this, _, e: &InputEvent, _, cx| match e {
                InputEvent::Change => this.schedule(DEBOUNCE, cx),
                InputEvent::PressEnter { .. } => this.schedule(Duration::ZERO, cx),
                _ => {}
            }),
            cx.subscribe_in(&include, window, |this, _, e: &InputEvent, _, cx| {
                if matches!(e, InputEvent::Change | InputEvent::PressEnter { .. }) {
                    this.schedule(DEBOUNCE, cx);
                }
            }),
            cx.subscribe_in(&replace, window, |this, _, e: &InputEvent, _, cx| {
                if let InputEvent::PressEnter { .. } = e {
                    this.replace_all(cx);
                }
            }),
        ];
        SearchPanel {
            focus: cx.focus_handle(),
            root: None,
            query,
            replace,
            include,
            regex: false,
            case_sensitive: false,
            whole_word: false,
            result: Arc::new(SearchResult::default()),
            error: None,
            searching: false,
            collapsed: HashSet::new(),
            rows: Arc::new(Vec::new()),
            selected: None,
            scroll: UniformListScrollHandle::new(),
            cancel: Arc::new(AtomicBool::new(false)),
            _search: None,
            _subscriptions: subs,
        }
    }

    pub fn set_root(&mut self, root: Option<PathBuf>, cx: &mut Context<Self>) {
        if self.root != root {
            self.root = root;
            self.schedule(Duration::ZERO, cx);
        }
    }

    /// 検索欄にフォーカスし、文字を全部選ぶ。`text` があれば入れる。
    pub fn focus_query(&mut self, text: Option<String>, window: &mut Window, cx: &mut Context<Self>) {
        self.query.update(cx, |q, cx| {
            if let Some(t) = text {
                q.set_value(t, window, cx);
            }
            q.focus(window, cx);
            q.select_all(window, cx);
        });
        self.schedule(Duration::ZERO, cx);
    }

    /// 結果を最新にする（置き換えた後など）。
    pub fn rerun(&mut self, cx: &mut Context<Self>) {
        self.schedule(Duration::ZERO, cx);
    }

    fn options(&self, cx: &App) -> SearchOptions {
        SearchOptions {
            query: self.query.read(cx).value().to_string(),
            regex: self.regex,
            case_sensitive: self.case_sensitive,
            whole_word: self.whole_word,
            include: self.include.read(cx).value().to_string(),
        }
    }

    fn schedule(&mut self, delay: Duration, cx: &mut Context<Self>) {
        self.cancel.store(true, Ordering::Relaxed);
        let cancel = Arc::new(AtomicBool::new(false));
        self.cancel = cancel.clone();
        let opts = self.options(cx);
        let Some(root) = self.root.clone() else {
            self.set_result(Ok(SearchResult::default()), cx);
            return;
        };
        if opts.query.is_empty() {
            self.searching = false;
            self.set_result(Ok(SearchResult::default()), cx);
            return;
        }
        self.searching = true;
        cx.notify();
        self._search = Some(cx.spawn(async move |this, cx| {
            if !delay.is_zero() {
                cx.background_executor().timer(delay).await;
            }
            if cancel.load(Ordering::Relaxed) {
                return;
            }
            let c = cancel.clone();
            let result = cx
                .background_spawn(async move { search::search(&root, &opts, LIMIT, &c) })
                .await;
            if cancel.load(Ordering::Relaxed) {
                return;
            }
            this.update(cx, |this, cx| {
                this.searching = false;
                this.set_result(result, cx);
            })
            .ok();
        }));
    }

    fn set_result(&mut self, result: Result<SearchResult, String>, cx: &mut Context<Self>) {
        match result {
            Ok(r) => {
                self.result = Arc::new(r);
                self.error = None;
            }
            Err(e) => {
                self.result = Arc::new(SearchResult::default());
                self.error = Some(e);
            }
        }
        self.collapsed.clear();
        self.selected = None;
        self.rebuild_rows();
        cx.notify();
    }

    fn rebuild_rows(&mut self) {
        let mut rows = Vec::new();
        for (fi, f) in self.result.files.iter().enumerate() {
            rows.push(RowKind::File(fi));
            if !self.collapsed.contains(&fi) {
                rows.extend((0..f.matches.len()).map(|mi| RowKind::Match(fi, mi)));
            }
        }
        self.rows = Arc::new(rows);
    }

    fn toggle(&mut self, which: u8, cx: &mut Context<Self>) {
        match which {
            0 => self.regex = !self.regex,
            1 => self.case_sensitive = !self.case_sensitive,
            _ => self.whole_word = !self.whole_word,
        }
        self.schedule(Duration::ZERO, cx);
    }

    fn replace_all(&mut self, cx: &mut Context<Self>) {
        if self.result.files.is_empty() {
            return;
        }
        let options = self.options(cx);
        let replacement = self.replace.read(cx).value().to_string();
        cx.emit(SearchPanelEvent::ReplaceAll {
            options,
            replacement,
            files: self.result.files.iter().map(|f| f.path.clone()).collect(),
            count: self.result.total,
        });
    }

    fn click(&mut self, row: RowKind, cx: &mut Context<Self>) {
        match row {
            RowKind::File(fi) => {
                if !self.collapsed.remove(&fi) {
                    self.collapsed.insert(fi);
                }
                self.rebuild_rows();
            }
            RowKind::Match(fi, mi) => {
                self.selected = Some((fi, mi));
                let f = &self.result.files[fi];
                let m = &f.matches[mi];
                cx.emit(SearchPanelEvent::Open {
                    path: f.path.clone(),
                    line: m.line,
                    column: m.column,
                });
            }
        }
        cx.notify();
    }

    fn render_row(&self, ix: usize, row: RowKind, cx: &mut Context<Self>) -> gpui_kit::AnyElement {
        let base = h_flex()
            .id(ix)
            .h(px(24.))
            .w_full()
            .pr_2()
            .gap_1()
            .cursor_pointer()
            .hover(|d| d.bg(cx.token(Token::ListHoverBackground)))
            .on_click(cx.listener(move |this, _, _, cx| this.click(row, cx)));
        match row {
            RowKind::File(fi) => {
                let f: &FileMatches = &self.result.files[fi];
                let (name, dir) = match f.rel.rsplit_once('/') {
                    Some((d, n)) => (n.to_string(), d.to_string()),
                    None => (f.rel.clone(), String::new()),
                };
                let collapsed = self.collapsed.contains(&fi);
                base.pl_2()
                    .child(
                        Icon::new(if collapsed {
                            IconName::ChevronRight
                        } else {
                            IconName::ChevronDown
                        })
                        .size(px(14.))
                        .text_color(cx.token(Token::MutedForeground)),
                    )
                    .child(
                        Icon::new(IconName::FileCode)
                            .small()
                            .text_color(cx.token(Token::ListHighlightForeground)),
                    )
                    .child(
                        div()
                            .flex_shrink_0()
                            .text_sm()
                            .text_color(cx.token(Token::SidebarForeground))
                            .child(name),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .overflow_hidden()
                            .whitespace_nowrap()
                            .text_ellipsis()
                            .text_xs()
                            .text_color(cx.token(Token::MutedForeground))
                            .child(dir),
                    )
                    .child(
                        div()
                            .px_1p5()
                            .rounded_full()
                            .text_xs()
                            .bg(cx.token(Token::MutedBackground))
                            .text_color(cx.token(Token::MutedForeground))
                            .child(f.count.to_string()),
                    )
                    .into_any_element()
            }
            RowKind::Match(fi, mi) => {
                let m = &self.result.files[fi].matches[mi];
                let selected = self.selected == Some((fi, mi));
                let highlight = HighlightStyle {
                    color: Some(cx.token(Token::HighlightModified)),
                    font_weight: Some(gpui_kit::FontWeight::SEMIBOLD),
                    ..Default::default()
                };
                let text = StyledText::new(SharedString::from(m.text.clone()))
                    .with_highlights(m.ranges.iter().map(|r| (r.clone(), highlight)));
                base.pl(px(30.))
                    .when(selected, |d| d.bg(cx.token(Token::ListActiveBackground)))
                    .font_family("JetBrains Mono")
                    .text_xs()
                    .child(
                        div()
                            .w(px(32.))
                            .flex_shrink_0()
                            .text_color(cx.token(Token::MutedForeground))
                            .child((m.line + 1).to_string()),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .overflow_hidden()
                            .whitespace_nowrap()
                            .text_ellipsis()
                            .text_color(cx.token(Token::SidebarForeground))
                            .child(text),
                    )
                    .into_any_element()
            }
        }
    }

    fn summary(&self, cx: &App) -> SharedString {
        if let Some(e) = &self.error {
            return e.clone().into();
        }
        if self.searching {
            return cx.t("core.search.searching");
        }
        if self.root.is_none() {
            return cx.t("core.search.no-folder");
        }
        if self.query.read(cx).value().is_empty() {
            return SharedString::default();
        }
        let r = &self.result;
        if r.total == 0 {
            return cx.t("core.search.no-results");
        }
        let key = if r.truncated {
            "core.search.summary-truncated"
        } else {
            "core.search.summary"
        };
        cx.t_args(key, &[("count", r.total.into()), ("files", r.files.len().into())])
    }
}

impl Focusable for SearchPanel {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}

impl BasePanel for SearchPanel {
    fn panel_name(&self) -> &'static str {
        "nucleus.search"
    }

    fn closable(&self, _: &App) -> bool {
        false
    }
}

impl Panel for SearchPanel {
    fn title(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        h_flex()
            .gap_2()
            .child(
                Icon::new(IconName::Search)
                    .small()
                    .text_color(cx.token(Token::ListHighlightForeground)),
            )
            .child(cx.t("core.panel.search"))
    }

    fn inner_padding(&self, _: &App) -> bool {
        false
    }
}

impl Render for SearchPanel {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let toggle = |id: &'static str, label: &'static str, tip: &str, on: bool, which: u8, cx: &mut Context<Self>| {
            Button::new(id)
                .xsmall()
                .label(label)
                .tooltip(cx.t(tip))
                .map(|b| if on { b.primary() } else { b.outline() })
                .on_click(cx.listener(move |this, _, _, cx| this.toggle(which, cx)))
        };
        let has_replace = !self.replace.read(cx).value().is_empty() && self.result.total > 0;
        let rows = self.rows.clone();
        v_flex()
            .track_focus(&self.focus)
            .size_full()
            .bg(cx.token(Token::SidebarBackground))
            .child(
                v_flex()
                    .p_2()
                    .gap_1p5()
                    .child(
                        Input::new(&self.query).small().prefix(
                            Icon::new(IconName::Search)
                                .small()
                                .text_color(cx.token(Token::MutedForeground)),
                        ),
                    )
                    .child(
                        h_flex()
                            .gap_1()
                            .child(div().flex_1().child(Input::new(&self.replace).small()))
                            .child(
                                Button::new("replace-all")
                                    .xsmall()
                                    .ghost()
                                    .icon(IconName::Replace)
                                    .tooltip(cx.t("core.search.replace-all"))
                                    .disabled(!has_replace)
                                    .on_click(cx.listener(|this, _, _, cx| this.replace_all(cx))),
                            ),
                    )
                    .child(
                        h_flex()
                            .gap_1()
                            .child(toggle("regex", ".*", "core.search.regex", self.regex, 0, cx))
                            .child(toggle(
                                "case",
                                "Aa",
                                "core.find.case-sensitive",
                                self.case_sensitive,
                                1,
                                cx,
                            ))
                            .child(toggle("word", "ab", "core.search.whole-word", self.whole_word, 2, cx))
                            .child(div().flex_1().child(Input::new(&self.include).xsmall())),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(if self.error.is_some() {
                                cx.token(Token::HighlightError)
                            } else {
                                cx.token(Token::MutedForeground)
                            })
                            .child(self.summary(cx)),
                    ),
            )
            .child(
                uniform_list(
                    "search-results",
                    rows.len(),
                    cx.processor(move |this, range: std::ops::Range<usize>, _, cx| {
                        range.map(|ix| this.render_row(ix, rows[ix], cx)).collect::<Vec<_>>()
                    }),
                )
                .track_scroll(&self.scroll)
                .flex_1()
                .min_h_0(),
            )
    }
}
