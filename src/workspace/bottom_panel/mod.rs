/// ボトムパネルおよび PTY ターミナル・PROBLEMS・OUTPUT UI コンポーネント

use gpui::*;
use gpui_component::theme::ActiveTheme;
use crate::terminal::TerminalSession;
use std::path::{Path, PathBuf};

/// 診断情報（問題）の表示用アイテム
#[derive(Clone, Debug)]
pub struct ProblemItem {
    pub file_path: String,
    pub line: usize,
    pub column: usize,
    pub message: String,
    pub is_error: bool,
}

pub struct BottomPanel {
    pub current_tab: &'static str,
    pub sessions: Vec<TerminalSession>,
    pub active_session: usize,
    pub logs: Vec<String>,
    pub problems: Vec<ProblemItem>,
    pub input_buffer: String,
    pub focus_handle: FocusHandle,
    pub scroll_offset: usize,
}

impl BottomPanel {
    pub fn new(cx: &mut Context<Self>) -> Self {
        let mut panel = Self {
            current_tab: "TERMINAL",
            sessions: Vec::new(),
            active_session: 0,
            logs: Vec::new(),
            problems: Vec::new(),
            input_buffer: String::new(),
            focus_handle: cx.focus_handle(),
            scroll_offset: 0,
        };

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

    /// 診断情報（エラー・警告）の更新
    pub fn set_problems(&mut self, problems: Vec<ProblemItem>, cx: &mut Context<Self>) {
        self.problems = problems;
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

        let problems_tab_label = if self.problems.is_empty() {
            "PROBLEMS".to_string()
        } else {
            format!("PROBLEMS ({})", self.problems.len())
        };

        // メインタブバー
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
                    .child(problems_tab_label)
            );

        // ターミナル専用サブバー
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
                let lines_guard = session.output_lines.read().unwrap();
                let total_lines = lines_guard.len();
                let capacity = 35;
                let max_offset = total_lines.saturating_sub(capacity);
                let effective_offset = self.scroll_offset.min(max_offset);
                let end = total_lines.saturating_sub(effective_offset);
                let start = end.saturating_sub(capacity);
                let visible_lines: Vec<String> = if total_lines > 0 {
                    lines_guard[start..end].iter().cloned().collect()
                } else {
                    Vec::new()
                };
                drop(lines_guard);

                let mut term_output = div()
                    .flex_1()
                    .p_3()
                    .font_family("Consolas")
                    .text_xs()
                    .flex()
                    .flex_col()
                    .gap_0p5()
                    .overflow_hidden()
                    .bg(cx.theme().background)
                    .track_focus(&self.focus_handle)
                    .cursor(CursorStyle::IBeam)
                    .on_mouse_down(MouseButton::Left, cx.listener(|this, _, window, cx| {
                        this.focus_handle.focus(window, cx);
                    }))
                    .on_key_down(cx.listener(|this, event: &KeyDownEvent, _, cx| {
                        let key = &event.keystroke.key;
                        if event.keystroke.modifiers.control && key == "c" {
                            this.send_input("\x03", cx);
                            this.input_buffer.clear();
                            cx.notify();
                        } else if event.keystroke.modifiers.control && key == "l" {
                            if let Some(session) = this.sessions.get(this.active_session) {
                                if let Ok(mut lines) = session.output_lines.write() {
                                    lines.clear();
                                }
                            }
                            cx.notify();
                        } else if key == "enter" {
                            let cmd = format!("{}\r\n", this.input_buffer);
                            this.send_input(&cmd, cx);
                            this.input_buffer.clear();
                            this.scroll_offset = 0; // 最新行へスクロール
                            cx.notify();
                        } else if key == "backspace" {
                            this.input_buffer.pop();
                            cx.notify();
                        } else if !event.keystroke.modifiers.control && !event.keystroke.modifiers.alt {
                            if key.len() == 1 {
                                this.input_buffer.push_str(key);
                                cx.notify();
                            } else if key == "space" {
                                this.input_buffer.push(' ');
                                cx.notify();
                            }
                        }
                    }))
                    .on_scroll_wheel(cx.listener(|this, event: &ScrollWheelEvent, _, cx| {
                        let delta = match event.delta {
                            ScrollDelta::Pixels(p) => (f32::from(p.y) / 20.0).round() as i32,
                            ScrollDelta::Lines(l) => l.y.round() as i32,
                        };
                        if delta > 0 {
                            this.scroll_offset = this.scroll_offset.saturating_add(delta as usize);
                        } else if delta < 0 {
                            this.scroll_offset = this.scroll_offset.saturating_sub((-delta) as usize);
                        }
                        cx.notify();
                    }));

                for line in visible_lines {
                    if !line.is_empty() {
                        term_output = term_output.child(
                            div()
                                .w_full()
                                .min_w_0()
                                .overflow_hidden()
                                .whitespace_nowrap()
                                .text_ellipsis()
                                .text_color(cx.theme().foreground)
                                .child(line.clone())
                        );
                    }
                }

                let input_bar = div()
                    .px_3()
                    .py_1p5()
                    .border_t_1()
                    .border_color(cx.theme().border)
                    .bg(cx.theme().muted.opacity(0.2))
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .flex_1()
                            .child(div().text_xs().font_weight(FontWeight::BOLD).text_color(gpui::rgb(0x007acc)).child("PS >"))
                            .child(
                                div()
                                    .flex_1()
                                    .text_xs()
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(cx.theme().foreground)
                                    .child(if self.input_buffer.is_empty() {
                                        "Type command here and press Enter (or type directly in terminal above)...".to_string()
                                    } else {
                                        format!("{}_", self.input_buffer)
                                    })
                            )
                    )
                    .child(
                        div().flex().gap_1p5()
                            .child(
                                div().px_2().py_0p5().bg(cx.theme().muted).rounded_sm().text_xs().cursor_pointer()
                                    .hover(|s| s.bg(cx.theme().secondary))
                                    .on_mouse_down(MouseButton::Left, cx.listener(|this, _, _, cx| {
                                        this.send_input("cargo check\r\n", cx);
                                    }))
                                    .child("cargo check")
                            )
                            .child(
                                div().px_2().py_0p5().bg(cx.theme().muted).rounded_sm().text_xs().cursor_pointer()
                                    .hover(|s| s.bg(cx.theme().secondary))
                                    .on_mouse_down(MouseButton::Left, cx.listener(|this, _, _, cx| {
                                        this.send_input("cargo test\r\n", cx);
                                    }))
                                    .child("cargo test")
                            )
                            .child(
                                div().px_2().py_0p5().bg(cx.theme().muted).rounded_sm().text_xs().cursor_pointer()
                                    .hover(|s| s.bg(cx.theme().secondary))
                                    .on_mouse_down(MouseButton::Left, cx.listener(|this, _, _, cx| {
                                        this.send_input("git status\r\n", cx);
                                    }))
                                    .child("git status")
                            )
                    );

