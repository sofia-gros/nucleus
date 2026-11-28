//! Atom 式の検索と置換パネル（エディタの下端）。gpui-kit エディタの検索 API を使う。
//! 正規表現と単語単位の検索はエンジンが対応していないため、今は大文字小文字の区別だけ。

use gpui_kit::{
    App, AppContext as _, Context, Entity, EventEmitter, FocusHandle, Focusable, InteractiveElement as _, IntoElement,
    KeyBinding, ParentElement, Render, SharedString, Styled, Subscription, Window, actions,
    assets::IconName,
    component::{
        Icon, Sizable as _,
        button::{Button, ButtonVariants as _},
        h_flex,
        input::{EditorState, InputEvent, InputState},
        v_flex,
    },
    div,
    prelude::FluentBuilder as _,
    px,
};
use i18n::Translate as _;
use theme::{Token, TokenColors as _};

actions!(find_panel, [CloseFindPanel, FindNext, FindPrevious]);

const CONTEXT: &str = "FindPanel";

pub fn init(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("escape", CloseFindPanel, Some(CONTEXT)),
        KeyBinding::new("f3", FindNext, Some("EditorPanel")),
        KeyBinding::new("shift-f3", FindPrevious, Some("EditorPanel")),
    ]);
}

pub enum FindPanelEvent {
    Closed,
}

pub struct FindPanel {
    editor: Entity<EditorState>,
    find: Entity<InputState>,
    replace: Entity<InputState>,
    case_sensitive: bool,
    focus: FocusHandle,
    _subscriptions: Vec<Subscription>,
}

impl FindPanel {
    pub fn new(editor: Entity<EditorState>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let find = cx.new(|cx| InputState::new(window, cx).placeholder(cx.t("core.find.find-placeholder")));
        let replace = cx.new(|cx| InputState::new(window, cx).placeholder(cx.t("core.find.replace-placeholder")));
        let subs = vec![
            cx.subscribe_in(&find, window, |this, _, e: &InputEvent, window, cx| match e {
                InputEvent::Change => this.update_query(cx),
                InputEvent::PressEnter { shift, .. } => {
                    if *shift {
                        this.previous(cx)
                    } else {
                        this.next(cx)
                    }
                    let _ = window;
                }
                _ => {}
            }),
            cx.subscribe_in(&replace, window, |this, _, e: &InputEvent, window, cx| {
                if let InputEvent::PressEnter { .. } = e {
                    this.replace_one(window, cx);
                }
            }),
            // 編集で一致数が変わったら表示を更新する。
            cx.observe(&editor, |_, _, cx| cx.notify()),
        ];
        FindPanel {
            editor,
            find,
            replace,
            case_sensitive: false,
            focus: cx.focus_handle(),
            _subscriptions: subs,
        }
    }

    /// パネルを開いたとき: 選択中の文字列があれば検索語にし、検索欄にフォーカスする。
    pub fn activate(&mut self, replace: bool, window: &mut Window, cx: &mut Context<Self>) {
        let selected = self.editor.read(cx).selected_text().to_string();
        if !selected.is_empty() && !selected.contains('\n') {
            self.find.update(cx, |s, cx| s.set_value(selected, window, cx));
        }
        let target = if replace { &self.replace } else { &self.find };
        target.update(cx, |s, cx| {
            s.focus(window, cx);
            s.select_all(window, cx);
        });
        self.update_query(cx);
    }

    fn query(&self, cx: &App) -> String {
        self.find.read(cx).value().to_string()
    }

    fn update_query(&mut self, cx: &mut Context<Self>) {
        let q = self.query(cx);
        let ci = !self.case_sensitive;
        self.editor.update(cx, |e, cx| e.set_search_query(q, ci, cx));
        cx.notify();
    }

    fn next(&mut self, cx: &mut Context<Self>) {
        self.editor.update(cx, |e, cx| e.next_search_match(cx));
        cx.notify();
    }

    fn previous(&mut self, cx: &mut Context<Self>) {
        self.editor.update(cx, |e, cx| e.previous_search_match(cx));
        cx.notify();
    }

    fn replace_one(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let with = self.replace.read(cx).value().to_string();
        self.editor
            .update(cx, |e, cx| e.replace_current_search_match(&with, window, cx));
        cx.notify();
    }

