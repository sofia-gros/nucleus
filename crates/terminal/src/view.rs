//! ターミナルパネル（下ドック）。

use std::path::PathBuf;

use gpui_kit::{
    App, AppContext as _, Context, Entity, EventEmitter, FocusHandle, Focusable, InteractiveElement as _, IntoElement,
    KeyBinding, KeyDownEvent, MouseButton, NoAction, ParentElement, Render, ScrollEvent, SharedString, Styled,
    Subscription, Window, actions,
    assets::IconName,
    component::{
        Icon, Sizable as _,
        dock::{BasePanel, Panel, PanelEvent},
        h_flex,
    },
    div,
    prelude::FluentBuilder as _,
    px,
};
use i18n::Translate as _;
use theme::{Token, TokenColors as _};

use crate::{
    element::TerminalElement,
    keys::{self, Mods},
    model::{Terminal, TerminalEvent},
};

actions!(terminal, [Paste, Restart]);

const CONTEXT: &str = "Terminal";

pub fn init(cx: &mut App) {
    // シェルで使うキーは、ワークスペースのショートカットに取られないようにする。
    let passthrough = [
        "ctrl-b", "ctrl-p", "ctrl-w", "ctrl-o", "ctrl-s", "ctrl-f", "ctrl-h", "ctrl-,", "f1", "f3",
    ];
    let mut bindings: Vec<KeyBinding> = passthrough
        .iter()
        .map(|k| KeyBinding::new(k, NoAction, Some(CONTEXT)))
        .collect();
    bindings.push(KeyBinding::new("ctrl-shift-v", Paste, Some(CONTEXT)));
    cx.bind_keys(bindings);
}

/// ターミナルパネル。`Workspace` が下ドックに置く。
pub struct TerminalView {
    terminal: Option<Entity<Terminal>>,
    error: Option<String>,
    cwd: Option<PathBuf>,
    focus: FocusHandle,
    scroll_remainder: f32,
    _subs: Vec<Subscription>,
}

impl TerminalView {
    pub fn new(cwd: Option<PathBuf>, cx: &mut Context<Self>) -> Self {
        TerminalView {
            terminal: None,
            error: None,
            cwd,
            focus: cx.focus_handle(),
            scroll_remainder: 0.,
            _subs: Vec::new(),
        }
    }

    pub fn set_cwd(&mut self, cwd: Option<PathBuf>) {
        if self.terminal.is_none() {
            self.cwd = cwd;
        }
    }

    /// 最初に表示されたときにシェルを起動する（アプリの起動時間に影響させない）。
    fn ensure_started(&mut self, cx: &mut Context<Self>) {
        if self.terminal.is_some() || self.error.is_some() {
            return;
        }
        let cwd = self.cwd.clone();
        let terminal = cx.new(|cx| Terminal::open(cwd, cx));
        self._subs.push(cx.observe(&terminal, |this, t, cx| {
            // 起動できなかったら、その理由を出す。
            if let Some(e) = t.read(cx).error() {
                log::error!("ターミナルを起動できなかった: {e}");
                this.error = Some(e.to_string());
                this.terminal = None;
            }
            cx.notify()
        }));
        self._subs
            .push(cx.subscribe(&terminal, |_, _, e: &TerminalEvent, cx| match e {
                TerminalEvent::TitleChanged | TerminalEvent::Exited => cx.notify(),
                TerminalEvent::Bell => {}
            }));
        self.terminal = Some(terminal);
    }

    fn restart(&mut self, cx: &mut Context<Self>) {
        self.terminal = None;
        self.error = None;
        self._subs.clear();
        self.ensure_started(cx);
        cx.notify();
    }

