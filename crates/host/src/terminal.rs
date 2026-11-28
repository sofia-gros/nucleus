//! ターミナル。alacritty_terminal で VT を解釈し、画面の変わった行を画面側へ送る。
//!
//! PTY の読み書きは alacritty の EventLoop（専用スレッド）が行う。更新の知らせは 1 本のスレッドで受け、
//! 短い間（`FRAME`）にまとめてから、前回送った画面と比べて変わった行だけを送る。

use std::{
    borrow::Cow,
    collections::HashMap,
    sync::{
        Arc, Mutex,
        mpsc::{Receiver, RecvTimeoutError, Sender, channel},
    },
    time::{Duration, Instant},
};

use alacritty_terminal::{
    event::{Event as TermEvent, EventListener, WindowSize},
    event_loop::{EventLoop, EventLoopSender, Msg},
    grid::{Dimensions, Scroll},
    sync::FairMutex,
    term::{Config, Term, TermMode, cell::Flags},
    tty,
    vte::ansi::{Color as AnsiColor, CursorShape as AnsiCursor},
};
use remote::{
    CursorShape, Event, TermCell, TermColor, TermCursor, TermSize, TermUpdate, TerminalSpec, cell_flags, term_mode,
};

/// 画面を送る間隔の下限（だいたい 60 回/秒）。
const FRAME: Duration = Duration::from_millis(16);

#[derive(Clone)]
struct Listener(Sender<TermEvent>);

impl EventListener for Listener {
    fn send_event(&self, event: TermEvent) {
        let _ = self.0.send(event);
    }
}

#[derive(Clone, Copy)]
struct Grid(TermSize);

impl Dimensions for Grid {
    fn total_lines(&self) -> usize {
        self.0.lines as usize
    }
    fn screen_lines(&self) -> usize {
        self.0.lines as usize
    }
    fn columns(&self) -> usize {
        self.0.cols as usize
    }
}

fn window_size(s: TermSize) -> WindowSize {
    WindowSize {
        num_lines: s.lines,
        num_cols: s.cols,
        cell_width: s.cell_width,
        cell_height: s.cell_height,
    }
}

/// 開いている 1 つのターミナル。
pub struct HostTerminal {
    term: Arc<FairMutex<Term<Listener>>>,
    sender: EventLoopSender,
    size: Arc<Mutex<TermSize>>,
    /// 画面を送り直してほしいとき（スクロールなど）に、知らせのスレッドを起こす。
    wake: Sender<TermEvent>,
}

impl HostTerminal {
    /// シェルを起動する。`emit` は画面の更新などを画面側へ届ける。
    pub fn open(id: u64, spec: TerminalSpec, emit: impl Fn(Event) + Send + 'static) -> anyhow::Result<HostTerminal> {
        let size = TermSize {
            cols: spec.size.cols.max(2),
            lines: spec.size.lines.max(1),
            cell_width: spec.size.cell_width.max(1),
            cell_height: spec.size.cell_height.max(1),
        };
        let (tx, rx) = channel();
        let listener = Listener(tx.clone());
        let config = Config {
            scrolling_history: spec.scrollback.min(1_000_000),
            ..Config::default()
        };
        let term = Arc::new(FairMutex::new(Term::new(config, &Grid(size), listener.clone())));

        let mut env = HashMap::new();
        env.insert("TERM".to_string(), "xterm-256color".to_string());
        env.insert("COLORTERM".to_string(), "truecolor".to_string());
        env.insert("TERM_PROGRAM".to_string(), "nucleus".to_string());
        let shell = spec
            .shell
            .filter(|s| !s.trim().is_empty())
            .map(|program| tty::Shell::new(program, spec.args.clone()));
        // Windows では項目が多いので、残りは既定値にする。
        #[allow(clippy::needless_update)]
        let options = tty::Options {
            shell,
            working_directory: spec.cwd.map(Into::into),
            drain_on_exit: true,
            env,
            ..tty::Options::default()
        };
        let pty = tty::new(&options, window_size(size), 0)?;
        let event_loop = EventLoop::new(term.clone(), listener, pty, true, false)?;
        let sender = event_loop.channel();
        let _io_thread = event_loop.spawn();

        let size = Arc::new(Mutex::new(size));
        let t = HostTerminal {
            term: term.clone(),
            sender: sender.clone(),
            size: size.clone(),
            wake: tx,
        };
        std::thread::Builder::new()
            .name(format!("terminal-{id}"))
            .spawn(move || pump(id, term, sender, size, rx, emit))?;
        Ok(t)
    }

    /// キー入力など。表示をいちばん下に戻してから送る。
    pub fn input(&self, bytes: Vec<u8>) {
        self.term.lock().scroll_display(Scroll::Bottom);
        let _ = self.sender.send(Msg::Input(Cow::Owned(bytes)));
    }

