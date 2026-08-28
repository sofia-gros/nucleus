/// ボトムパネルおよび PTY ターミナル UI コンポーネント

use gpui::*;
use gpui_component::theme::ActiveTheme;
use crate::terminal::TerminalSession;
use std::path::PathBuf;

pub struct BottomPanel {
    pub current_tab: &'static str,
    pub sessions: Vec<TerminalSession>,
    pub active_session: usize,
    pub logs: Vec<String>,
    pub input_buffer: String,
    pub focus_handle: FocusHandle,
}

impl BottomPanel {
    pub fn new(cx: &mut Context<Self>) -> Self {
        let mut panel = Self {
            current_tab: "TERMINAL",
            sessions: Vec::new(),
            active_session: 0,
            logs: Vec::new(),
            input_buffer: String::new(),
            focus_handle: cx.focus_handle(),
        };

        // 初期ターミナルセッションの作成
        if let Ok(session) = TerminalSession::new("term-1".to_string(), "pwsh".to_string(), None) {
            panel.sessions.push(session);
        }

        panel
    }

    /// 新規ターミナルタブの作成
    pub fn new_terminal(&mut self, cwd: Option<PathBuf>, cx: &mut Context<Self>) {
        let id = format!("term-{}", self.sessions.len() + 1);
        let title = format!("term-{}", self.sessions.len() + 1);
        if let Ok(session) = TerminalSession::new(id, title, cwd) {
            self.sessions.push(session);
            self.active_session = self.sessions.len() - 1;
            self.current_tab = "TERMINAL";
            cx.notify();
        }
    }

    /// ターミナルタブの終了
    pub fn close_terminal(&mut self, index: usize, cx: &mut Context<Self>) {
        if index < self.sessions.len() {
            self.sessions.remove(index);
            if self.active_session >= self.sessions.len() && !self.sessions.is_empty() {
                self.active_session = self.sessions.len() - 1;
            }
            cx.notify();
        }
    }

    pub fn write_log(&mut self, text: String, cx: &mut Context<Self>) {
        self.logs.push(text);
        cx.notify();
    }

    /// 現在のセッションへの入力送信
    pub fn send_input(&mut self, input: &str, cx: &mut Context<Self>) {
        if let Some(session) = self.sessions.get(self.active_session) {
            let _ = session.write_input(input);
        }
        cx.notify();
    }
}

impl Render for BottomPanel {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let is_terminal = self.current_tab == "TERMINAL";
        let is_output = self.current_tab == "OUTPUT";
        let is_problems = self.current_tab == "PROBLEMS";

        // メインタブバー (TERMINAL, OUTPUT, PROBLEMS)
        let main_tabs = div()
            .flex()
            .items_center()
            .child(
                div()
                    .px_3()
                    .py_1()
                    .border_r_1()
                    .border_color(cx.theme().border)
                    .text_xs()
                    .font_weight(if is_terminal { FontWeight::BOLD } else { FontWeight::NORMAL })
                    .text_color(if is_terminal { cx.theme().foreground } else { cx.theme().muted_foreground })
                    .bg(if is_terminal { cx.theme().background } else { cx.theme().muted.opacity(0.3) })
                    .hover(|s| s.bg(cx.theme().secondary))
                    .cursor_pointer()
                    .on_mouse_down(MouseButton::Left, cx.listener(|this, _, _, cx| {
                        this.current_tab = "TERMINAL";
                        cx.notify();
                    }))
                    .child("TERMINAL")
            )
            .child(
                div()
                    .px_3()
                    .py_1()
                    .border_r_1()
                    .border_color(cx.theme().border)
                    .text_xs()
                    .font_weight(if is_output { FontWeight::BOLD } else { FontWeight::NORMAL })
                    .text_color(if is_output { cx.theme().foreground } else { cx.theme().muted_foreground })
                    .bg(if is_output { cx.theme().background } else { cx.theme().muted.opacity(0.3) })
                    .hover(|s| s.bg(cx.theme().secondary))
                    .cursor_pointer()
                    .on_mouse_down(MouseButton::Left, cx.listener(|this, _, _, cx| {
                        this.current_tab = "OUTPUT";
                        cx.notify();
                    }))
                    .child("OUTPUT")
            )
            .child(
                div()
                    .px_3()
                    .py_1()
                    .border_r_1()
                    .border_color(cx.theme().border)
                    .text_xs()
                    .font_weight(if is_problems { FontWeight::BOLD } else { FontWeight::NORMAL })
                    .text_color(if is_problems { cx.theme().foreground } else { cx.theme().muted_foreground })
                    .bg(if is_problems { cx.theme().background } else { cx.theme().muted.opacity(0.3) })
                    .hover(|s| s.bg(cx.theme().secondary))
                    .cursor_pointer()
                    .on_mouse_down(MouseButton::Left, cx.listener(|this, _, _, cx| {
                        this.current_tab = "PROBLEMS";
                        cx.notify();
                    }))
                    .child("PROBLEMS")
            );

