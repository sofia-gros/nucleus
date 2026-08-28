/// プロジェクト内で開かれているテキストバッファを一元管理するストア

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use crate::editor::buffer::TextBuffer;

/// 開かれているすべてのバッファを管理するストア
#[derive(Default)]
pub struct BufferStore {
    /// ファイルパスからバッファへのマップ
    buffers: HashMap<PathBuf, Arc<RwLock<TextBuffer>>>,
}

impl BufferStore {
    /// 新しい BufferStore を作成
    pub fn new() -> Self {
        Self {
            buffers: HashMap::new(),
        }
    }

    /// ファイルパスに対応するバッファを開く（既に開かれている場合は既存のものを返す）
    pub fn open_file(&mut self, path: &Path) -> std::io::Result<Arc<RwLock<TextBuffer>>> {
        let canonical_path = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
        if let Some(buf) = self.buffers.get(&canonical_path) {
            return Ok(buf.clone());
        }

        let content = std::fs::read_to_string(path)?;
        let buffer = TextBuffer::from_file(&canonical_path, &content);
        let arc_buf = Arc::new(RwLock::new(buffer));
        self.buffers.insert(canonical_path, arc_buf.clone());
        Ok(arc_buf)
    }

    /// メモリ上に新規バッファを作成
    pub fn create_buffer(&mut self, title: Option<&str>, content: &str) -> Arc<RwLock<TextBuffer>> {
        let mut buffer = TextBuffer::new(content);
        if let Some(t) = title {
            buffer.file_path = Some(PathBuf::from(t));
        }
        let arc_buf = Arc::new(RwLock::new(buffer));
        if let Some(t) = title {
            self.buffers.insert(PathBuf::from(t), arc_buf.clone());
        }
        arc_buf
    }

    /// パスに紐づくバッファを取得
    pub fn get(&self, path: &Path) -> Option<Arc<RwLock<TextBuffer>>> {
        let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
        self.buffers.get(&canonical).cloned()
    }

    /// パスに紐づくバッファを閉じる
    pub fn close(&mut self, path: &Path) {
        let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
        self.buffers.remove(&canonical);
    }

    /// 開かれているすべてのバッファ一覧を取得
    pub fn all_buffers(&self) -> Vec<Arc<RwLock<TextBuffer>>> {
        self.buffers.values().cloned().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_buffer_store_memory_buffer() {
        let mut store = BufferStore::new();
        let buf = store.create_buffer(Some("test.txt"), "Hello Store");
        {
            let b = buf.read().unwrap();
            assert_eq!(b.to_string(), "Hello Store");
        }

        let retrieved = store.get(Path::new("test.txt"));
        assert!(retrieved.is_some());
    }
}