    pub fn scroll(&self, lines: i32) {
        self.term.lock().scroll_display(Scroll::Delta(lines));
        let _ = self.wake.send(TermEvent::Wakeup);
    }

    pub fn resize(&self, new: TermSize) {
        if new.cols == 0 || new.lines == 0 {
            return;
        }
        let mut size = self.size.lock().unwrap();
        if *size == new {
            return;
        }
        *size = new;
        self.term.lock().resize(Grid(new));
        let _ = self.sender.send(Msg::Resize(window_size(new)));
        let _ = self.wake.send(TermEvent::Wakeup);
    }
}

impl Drop for HostTerminal {
    fn drop(&mut self) {
        let _ = self.sender.send(Msg::Shutdown);
    }
}

/// 端末からの知らせを受けて、まとめて画面側へ送る。端末が閉じたら終わる。
fn pump(
    id: u64,
    term: Arc<FairMutex<Term<Listener>>>,
    sender: EventLoopSender,
    size: Arc<Mutex<TermSize>>,
    rx: Receiver<TermEvent>,
    emit: impl Fn(Event),
) {
    let mut sent: Vec<Vec<TermCell>> = Vec::new();
    let mut last = Instant::now() - FRAME;
    let mut dirty = true;
    loop {
        // 画面を送る時刻まで、知らせを集める。
        let wait = if dirty {
            FRAME.saturating_sub(last.elapsed())
        } else {
            Duration::from_secs(3600)
        };
        match rx.recv_timeout(wait) {
            Ok(event) => {
                match event {
                    TermEvent::Wakeup => dirty = true,
                    TermEvent::Title(t) => emit(Event::TerminalTitle { id, title: Some(t) }),
                    TermEvent::ResetTitle => emit(Event::TerminalTitle { id, title: None }),
                    TermEvent::PtyWrite(s) => {
                        let _ = sender.send(Msg::Input(Cow::Owned(s.into_bytes())));
                    }
                    TermEvent::ClipboardStore(_, text) => emit(Event::TerminalClipboard { id, text }),
                    // 画面側のクリップボードは、端末の中のプログラムには読ませない。
                    TermEvent::ClipboardLoad(_, format) => {
                        let _ = sender.send(Msg::Input(Cow::Owned(format("").into_bytes())));
                    }
                    TermEvent::TextAreaSizeRequest(format) => {
                        let s = *size.lock().unwrap();
                        let _ = sender.send(Msg::Input(Cow::Owned(format(window_size(s)).into_bytes())));
                    }
                    TermEvent::ChildExit(_) | TermEvent::Exit => {
                        send_screen(id, &term, &mut sent, &emit);
                        emit(Event::TerminalExited { id });
                        return;
                    }
                    TermEvent::Bell => emit(Event::TerminalBell { id }),
                    _ => {}
                }
                continue;
            }
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => return,
        }
        if dirty {
            send_screen(id, &term, &mut sent, &emit);
            last = Instant::now();
            dirty = false;
        }
    }
}

/// 前回送った画面と比べて、変わった行だけを送る。
fn send_screen(id: u64, term: &FairMutex<Term<Listener>>, sent: &mut Vec<Vec<TermCell>>, emit: &impl Fn(Event)) {
    let update = {
        let term = term.lock();
        snapshot(id, &term, sent)
    };
    emit(Event::TerminalUpdate(update));
}