        // ターミナル専用サブバー（複数ターミナルタブ & ＋ボタン）
        let mut term_tabs = div().flex().items_center().gap_1();
        if is_terminal {
            for (idx, sess) in self.sessions.iter().enumerate() {
                let is_active_sess = idx == self.active_session;
                let title = sess.title.clone();

                term_tabs = term_tabs.child(
                    div()
                        .px_2()
                        .py_0p5()
                        .rounded_sm()
                        .text_xs()
                        .bg(if is_active_sess { cx.theme().muted } else { cx.theme().background })
                        .text_color(if is_active_sess { cx.theme().foreground } else { cx.theme().muted_foreground })
                        .hover(|s| s.bg(cx.theme().secondary))
                        .cursor_pointer()
                        .flex()
                        .items_center()
                        .gap_1()
                        .on_mouse_down(MouseButton::Left, cx.listener(move |this, _, _, cx| {
                            this.active_session = idx;
                            cx.notify();
                        }))
                        .child(title)
                        .child(
                            div()
                                .px_1()
                                .hover(|s| s.bg(gpui::rgb(0xe81123)).text_color(gpui::rgb(0xffffff)))
                                .on_mouse_down(MouseButton::Left, cx.listener(move |this, _, _, cx| {
                                    cx.stop_propagation();
                                    this.close_terminal(idx, cx);
                                }))
                                .child("✕")
                        )
                );
            }

            // 新規ターミナル作成ボタン (+)
            term_tabs = term_tabs.child(
                div()
                    .px_2()
                    .py_0p5()
                    .rounded_sm()
                    .text_xs()
                    .text_color(cx.theme().muted_foreground)
                    .hover(|s| s.bg(cx.theme().secondary).text_color(cx.theme().foreground))
                    .cursor_pointer()
                    .on_mouse_down(MouseButton::Left, cx.listener(|this, _, _, cx| {
                        this.new_terminal(None, cx);
                    }))
                    .child("+")
            );
        }

        // メインコンテンツ
        let content = if is_terminal {
            if let Some(session) = self.sessions.get(self.active_session) {
                let lines = session.output_lines.read().unwrap().clone();
                let mut term_output = div()
                    .flex_1()
                    .p_3()
                    .font_family("Consolas")
                    .text_xs()
                    .flex()
                    .flex_col()
                    .overflow_hidden()
                    .bg(cx.theme().background);

                for line in lines.iter().rev().take(30).collect::<Vec<_>>().into_iter().rev() {
                    term_output = term_output.child(
                        div().text_color(cx.theme().foreground).child(line.clone())
                    );
                }

                // 入力用プロンプトバー
                let input_bar = div()
                    .px_3()
                    .py_1p5()
                    .border_t_1()
                    .border_color(cx.theme().border)
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(div().text_xs().font_weight(FontWeight::BOLD).text_color(gpui::rgb(0x007acc)).child(">"))
                    .child(
                        div()
                            .flex_1()
                            .text_xs()
                            .text_color(cx.theme().foreground)
                            .child(if self.input_buffer.is_empty() {
                                "Type command here and press Enter (or click Quick Commands below)...".to_string()
                            } else {
                                self.input_buffer.clone()
                            })
                    )
                    .child(
                        // クイックコマンドボタン群
                        div().flex().gap_1()
                            .child(
                                div().px_1p5().py_0p5().bg(cx.theme().muted).rounded_sm().text_xs().cursor_pointer()
                                    .hover(|s| s.bg(cx.theme().secondary))
                                    .on_mouse_down(MouseButton::Left, cx.listener(|this, _, _, cx| {
                                        this.send_input("cargo check\r\n", cx);
                                    }))
                                    .child("cargo check")
                            )
                            .child(
                                div().px_1p5().py_0p5().bg(cx.theme().muted).rounded_sm().text_xs().cursor_pointer()
                                    .hover(|s| s.bg(cx.theme().secondary))
                                    .on_mouse_down(MouseButton::Left, cx.listener(|this, _, _, cx| {
                                        this.send_input("cargo test\r\n", cx);
                                    }))
                                    .child("cargo test")
                            )
                            .child(
                                div().px_1p5().py_0p5().bg(cx.theme().muted).rounded_sm().text_xs().cursor_pointer()
                                    .hover(|s| s.bg(cx.theme().secondary))
                                    .on_mouse_down(MouseButton::Left, cx.listener(|this, _, _, cx| {
                                        this.send_input("git status\r\n", cx);
                                    }))
                                    .child("git status")
                            )
                    );

                div().size_full().flex().flex_col().child(term_output).child(input_bar).into_any_element()
            } else {
                div().p_4().text_xs().text_color(cx.theme().muted_foreground).child("No active terminal session. Click '+' to open one.").into_any_element()
            }
        } else if is_output {
            let mut log_list = div().flex().flex_col().p_3().overflow_hidden().size_full().font_family("Consolas").text_xs();
            for log in &self.logs {
                log_list = log_list.child(div().text_color(cx.theme().foreground).child(log.clone()));
            }
            log_list.into_any_element()
        } else {
            div().p_4().text_xs().text_color(cx.theme().muted_foreground).child("No problems detected in the workspace.").into_any_element()
        };

        div()
            .w_full()
            .h(px(220.0))
            .bg(cx.theme().background)
            .border_t_1()
            .border_color(cx.theme().border)
            .flex()
            .flex_col()
            .track_focus(&self.focus_handle)
            .child(
                div()
                    .w_full()
                    .h(gpui::px(28.0))
                    .px_2()
                    .flex()
                    .items_center()
                    .justify_between()
                    .border_b_1()
                    .border_color(cx.theme().border)
                    .child(main_tabs)
                    .child(term_tabs)
            )
            .child(div().w_full().flex_1().child(content))
    }
}