                div()
                    .size_full()
                    .flex()
                    .flex_col()
                    .track_focus(&self.focus_handle)
                    .cursor(CursorStyle::IBeam)
                    .on_mouse_down(MouseButton::Left, cx.listener(|this, _, window, cx| {
                        this.focus_handle.focus(window, cx);
                    }))
                    .on_key_down(cx.listener(|this, event: &KeyDownEvent, _, cx| {
                        let key = &event.keystroke.key;
                        if key == "enter" {
                            let cmd = format!("{}\r\n", this.input_buffer);
                            this.send_input(&cmd, cx);
                            this.input_buffer.clear();
                            cx.notify();
                        } else if key == "backspace" {
                            this.input_buffer.pop();
                            cx.notify();
                        } else if event.keystroke.modifiers.control && key == "c" {
                            this.send_input("\x03", cx);
                            this.input_buffer.clear();
                            cx.notify();
                        } else if !event.keystroke.modifiers.control && !event.keystroke.modifiers.alt {
                            if key.len() == 1 {
                                this.input_buffer.push_str(key);
                                cx.notify();
                            } else if key == "space" {
                                this.input_buffer.push(' ');
                                cx.notify();
                            }
                        }
                    }))
                    .child(term_output)
                    .child(input_bar)
                    .into_any_element()
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
            // PROBLEMS タブ
            if self.problems.is_empty() {
                div().p_4().text_xs().text_color(cx.theme().muted_foreground).child("No problems detected in the workspace.").into_any_element()
            } else {
                let mut prob_list = div().flex().flex_col().p_2().overflow_hidden().size_full();
                for prob in &self.problems {
                    let file_path = prob.file_path.clone();
                    let line = prob.line;
                    let col = prob.column;
                    let msg = prob.message.clone();
                    let is_err = prob.is_error;
                    let file_name = Path::new(&file_path).file_name().unwrap_or_default().to_string_lossy().to_string();

                    let fn_title_for_tab = file_name.clone();
                    let row = div()
                        .px_2()
                        .py_1()
                        .rounded_sm()
                        .flex()
                        .items_center()
                        .justify_between()
                        .hover(|s| s.bg(cx.theme().secondary))
                        .cursor_pointer()
                        .on_mouse_down(MouseButton::Left, cx.listener(move |_this, _, _, cx| {
                            if let Ok(content) = std::fs::read_to_string(&file_path) {
                                if cx.has_global::<crate::plugin_manager::PluginManagerGlobal>() {
                                    let pm_global = cx.global::<crate::plugin_manager::PluginManagerGlobal>().0.clone();
                                    let fp = file_path.clone();
                                    let fn_title = fn_title_for_tab.clone();
                                    pm_global.update(cx, |pm, _| {
                                        pm.dispatch_action(crate::plugin_manager::action::PluginAction::OpenTab {
                                            path: fp,
                                            title: fn_title,
                                            content,
                                        });
                                    });
                                }
                            }
                        }))
                        .child(
                            div().flex().items_center().gap_2()
                                .child(
                                    div().text_xs().font_weight(FontWeight::BOLD)
                                        .text_color(if is_err { gpui::rgb(0xef4444) } else { gpui::rgb(0xeab308) })
                                        .child(if is_err { "⨂" } else { "⚠" })
                                )
                                .child(div().text_xs().text_color(cx.theme().foreground).child(msg))
                        )
                        .child(
                            div().text_xs().text_color(cx.theme().muted_foreground)
                                .child(format!("{} [{}:{}]", file_name, line, col))
                        );

                    prob_list = prob_list.child(row);
                }
                prob_list.into_any_element()
            }
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
