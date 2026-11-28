//! 端末の画面の状態。端末そのもの（シェルと VT の解釈）はホストが動かし、変わった行がここへ届く。

use std::path::PathBuf;

use futures::StreamExt as _;
use gpui_kit::{ClipboardItem, Context, EventEmitter, Task};
use remote::{Event, RemoteExt as _, Request, TermCell, TermCursor, TermSize, TerminalSpec};

pub use remote::TermSize as GridSize;

pub enum TerminalEvent {
    TitleChanged,
    Exited,
    Bell,
}

pub struct Terminal {
    client: remote::Client,
    /// ホストでの番号（開き終わるまでは None）。
    id: Option<u64>,
    size: TermSize,
    pub(crate) rows: Vec<Vec<TermCell>>,
    pub(crate) cursor: Option<TermCursor>,
    mode: u32,
    title: Option<String>,
    exited: bool,
    error: Option<String>,
    _task: Task<()>,
}

impl Terminal {
    /// ホストにシェルを起動してもらう。`cwd` はプロジェクトのルートなど。
    pub fn open(cwd: Option<PathBuf>, cx: &mut Context<Self>) -> Self {
        let s = settings::ActiveSettings::get(cx).terminal.clone();
        let size = TermSize {
            cols: 80,
            lines: 24,
            cell_width: 8,
            cell_height: 18,
        };
        let client = cx.remote();
        // ターミナルの知らせだけを受け取る（番号は開くまで分からないので、ここでは種類だけで選ぶ）。
        let mut events = client.subscribe_where(|e| {
            matches!(
                e,
                Event::TerminalUpdate(_)
                    | Event::TerminalTitle { .. }
                    | Event::TerminalExited { .. }
                    | Event::TerminalBell { .. }
                    | Event::TerminalClipboard { .. }
            )
        });
        let open = client.terminal_open(TerminalSpec {
            cwd: cwd.as_deref().map(remote::wire),
            shell: s.shell.clone(),
            args: s.shell_args.clone(),
            scrollback: s.scrollback,
            size,
        });
        let client2 = client.clone();
        let task = cx.spawn(async move |this, cx| {
            let client = client2;
            let id = match open.await {
                Ok(id) => id,
                Err(e) => {
                    this.update(cx, |t, cx| {
                        t.error = Some(e.to_string());
                        cx.notify();
                    })
                    .ok();
                    return;
                }
            };
            let alive = this.update(cx, |t, _| {
                t.id = Some(id);
                // 開いている間に大きさが変わっていたら伝える。
                if t.size != size {
                    client.send(Request::TerminalResize { id, size: t.size });
                }
            });
            if alive.is_err() {
                return;
            }
            // 届いた知らせをまとめて 1 回の描画にする。
            while let Some(first) = events.next().await {
                let mut batch = vec![first];
                while let Ok(e) = events.try_recv() {
                    batch.push(e);
                }
                let alive = this.update(cx, |t, cx| {
                    for e in batch {
                        t.handle(id, e, cx);
                    }
                });
                if alive.is_err() {
                    break;
                }
            }
        });
        Terminal {
            client,
            id: None,
            size,
            rows: Vec::new(),
            cursor: None,
            mode: 0,
            title: None,
            exited: false,
            error: None,
            _task: task,
        }
    }

    fn handle(&mut self, id: u64, event: Event, cx: &mut Context<Self>) {
        match event {
            Event::TerminalUpdate(u) if u.id == id => {
                self.rows.resize(u.lines as usize, Vec::new());
                for (row, cells) in u.rows {
                    if let Some(r) = self.rows.get_mut(row as usize) {
                        *r = cells;
                    }
                }
                self.cursor = u.cursor;
                self.mode = u.mode;
                cx.notify();
            }
            Event::TerminalTitle { id: t, title } if t == id => {
                self.title = title;
                cx.emit(TerminalEvent::TitleChanged);
            }
            Event::TerminalExited { id: t } if t == id => {
                self.exited = true;
                cx.emit(TerminalEvent::Exited);
                cx.notify();
            }
            Event::TerminalBell { id: t } if t == id => cx.emit(TerminalEvent::Bell),
            Event::TerminalClipboard { id: t, text } if t == id => {
                cx.write_to_clipboard(ClipboardItem::new_string(text));
            }
            _ => {}
        }
    }

    pub fn title(&self) -> Option<&str> {
        self.title.as_deref()
    }

    pub fn has_exited(&self) -> bool {
        self.exited
    }

    /// 起動できなかったときの理由。
    pub fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }

    /// [`remote::term_mode`] の組み合わせ。
    pub fn mode(&self) -> u32 {
        self.mode
    }

    /// キー入力（表示はいちばん下に戻る）。
    pub fn input(&self, bytes: Vec<u8>) {
        if let Some(id) = self.id {
            self.client.send(Request::TerminalInput { id, bytes });
        }
    }

    pub fn scroll(&self, lines: i32) {
        if let Some(id) = self.id {
            self.client.send(Request::TerminalScroll { id, lines });
        }
    }

    pub fn resize(&mut self, size: TermSize) {
        if size == self.size || size.cols == 0 || size.lines == 0 {
            return;
        }
        self.size = size;
        if let Some(id) = self.id {
            self.client.send(Request::TerminalResize { id, size });
        }
    }
}

/// ビューを閉じたら、ホストの端末（シェル）も終わらせる。
impl Drop for Terminal {
    fn drop(&mut self) {
        if let Some(id) = self.id.take() {
            self.client.send(Request::TerminalClose { id });
        }
    }
}

impl EventEmitter<TerminalEvent> for Terminal {}
