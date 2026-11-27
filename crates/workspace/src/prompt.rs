//! 確認ダイアログ。gpui の prompt を差し替え、テーマの Token だけで描く（OS によらず同じ見た目）。

use gpui_kit::{
    App, AppContext as _, Context, EventEmitter, FocusHandle, Focusable, InteractiveElement as _, IntoElement,
    KeyBinding, ParentElement, PromptButton, PromptHandle, PromptLevel, PromptResponse, Render, RenderablePromptHandle,
    SharedString, Styled, Window, actions,
    assets::IconName,
    component::{
        Icon,
        button::{Button, ButtonVariants as _},
        h_flex, v_flex,
    },
    div,
    prelude::FluentBuilder as _,
    px,
};
use theme::{Token, TokenColors as _};

actions!(prompt, [ConfirmPrompt, CancelPrompt]);

const CONTEXT: &str = "NucleusPrompt";

pub fn init(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("enter", ConfirmPrompt, Some(CONTEXT)),
        KeyBinding::new("escape", CancelPrompt, Some(CONTEXT)),
    ]);
    cx.set_prompt_builder(build);
}

fn build(
    level: PromptLevel,
    message: &str,
    detail: Option<&str>,
    buttons: &[PromptButton],
    handle: PromptHandle,
    window: &mut Window,
    cx: &mut App,
) -> RenderablePromptHandle {
    let view = cx.new(|cx| NucleusPrompt {
        level,
        message: message.to_string().into(),
        detail: detail.map(|d| d.to_string().into()),
        buttons: buttons.iter().map(|b| (b.label().clone(), b.is_cancel())).collect(),
        focus: cx.focus_handle(),
    });
    handle.with_view(view, window, cx)
}

struct NucleusPrompt {
    level: PromptLevel,
    message: SharedString,
    detail: Option<SharedString>,
    buttons: Vec<(SharedString, bool)>,
    focus: FocusHandle,
}

impl NucleusPrompt {
    /// Escape で選ぶボタン: Cancel と明示されたもの、なければ最後のもの。
    fn cancel_index(&self) -> usize {
        self.buttons
            .iter()
            .position(|(_, c)| *c)
            .unwrap_or(self.buttons.len().saturating_sub(1))
    }
}

impl EventEmitter<PromptResponse> for NucleusPrompt {}

impl Focusable for NucleusPrompt {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}

impl Render for NucleusPrompt {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let (icon, color) = match self.level {
            PromptLevel::Info => (IconName::Info, Token::InfoBackground),
            PromptLevel::Warning => (IconName::TriangleAlert, Token::WarningBackground),
            PromptLevel::Critical => (IconName::CircleX, Token::DangerBackground),
        };
        let cancel = self.cancel_index();
        let buttons = self.buttons.iter().enumerate().map(|(i, (label, _))| {
            let b = Button::new(("prompt-button", i)).label(label.clone());
            let b = if i == 0 {
                b.primary()
            } else if i == cancel {
                b.ghost()
            } else {
                b.outline()
            };
            b.on_click(cx.listener(move |_, _, _, cx| cx.emit(PromptResponse(i))))
        });
        div()
            .id("nucleus-prompt")
            .key_context(CONTEXT)
            .track_focus(&self.focus)
            .on_action(cx.listener(|_, _: &ConfirmPrompt, _, cx| cx.emit(PromptResponse(0))))
            .on_action(cx.listener(move |_, _: &CancelPrompt, _, cx| cx.emit(PromptResponse(cancel))))
            .occlude()
            .size_full()
            .flex()
            .items_center()
            .justify_center()
            .bg(cx.token(Token::Overlay))
            .font_family(cx.global::<theme::ActiveTheme>().resolved.fonts.ui.clone())
            .child(
                v_flex()
                    .w(px(440.))
                    .p_5()
                    .gap_4()
                    .rounded(px(8.))
                    .border_1()
                    .border_color(cx.token(Token::Border))
                    .bg(cx.token(Token::PopoverBackground))
                    .shadow_lg()
                    .child(
                        h_flex()
                            .items_start()
                            .gap_3()
                            .child(Icon::new(icon).size(px(20.)).text_color(cx.token(color)))
                            .child(
                                v_flex()
                                    .flex_1()
                                    .min_w_0()
                                    .gap_1()
                                    .child(
                                        div()
                                            .text_base()
                                            .font_weight(gpui_kit::FontWeight::MEDIUM)
                                            .text_color(cx.token(Token::AccentForeground))
                                            .child(self.message.clone()),
                                    )
                                    .when_some(self.detail.clone(), |d, detail| {
                                        d.child(
                                            div()
                                                .text_sm()
                                                .text_color(cx.token(Token::MutedForeground))
                                                .child(detail),
                                        )
                                    }),
                            ),
                    )
                    .child(h_flex().justify_end().gap_2().children(buttons)),
            )
    }
}
