use std::path::{Path, PathBuf};
use std::fs;

#[derive(Clone, Debug, PartialEq)]
pub enum FileType {
    File,
    Directory,
}

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
        let mut entries = Vec::new();
        if let Ok(dir) = fs::read_dir(path) {
            for entry in dir.filter_map(Result::ok) {
                entries.push(FileEntry::new(entry.path()));
            }
            // Sort: directories first, then alphabetical
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
