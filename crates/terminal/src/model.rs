//! 端末プロセスと画面の状態。

use std::{borrow::Cow, collections::HashMap, path::PathBuf, sync::Arc};

use alacritty_terminal::{
    event::{Event as TermEvent, EventListener, WindowSize},
    event_loop::{EventLoop, EventLoopSender, Msg},
    grid::{Dimensions, Scroll},
    sync::FairMutex,
    term::{Config, Term, TermMode},
    tty,
};
use futures::{StreamExt as _, channel::mpsc::UnboundedSender};
use gpui_kit::{ClipboardItem, Context, EventEmitter, Task};

#[derive(Clone)]
pub struct Listener(UnboundedSender<TermEvent>);

impl EventListener for Listener {
    fn send_event(&self, event: TermEvent) {
        let _ = self.0.unbounded_send(event);
    }
}

/// 画面の大きさ（セル数とセルの大きさ）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GridSize {
    pub cols: u16,
    pub lines: u16,
    pub cell_width: u16,
    pub cell_height: u16,
}

impl Dimensions for GridSize {
    fn total_lines(&self) -> usize {
        self.lines as usize
    }
    fn screen_lines(&self) -> usize {
        self.lines as usize
    }
    fn columns(&self) -> usize {
        self.cols as usize
    }
}

impl From<GridSize> for WindowSize {
    fn from(s: GridSize) -> Self {
        WindowSize {
            num_lines: s.lines,
            num_cols: s.cols,
            cell_width: s.cell_width,
            cell_height: s.cell_height,
        }
    }
}

pub enum TerminalEvent {
    TitleChanged,
    Exited,
    Bell,
}

pub struct Terminal {
    pub(crate) term: Arc<FairMutex<Term<Listener>>>,
    sender: EventLoopSender,
    size: GridSize,
    title: Option<String>,
    exited: bool,
    _events: Task<()>,
}

/// 起動した端末（まだ gpui の Entity になっていないもの）。
pub struct Spawned {
    term: Arc<FairMutex<Term<Listener>>>,
    sender: EventLoopSender,
    events: futures::channel::mpsc::UnboundedReceiver<TermEvent>,
    size: GridSize,
}

/// シェルを起動する。`cwd` はプロジェクトのルートなど。PTY の読み書きは専用スレッドで始まる。
pub fn spawn(cwd: Option<PathBuf>, settings: &settings::TerminalSettings) -> anyhow::Result<Spawned> {
    let size = GridSize {
        cols: 80,
        lines: 24,
        cell_width: 8,
        cell_height: 18,
    };
    let (tx, events) = futures::channel::mpsc::unbounded();
    let listener = Listener(tx);
    let config = Config {
        scrolling_history: settings.scrollback.min(1_000_000),
        ..Config::default()
    };
    let term = Arc::new(FairMutex::new(Term::new(config, &size, listener.clone())));

    let mut env = HashMap::new();
    env.insert("TERM".to_string(), "xterm-256color".to_string());
    env.insert("COLORTERM".to_string(), "truecolor".to_string());
    env.insert("TERM_PROGRAM".to_string(), "nucleus".to_string());
    // terminal.shell: 決めてあればそのシェル、なければ OS の既定。
    let shell = settings
        .shell
        .as_ref()
        .filter(|s| !s.trim().is_empty())
        .map(|program| tty::Shell::new(program.clone(), settings.shell_args.clone()));
    // Windows では項目が多いので、残りは既定値にする。
    #[allow(clippy::needless_update)]
    let options = tty::Options {
        shell,
        working_directory: cwd,
        drain_on_exit: true,
        env,
        ..tty::Options::default()
    };
    let pty = tty::new(&options, size.into(), 0)?;
    let event_loop = EventLoop::new(term.clone(), listener, pty, true, false)?;
    let sender = event_loop.channel();
    let _io_thread = event_loop.spawn();
    Ok(Spawned {
        term,
        sender,
        events,
        size,
    })
}

impl Terminal {
    pub fn new(spawned: Spawned, cx: &mut Context<Self>) -> Self {
        let Spawned {
            term,
            sender,
            mut events,
            size,
        } = spawned;
        // 端末からの通知は、届いた分をまとめて1回の描画にする。
        let task = cx.spawn(async move |this, cx| {
            while let Some(first) = events.next().await {
                let mut batch = vec![first];
                while let Ok(e) = events.try_recv() {
                    batch.push(e);
                }
                let alive = this.update(cx, |this, cx| {
                    for e in batch {
                        this.handle_event(e, cx);
                    }
                    cx.notify();
                });
                if alive.is_err() {
                    break;
                }
            }
        });
        Terminal {
            term,
            sender,
            size,
            title: None,
            exited: false,
            _events: task,
        }
    }

    fn handle_event(&mut self, event: TermEvent, cx: &mut Context<Self>) {
        match event {
            TermEvent::Title(t) => {
                self.title = Some(t);
                cx.emit(TerminalEvent::TitleChanged);
            }
            TermEvent::ResetTitle => {
                self.title = None;
                cx.emit(TerminalEvent::TitleChanged);
            }
            TermEvent::PtyWrite(s) => self.write(Cow::Owned(s.into_bytes())),
            TermEvent::ClipboardStore(_, text) => cx.write_to_clipboard(ClipboardItem::new_string(text)),
            TermEvent::ClipboardLoad(_, format) => {
                let text = cx.read_from_clipboard().and_then(|c| c.text()).unwrap_or_default();
                self.write(Cow::Owned(format(&text).into_bytes()));
            }
            TermEvent::TextAreaSizeRequest(format) => {
                let text = format(self.size.into());
                self.write(Cow::Owned(text.into_bytes()));
            }
            TermEvent::ChildExit(_) | TermEvent::Exit => {
                self.exited = true;
                cx.emit(TerminalEvent::Exited);
            }
            TermEvent::Bell => cx.emit(TerminalEvent::Bell),
            _ => {}
        }
    }

    pub fn title(&self) -> Option<&str> {
        self.title.as_deref()
    }

    pub fn has_exited(&self) -> bool {
        self.exited
    }

    pub fn mode(&self) -> TermMode {
        *self.term.lock().mode()
    }

    /// 端末へ書き込む（PTY への書き込みは EventLoop のスレッドが行う）。
    pub fn write(&self, bytes: Cow<'static, [u8]>) {
        let _ = self.sender.send(Msg::Input(bytes));
    }

    /// キー入力。スクロールしていたら最下部に戻してから送る。
    pub fn input(&self, bytes: Vec<u8>) {
        self.term.lock().scroll_display(Scroll::Bottom);
        self.write(Cow::Owned(bytes));
    }

    pub fn scroll(&self, lines: i32) {
        self.term.lock().scroll_display(Scroll::Delta(lines));
    }

    pub fn resize(&mut self, size: GridSize) {
        if size == self.size || size.cols == 0 || size.lines == 0 {
            return;
        }
        self.size = size;
        self.term.lock().resize(size);
        let _ = self.sender.send(Msg::Resize(size.into()));
    }
}

impl Drop for Terminal {
    fn drop(&mut self) {
        let _ = self.sender.send(Msg::Shutdown);
    }
}

impl EventEmitter<TerminalEvent> for Terminal {}
