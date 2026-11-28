//! UI とホストのあいだで使う型（ブラウザ版でも使う）。

use std::path::PathBuf;

use futures::channel::oneshot;
use serde::{Deserialize, Serialize};

/// UI スレッドにお願いすること。workspace が受け取って処理する。
pub enum UiRequest {
    Notify {
        extension: String,
        level: NoticeLevel,
        message: String,
    },
    ActiveDocument {
        reply: oneshot::Sender<Option<ActiveDocument>>,
    },
    ReplaceActiveText {
        text: String,
        reply: oneshot::Sender<Result<(), String>>,
    },
    /// ワークスペース内のファイルを開く（絶対パス）。
    OpenFile {
        path: PathBuf,
        line: Option<u32>,
    },
    /// エディタの表示の状態。
    EditorView {
        editor: u32,
        reply: oneshot::Sender<Option<EditorViewInfo>>,
    },
    EditorText {
        editor: u32,
        reply: oneshot::Sender<Option<String>>,
    },
    ScrollEditor {
        editor: u32,
        line: u32,
    },
    /// プロジェクトツリーの印を置き換える（この Extension の分、絶対パス）。
    DecorateFiles {
        extension: String,
        marks: Vec<(PathBuf, FileState)>,
    },
    /// 差分を開く（すでに開いていれば元を新しくする）。
    ShowDiff {
        extension: String,
        path: PathBuf,
        rel: String,
        base: String,
        base_label: String,
    },
    /// Extension の状態が変わった。そのビューを描き直す。
    Invalidate {
        extension: String,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum NoticeLevel {
    Info,
    Warning,
    Error,
}

/// プロジェクトツリーに出すファイルの状態。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FileState {
    Modified,
    Added,
    Untracked,
    Deleted,
    Renamed,
    Conflicted,
    Ignored,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct EditorViewInfo {
    pub version: u64,
    pub first_line: u32,
    pub last_line: u32,
    pub line_count: u32,
    pub cursor_line: u32,
    pub height: f32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ActiveDocument {
    /// ワークスペースからの相対パス（ワークスペースの外なら絶対パス）。
    pub path: String,
    pub language: String,
    pub text: String,
}
