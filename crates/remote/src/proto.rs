//! 画面とホストのあいだで送り合うもの。
//!
//! どちらの向きも [`Frame`] を postcard で符号化して送る。同じ PC の中（デスクトップ版）でも、
//! ネットワーク越し（ブラウザ版・リモート接続）でも、同じ形で送る（どちらでも同じように動くように）。
//!
//! パスは文字列で送る。区切りは常に `/`（Windows のホストでも。`C:/work/a.rs` のように送る）。
//!
//! **互換性**: 送る形を変えるときは、enum の最後に足すだけにする（途中に挿入・削除・並べ替えをしない）。
//! postcard は enum を番号で送るので、並びが変わると古い相手と話せなくなる。

use serde::{Deserialize, Serialize};

use project::{Entry, search::SearchOptions, search::SearchResult};

/// 取り決めの版。互換性のない変更をしたら上げる。
pub const PROTOCOL_VERSION: u32 = 1;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Frame {
    /// 画面 → ホスト。
    Request { id: u64, request: Request },
    /// 画面 → ホスト。途中で要らなくなった（検索の打ち切りなど）。
    Cancel { id: u64 },
    /// ホスト → 画面。
    Response {
        id: u64,
        result: Result<Response, RemoteError>,
    },
    /// ホスト → 画面。頼まれていない知らせ（ファイルの変更など）。
    Event(Event),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Request {
    /// ファイルを読む。`max_size` より大きければ [`ErrorKind::TooLarge`]。
    ReadFile {
        path: String,
        max_size: u64,
    },
    /// ファイルを書く。一時ファイルに書いてから置き換える（元の権限を保つ）。
    WriteFile {
        path: String,
        bytes: Vec<u8>,
    },
    /// フォルダの直下（`.gitignore` を反映し、`.git` は除く。フォルダが先）。
    ReadDir {
        path: String,
    },
    /// 空のファイルを作る。すでにあれば [`ErrorKind::AlreadyExists`]。
    CreateFile {
        path: String,
    },
    /// フォルダを作る（途中のフォルダも）。
    CreateDir {
        path: String,
    },
    /// 名前の変更・移動。移動先がすでにあれば [`ErrorKind::AlreadyExists`]。
    Rename {
        from: String,
        to: String,
    },
    /// ゴミ箱へ移す。
    Trash {
        path: String,
    },
    /// あるかどうか（ファイルでもフォルダでも）。
    Exists {
        path: String,
    },
    /// フォルダを（再帰せずに）見張る。変わったら [`Event::DirChanged`]。
    Watch {
        dir: String,
    },
    Unwatch {
        dir: String,
    },
    /// プロジェクト内のファイルの一覧（ルートからの相対パス、`/` 区切り）。
    ListFiles {
        root: String,
    },
    /// プロジェクト全体の検索。
    Search {
        root: String,
        options: SearchOptions,
        limit: usize,
    },
    /// プログラムを動かして、終わるのを待つ（git など）。端末の入力は求めない（`GIT_TERMINAL_PROMPT=0` など）。
    RunProcess(ProcessSpec),
    /// ターミナルを開く。返事は [`Response::Terminal`]。画面は [`Event::TerminalUpdate`] で届く。
    TerminalOpen(TerminalSpec),
    /// キー入力など（表示を最下部に戻してから送る）。
    TerminalInput {
        id: u64,
        bytes: Vec<u8>,
    },
    TerminalResize {
        id: u64,
        size: TermSize,
    },
    /// 表示を上下に動かす（行数。上が正）。
    TerminalScroll {
        id: u64,
        lines: i32,
    },
    TerminalClose {
        id: u64,
    },
    /// 言語サーバーを起動する。返事は [`Response::Server`]。サーバーからのメッセージは [`Event::LspMessage`]。
    LspStart {
        command: String,
        args: Vec<String>,
        cwd: String,
    },
    /// 言語サーバーへメッセージ（JSON）を送る。送った順に届く。
    LspSend {
        id: u64,
        body: Vec<u8>,
    },
    LspStop {
        id: u64,
    },
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct TerminalSpec {
    pub cwd: Option<String>,
    /// null なら OS の既定のシェル。
    pub shell: Option<String>,
    pub args: Vec<String>,
    pub scrollback: usize,
    pub size: TermSize,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TermSize {
    pub cols: u16,
    pub lines: u16,
    pub cell_width: u16,
    pub cell_height: u16,
}

/// 画面の変わったところ。
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct TermUpdate {
    pub id: u64,
    pub cols: u16,
    pub lines: u16,
    /// 変わった行（行番号と、その行の文字。行末の空白は省く）。
    pub rows: Vec<(u16, Vec<TermCell>)>,
    pub cursor: Option<TermCursor>,
    /// [`term_mode`] の組み合わせ。
    pub mode: u32,
}

/// 端末のモード（[`TermUpdate::mode`]）。
pub mod term_mode {
    pub const APP_CURSOR: u32 = 1;
    pub const BRACKETED_PASTE: u32 = 2;
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TermCell {
    /// 文字（結合文字を含む）。
    pub text: String,
    pub fg: TermColor,
    pub bg: TermColor,
    /// [`cell_flags`] の組み合わせ。
    pub flags: u16,
}

/// 文字の飾り（[`TermCell::flags`]）。
pub mod cell_flags {
    pub const BOLD: u16 = 1;
    pub const ITALIC: u16 = 2;
    pub const UNDERLINE: u16 = 4;
    pub const UNDERCURL: u16 = 8;
    pub const STRIKEOUT: u16 = 16;
    pub const DIM: u16 = 32;
    pub const INVERSE: u16 = 64;
    pub const HIDDEN: u16 = 128;
    /// 全角（2 マス）。
    pub const WIDE: u16 = 256;
}

/// 端末の色。名前の色（前景・背景・0〜15 など）は画面側がテーマの色にする。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TermColor {
    /// 0〜15 は ANSI の色。256 前景、257 背景、258 カーソル、259〜266 薄い 0〜7、267 明るい前景、268 薄い前景。
    Named(u16),
    Indexed(u8),
    Rgb(u8, u8, u8),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TermCursor {
    pub col: u16,
    pub row: u16,
    pub shape: CursorShape,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum CursorShape {
    Block,
    Underline,
    Beam,
    HollowBlock,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ProcessSpec {
    pub program: String,
    pub args: Vec<String>,
    pub cwd: Option<String>,
    pub env: Vec<(String, String)>,
    /// 標準入力に渡すもの（なければ空の入力）。
    pub stdin: Option<Vec<u8>>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProcessOutput {
    /// 終了コード（シグナルで終わったときは None）。
    pub code: Option<i32>,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
}

impl ProcessOutput {
    pub fn success(&self) -> bool {
        self.code == Some(0)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Response {
    Done,
    Bytes(Vec<u8>),
    Entries(Vec<Entry>),
    Bool(bool),
    Paths(Vec<String>),
    Search(SearchResult),
    Output(ProcessOutput),
    Terminal(u64),
    Server(u64),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Event {
    /// 見張っているフォルダの中身が変わった。
    DirChanged(String),
    TerminalUpdate(TermUpdate),
    TerminalTitle {
        id: u64,
        title: Option<String>,
    },
    TerminalExited {
        id: u64,
    },
    TerminalBell {
        id: u64,
    },
    /// 端末の中のプログラムがクリップボードに書いた（OSC 52）。
    TerminalClipboard {
        id: u64,
        text: String,
    },
    /// 言語サーバーからのメッセージ（JSON）。
    LspMessage {
        id: u64,
        body: Vec<u8>,
    },
    /// 言語サーバーが終わった。
    LspExited {
        id: u64,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ErrorKind {
    NotFound,
    AlreadyExists,
    PermissionDenied,
    /// ファイルが大きすぎる（バイト数）。
    TooLarge(u64),
    Cancelled,
    /// ホストとのつながりが切れた。
    Disconnected,
    /// この接続には許されていない操作（共同開発の権限など）。
    NotAllowed,
    /// 返事の形が合わない（取り決めの版違いなど）。
    Protocol,
    Other,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RemoteError {
    pub kind: ErrorKind,
    pub message: String,
}

impl RemoteError {
    pub fn new(kind: ErrorKind, message: impl Into<String>) -> RemoteError {
        RemoteError {
            kind,
            message: message.into(),
        }
    }

    pub fn other(message: impl Into<String>) -> RemoteError {
        RemoteError::new(ErrorKind::Other, message)
    }
}

impl std::fmt::Display for RemoteError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for RemoteError {}

impl From<std::io::Error> for RemoteError {
    fn from(e: std::io::Error) -> RemoteError {
        use std::io::ErrorKind as K;
        let kind = match e.kind() {
            K::NotFound => ErrorKind::NotFound,
            K::AlreadyExists => ErrorKind::AlreadyExists,
            K::PermissionDenied => ErrorKind::PermissionDenied,
            _ => ErrorKind::Other,
        };
        RemoteError::new(kind, e.to_string())
    }
}

pub fn encode(frame: &Frame) -> Vec<u8> {
    postcard::to_allocvec(frame).expect("frames always encode")
}

pub fn decode(bytes: &[u8]) -> Result<Frame, RemoteError> {
    postcard::from_bytes(bytes).map_err(|e| RemoteError::new(ErrorKind::Protocol, e.to_string()))
}
