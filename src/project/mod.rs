/// プロジェクト単位の状態管理（ワークツリー、バッファストア、LSP等の統合）モジュール

pub mod buffer_store;

use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use crate::editor::buffer::TextBuffer;
use buffer_store::BufferStore;

/// プロジェクト全体の状態を管理する構造体
pub struct Project {
    /// プロジェクトのルートディレクトリ
    pub root_path: Option<PathBuf>,
    /// バッファストア
    pub buffer_store: BufferStore,
}

impl Project {
    /// 新しいプロジェクトを作成
    pub fn new(root_path: Option<PathBuf>) -> Self {
        Self {
            root_path,
            buffer_store: BufferStore::new(),
        }
    }

    /// ルートパスを設定
    pub fn set_root(&mut self, root_path: Option<PathBuf>) {
        self.root_path = root_path;
    }

    /// ファイルを開いてバッファを取得
    pub fn open_file(&mut self, path: &Path) -> std::io::Result<Arc<RwLock<TextBuffer>>> {
        self.buffer_store.open_file(path)
    }
}
