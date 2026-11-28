//! 入力欄の見た目をそろえる。gpui-kit の既定は、フォーカスすると枠の外に太い輪（shadcn / Bootstrap 風）を描く。
//! Nucleus ではそれを消し、VSCode と同じく 1px の枠の色だけを変える。

use gpui_kit::{
    App, Entity, Focusable as _, IntoElement, ParentElement, Styled, Window,
    component::{
        FocusableExt as _,
        input::{Input, InputState, Textarea, TextareaState},
    },
    div,
    prelude::FluentBuilder as _,
};
use theme::{Token, TokenColors as _};

/// 1行の入力欄。
pub fn input(state: &Entity<InputState>, window: &Window, cx: &App) -> Input {
    let focused = state.read(cx).focus_handle(cx).contains_focused(window, cx);
    Input::new(state)
        .focus_ring(false)
        .when(focused, |i| i.border_color(cx.token(Token::Ring)))
}

/// 複数行の入力欄（Textarea は輪を消せないので、自分で枠を描く）。
pub fn textarea(state: &Entity<TextareaState>, window: &Window, cx: &App) -> impl IntoElement {
    let focused = state.read(cx).focus_handle(cx).contains_focused(window, cx);
    div()
        .w_full()
        .rounded(gpui_kit::px(6.))
        .border_1()
        .border_color(if focused {
            cx.token(Token::Ring)
        } else {
            cx.token(Token::InputBorder)
        })
        .bg(cx.token(Token::Background))
        .px_2()
        .py_1()
        .child(Textarea::new(state).appearance(false))
}
