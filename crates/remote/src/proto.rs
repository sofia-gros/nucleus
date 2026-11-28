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
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Response {
    Done,
    Bytes(Vec<u8>),
    Entries(Vec<Entry>),
    Bool(bool),
    Paths(Vec<String>),
    Search(SearchResult),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Event {
    /// 見張っているフォルダの中身が変わった。
    DirChanged(String),
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