    fn on_key_down(&mut self, e: &KeyDownEvent, _: &mut Window, cx: &mut Context<Self>) {
        let Some(t) = &self.terminal else { return };
        let ks = &e.keystroke;
        if ks.modifiers.platform {
            return;
        }
        if t.read(cx).has_exited() {
            if ks.key == "enter" {
                self.restart(cx);
                cx.stop_propagation();
            }
            return;
        }
        let mods = Mods {
            ctrl: ks.modifiers.control,
            alt: ks.modifiers.alt,
            shift: ks.modifiers.shift,
        };
        let app_cursor = t.read(cx).mode() & remote::term_mode::APP_CURSOR != 0;
        if let Some(bytes) = keys::to_bytes(&ks.key, ks.key_char.as_deref(), mods, app_cursor) {
            t.read(cx).input(bytes);
            cx.stop_propagation();
        }
    }

    fn on_scroll(&mut self, e: &ScrollEvent, window: &mut Window, cx: &mut Context<Self>) {
        let Some(t) = &self.terminal else { return };
        let line_height = window.line_height();
        self.scroll_remainder += f32::from(e.delta.pixel_delta(line_height).y) / f32::from(line_height);
        let lines = self.scroll_remainder.trunc();
        if lines != 0. {
            self.scroll_remainder -= lines;
            t.read(cx).scroll(lines as i32);
            cx.notify();
        }
    }

    fn paste(&mut self, cx: &mut Context<Self>) {
        let Some(t) = &self.terminal else { return };
        let Some(text) = cx.read_from_clipboard().and_then(|c| c.text()) else {
            return;
        };
        let bracketed = t.read(cx).mode() & remote::term_mode::BRACKETED_PASTE != 0;
        t.read(cx).input(keys::paste(&text, bracketed));
    }
}

impl EventEmitter<PanelEvent> for TerminalView {}

impl Focusable for TerminalView {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}

impl BasePanel for TerminalView {
    fn panel_name(&self) -> &'static str {
        "nucleus.terminal"
    }
}

impl Panel for TerminalView {
    fn title(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let title: SharedString = self
            .terminal
            .as_ref()
            .and_then(|t| t.read(cx).title().map(|s| SharedString::from(s.to_string())))
            .unwrap_or_else(|| cx.t("core.panel.terminal"));
        h_flex()
            .gap_2()
            .max_w(px(220.))
            .child(
                Icon::new(IconName::Terminal)
                    .small()
                    .text_color(cx.token(Token::ListHighlightForeground)),
            )
            .child(div().overflow_hidden().whitespace_nowrap().text_ellipsis().child(title))
    }

    fn inner_padding(&self, _: &App) -> bool {
        false
    }
}

impl Render for TerminalView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.ensure_started(cx);
        let focused = self.focus.is_focused(window);
        let exited = self.terminal.as_ref().is_some_and(|t| t.read(cx).has_exited());
        div()
            .id("terminal")
            .key_context(CONTEXT)
            .track_focus(&self.focus)
            .on_key_down(cx.listener(Self::on_key_down))
            .on_scroll(cx.listener(Self::on_scroll))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _, window, cx| {
                    this.focus.focus(window, cx);
                }),
            )
            .on_action(cx.listener(|this, _: &Paste, _, cx| this.paste(cx)))
            .on_action(cx.listener(|this, _: &Restart, _, cx| this.restart(cx)))
            .relative()
            .size_full()
            .bg(cx.token(Token::TerminalBackground))
            .when_some(self.terminal.clone(), |d, t| d.child(TerminalElement::new(t, focused)))
            .when_some(self.error.clone(), |d, e| {
                d.child(
                    div()
                        .p_3()
                        .text_sm()
                        .text_color(cx.token(Token::DangerBackground))
                        .child(cx.t_args("core.terminal.spawn-failed", &[("error", e.into())])),
                )
            })
            .when(exited, |d| {
                d.child(
                    div()
                        .absolute()
                        .bottom_2()
                        .left_3()
                        .px_2()
                        .py_1()
                        .rounded(px(4.))
                        .bg(cx.token(Token::MutedBackground))
                        .text_xs()
                        .text_color(cx.token(Token::MutedForeground))
                        .child(cx.t("core.terminal.exited")),
                )
            })
    }
}