fn snapshot(id: u64, term: &Term<Listener>, sent: &mut Vec<Vec<TermCell>>) -> TermUpdate {
    let content = term.renderable_content();
    let lines = term.screen_lines();
    let cols = term.columns();
    let offset = content.display_offset as i32;
    let colors = content.colors;
    let mut rows: Vec<Vec<TermCell>> = vec![Vec::new(); lines];
    let to_color = |c: AnsiColor| -> TermColor {
        match c {
            AnsiColor::Named(n) => match colors[n] {
                // OSC でプログラムが色を変えたら、それに従う。
                Some(rgb) if (n as usize) < 16 => TermColor::Rgb(rgb.r, rgb.g, rgb.b),
                _ => TermColor::Named(n as u16),
            },
            AnsiColor::Indexed(i) => match colors[i as usize] {
                Some(rgb) => TermColor::Rgb(rgb.r, rgb.g, rgb.b),
                None => TermColor::Indexed(i),
            },
            AnsiColor::Spec(rgb) => TermColor::Rgb(rgb.r, rgb.g, rgb.b),
        }
    };
    for indexed in content.display_iter {
        let row = indexed.point.line.0 + offset;
        if row < 0 || row as usize >= lines {
            continue;
        }
        let row = &mut rows[row as usize];
        let col = indexed.point.column.0;
        let cell = &indexed.cell;
        // 全角文字の後ろの空きマスも、列をそろえるために空のマスとして入れる。
        while row.len() < col {
            row.push(blank());
        }
        if cell
            .flags
            .intersects(Flags::WIDE_CHAR_SPACER | Flags::LEADING_WIDE_CHAR_SPACER)
        {
            row.push(TermCell {
                text: String::new(),
                ..blank()
            });
            continue;
        }
        let mut text = cell.c.to_string();
        if let Some(zw) = cell.zerowidth() {
            text.extend(zw.iter());
        }
        let f = cell.flags;
        let mut flags = 0;
        for (from, to) in [
            (Flags::BOLD, cell_flags::BOLD),
            (Flags::ITALIC, cell_flags::ITALIC),
            (Flags::UNDERCURL, cell_flags::UNDERCURL),
            (Flags::STRIKEOUT, cell_flags::STRIKEOUT),
            (Flags::DIM, cell_flags::DIM),
            (Flags::INVERSE, cell_flags::INVERSE),
            (Flags::HIDDEN, cell_flags::HIDDEN),
            (Flags::WIDE_CHAR, cell_flags::WIDE),
        ] {
            if f.contains(from) {
                flags |= to;
            }
        }
        if f.intersects(Flags::ALL_UNDERLINES) {
            flags |= cell_flags::UNDERLINE;
        }
        row.push(TermCell {
            text,
            fg: to_color(cell.fg),
            bg: to_color(cell.bg),
            flags,
        });
    }
    // 行末の空白（既定の色のもの）は送らない。
    for row in rows.iter_mut() {
        while row.last().is_some_and(|c| *c == blank()) {
            row.pop();
        }
    }
    let cursor_row = content.cursor.point.line.0 + offset;
    let cursor = (content.mode.contains(TermMode::SHOW_CURSOR)
        && content.cursor.shape != AnsiCursor::Hidden
        && cursor_row >= 0
        && (cursor_row as usize) < lines)
        .then_some(TermCursor {
            col: content.cursor.point.column.0 as u16,
            row: cursor_row.max(0) as u16,
            shape: match content.cursor.shape {
                AnsiCursor::Underline => CursorShape::Underline,
                AnsiCursor::Beam => CursorShape::Beam,
                AnsiCursor::HollowBlock => CursorShape::HollowBlock,
                _ => CursorShape::Block,
            },
        });
    let mut mode = 0;
    if content.mode.contains(TermMode::APP_CURSOR) {
        mode |= term_mode::APP_CURSOR;
    }
    if content.mode.contains(TermMode::BRACKETED_PASTE) {
        mode |= term_mode::BRACKETED_PASTE;
    }
    // 大きさが変わったら、すべての行を送り直す。
    if sent.len() != lines {
        sent.clear();
        sent.resize(lines, vec![blank(); 1]);
    }
    let mut changed = Vec::new();
    for (i, r) in rows.into_iter().enumerate() {
        if sent[i] != r {
            sent[i] = r.clone();
            changed.push((i as u16, r));
        }
    }
    TermUpdate {
        id,
        cols: cols as u16,
        lines: lines as u16,
        rows: changed,
        cursor,
        mode,
    }
}

fn blank() -> TermCell {
    TermCell {
        text: " ".into(),
        fg: TermColor::Named(256),
        bg: TermColor::Named(257),
        flags: 0,
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;

    #[test]
    fn sends_screen_updates_and_echoes_input() {
        let (tx, rx) = channel();
        let t = HostTerminal::open(
            7,
            TerminalSpec {
                shell: Some("sh".into()),
                scrollback: 100,
                size: TermSize {
                    cols: 40,
                    lines: 5,
                    cell_width: 8,
                    cell_height: 16,
                },
                ..Default::default()
            },
            move |e| {
                let _ = tx.send(e);
            },
        )
        .unwrap();
        t.input(b"echo hello-nucleus\n".to_vec());
        let deadline = Instant::now() + Duration::from_secs(10);
        let mut screen: Vec<String> = vec![String::new(); 5];
        while Instant::now() < deadline {
            if let Ok(Event::TerminalUpdate(u)) = rx.recv_timeout(Duration::from_millis(200)) {
                assert_eq!((u.id, u.cols, u.lines), (7, 40, 5));
                for (i, row) in u.rows {
                    screen[i as usize] = row.iter().map(|c| c.text.as_str()).collect();
                }
                if screen.iter().filter(|l| l.contains("hello-nucleus")).count() >= 2 {
                    return;
                }
            }
        }
        panic!("output did not arrive: {screen:?}");
    }
}
