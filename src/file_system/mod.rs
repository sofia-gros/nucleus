/// ファイルシステム抽象化およびディレクトリ走査モジュール

pub mod watcher;
pub mod gitignore;

use std::path::{Path, PathBuf};
use std::fs;
use gitignore::GitIgnore;

/// ファイルの種類（ファイルまたはディレクトリ）
#[derive(Clone, Debug, PartialEq)]
pub enum FileType {
    File,
    Directory,
}

/// ファイル・ディレクトリエントリ
#[derive(Clone, Debug)]
pub struct FileEntry {
    pub name: String,
    pub path: PathBuf,
    pub file_type: FileType,
    pub children: Option<Vec<FileEntry>>,
    pub is_expanded: bool,
}

impl FileEntry {
    pub fn new(path: PathBuf) -> Self {
        let name = path.file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| path.to_string_lossy().into_owned());
            
        let file_type = if path.is_dir() {
            FileType::Directory
        } else {
            FileType::File
        };

        Self {
            name,
            path,
            file_type,
            children: None,
            is_expanded: false,
        }
    }

    pub fn read_dir(path: &Path) -> Option<Vec<FileEntry>> {
        let gitignore = if let Ok(content) = fs::read_to_string(path.join(".gitignore")) {
            GitIgnore::parse(&content)
        } else {
            GitIgnore::default()
        };

        Self::read_dir_with_depth(path, 0, &gitignore)
    }

    fn read_dir_with_depth(path: &Path, depth: usize, gitignore: &GitIgnore) -> Option<Vec<FileEntry>> {
        if depth > 5 {
            return None; // Max depth
        }

        let mut entries = Vec::new();
        if let Ok(dir) = fs::read_dir(path) {
            for entry in dir.filter_map(Result::ok) {
                let entry_path = entry.path();
                if gitignore.is_ignored(&entry_path) {
                    continue;
                }
                
                let mut fe = FileEntry::new(entry_path.clone());
                if fe.file_type == FileType::Directory {
                    fe.children = Self::read_dir_with_depth(&entry_path, depth + 1, gitignore);
                }
                entries.push(fe);
            }
            // フォルダ優先、名前順ソート
            entries.sort_by(|a, b| {
                if a.file_type == b.file_type {
                    a.name.cmp(&b.name)
                } else if a.file_type == FileType::Directory {
                    std::cmp::Ordering::Less
                } else {
                    std::cmp::Ordering::Greater
                }
            });
            Some(entries)
        } else {
            None
        }
    }
}