    fn replace_all(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let with = self.replace.read(cx).value().to_string();
        self.editor
            .update(cx, |e, cx| e.replace_all_search_matches(&with, window, cx));
        cx.notify();
    }

    fn close(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.editor.update(cx, |e, cx| {
            e.close_search(cx);
            e.focus(window, cx);
        });
        cx.emit(FindPanelEvent::Closed);
    }

    pub fn find_next(&mut self, cx: &mut Context<Self>) {
        self.next(cx);
    }

    pub fn find_previous(&mut self, cx: &mut Context<Self>) {
        self.previous(cx);
    }

    fn summary(&self, cx: &App) -> SharedString {
        let q = self.query(cx);
        if q.is_empty() {
            return cx.t("core.find.hint");
        }
        let session = self.editor.read(cx).search_session();
        let count = session.matcher.len();
        if count == 0 {
            return cx.t_args("core.find.no-results", &[("query", q.into())]);
        }
        let current = session.matcher.current().map_or(0, |i| i + 1);
        cx.t_args(
            "core.find.results",
            &[
                ("current", current.into()),
                ("count", count.into()),
                ("query", q.into()),
            ],
        )
    }
}

impl EventEmitter<FindPanelEvent> for FindPanel {}

impl Focusable for FindPanel {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}

impl Render for FindPanel {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let case = self.case_sensitive;
        let toggle = Button::new("case")
            .xsmall()
            .label("Aa")
            .tooltip(cx.t("core.find.case-sensitive"))
            .map(|b| if case { b.primary() } else { b.outline() })
            .on_click(cx.listener(|this, _, _, cx| {
                this.case_sensitive = !this.case_sensitive;
                this.update_query(cx);
            }));
        let button = |id: &'static str, key: &str, cx: &App| Button::new(id).outline().label(cx.t(key));
        v_flex()
            .key_context(CONTEXT)
            .track_focus(&self.focus)
            .on_action(cx.listener(|this, _: &CloseFindPanel, window, cx| this.close(window, cx)))
            .w_full()
            .px_3()
            .py(px(10.))
            .gap_2()
            .border_t_1()
            .border_color(cx.token(Token::Border))
            .bg(cx.token(Token::TabBarBackground))
            .child(
                h_flex()
                    .gap_2()
                    .child(
                        div()
                            .text_sm()
                            .font_weight(gpui_kit::FontWeight::MEDIUM)
                            .text_color(cx.token(Token::Foreground))
                            .child(cx.t("core.find.title")),
                    )
                    .child(
                        div()
                            .flex_1()
                            .text_xs()
                            .text_color(cx.token(Token::MutedForeground))
                            .child(self.summary(cx)),
                    )
                    .child(toggle)
                    .child(
                        Button::new("close-find")
                            .ghost()
                            .xsmall()
                            .icon(Icon::new(IconName::X).text_color(cx.token(Token::MutedForeground)))
                            .on_click(cx.listener(|this, _, window, cx| this.close(window, cx))),
                    ),
            )
            .child(
                h_flex()
                    .gap_2()
                    .child(div().flex_1().child(crate::fields::input(&self.find, window, cx)))
                    .child(
                        Button::new("prev")
                            .outline()
                            .icon(IconName::ArrowUp)
                            .on_click(cx.listener(|this, _, _, cx| this.previous(cx))),
                    )
                    .child(
                        Button::new("next")
                            .outline()
                            .icon(IconName::ArrowDown)
                            .on_click(cx.listener(|this, _, _, cx| this.next(cx))),
                    )
                    .child(
                        button("find", "core.find.find", cx)
                            .w(px(104.))
                            .on_click(cx.listener(|this, _, _, cx| this.next(cx))),
                    ),
            )
            .child(
                h_flex()
                    .gap_2()
                    .child(div().flex_1().child(crate::fields::input(&self.replace, window, cx)))
                    .child(
                        button("replace", "core.find.replace", cx)
                            .on_click(cx.listener(|this, _, window, cx| this.replace_one(window, cx))),
                    )
                    .child(
                        button("replace-all", "core.find.replace-all", cx)
                            .w(px(104.))
                            .on_click(cx.listener(|this, _, window, cx| this.replace_all(window, cx))),
                    ),
            )
            .when(false, |d| d)
    }
}
