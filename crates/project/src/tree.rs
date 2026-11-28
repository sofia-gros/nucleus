use serde::{Deserialize, Serialize};
use std::{
    cmp::Ordering,
    collections::{HashMap, HashSet},
    io,
    path::{Path, PathBuf},
    sync::Arc,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum EntryKind {
    Dir,
    File,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Entry {
    pub name: String,
    pub path: PathBuf,
    pub kind: EntryKind,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DirState {
    /// 読み込み中。前回の内容があれば表示し続ける。
    Loading(Option<Arc<Vec<Entry>>>),
    Loaded(Arc<Vec<Entry>>),
    Error(String),
}

/// ツリーの1行（表示用）。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Row {
    pub path: PathBuf,
    pub name: String,
    pub depth: usize,
    pub kind: EntryKind,
    pub expanded: bool,
    pub loading: bool,
}

/// 開いているフォルダのツリー。
#[derive(Debug)]
pub struct ProjectTree {
    root: PathBuf,
    dirs: HashMap<PathBuf, DirState>,
    expanded: HashSet<PathBuf>,
    rows: Arc<Vec<Row>>,
}

impl ProjectTree {
    /// ルートは展開した状態で始まる。返り値の 2 つ目は、読み込むべきフォルダ（ルート）。
    pub fn new(root: PathBuf) -> (ProjectTree, PathBuf) {
        let mut t = ProjectTree {
            root: root.clone(),
            dirs: HashMap::new(),
            expanded: HashSet::new(),
            rows: Arc::new(Vec::new()),
        };
        t.expanded.insert(root.clone());
        t.dirs.insert(root.clone(), DirState::Loading(None));
        t.rebuild();
        (t, root)
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn root_name(&self) -> String {
        self.root
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| self.root.display().to_string())
    }

    /// 表示する行（展開されていて読み込み済みのフォルダを深さ優先でたどる）。
    pub fn rows(&self) -> Arc<Vec<Row>> {
        self.rows.clone()
    }

    pub fn is_expanded(&self, dir: &Path) -> bool {
        self.expanded.contains(dir)
    }

    /// 展開・折りたたみを切り替える。新たに読み込みが必要なら、そのフォルダを返す。
    pub fn toggle(&mut self, dir: &Path) -> Option<PathBuf> {
        if self.expanded.remove(dir) {
            self.rebuild();
            return None;
        }
        self.expanded.insert(dir.to_path_buf());
        let need_load = !matches!(
            self.dirs.get(dir),
            Some(DirState::Loaded(_)) | Some(DirState::Loading(_))
        );
        if need_load {
            self.dirs.insert(dir.to_path_buf(), DirState::Loading(None));
        }
        self.rebuild();
        need_load.then(|| dir.to_path_buf())
    }

    /// 読み込み結果を反映する。
    pub fn set_children(&mut self, dir: &Path, result: io::Result<Vec<Entry>>) {
        let state = match result {
            Ok(entries) => DirState::Loaded(Arc::new(entries)),
            Err(e) => DirState::Error(e.to_string()),
        };
        self.dirs.insert(dir.to_path_buf(), state);
        self.rebuild();
    }

    /// 外部でフォルダの中身が変わった。読み込み済みなら読み直しが必要なので true を返す。
    pub fn invalidate(&mut self, dir: &Path) -> bool {
        match self.dirs.get(dir) {
            Some(DirState::Loaded(prev)) => {
                let prev = prev.clone();
                self.dirs.insert(dir.to_path_buf(), DirState::Loading(Some(prev)));
                true
            }
            _ => false,
        }
    }

    /// いま読み込み済みのフォルダ（ファイル監視の対象）。
    pub fn loaded_dirs(&self) -> impl Iterator<Item = &Path> {
        self.dirs.keys().map(PathBuf::as_path)
    }

    /// 読み込み済みのフォルダの中の 1 つ（なければ None。読み込んでいないフォルダのものも None）。
    pub fn entry(&self, path: &Path) -> Option<&Entry> {
        self.children(path.parent()?)?.iter().find(|e| e.path == path)
    }

    fn children(&self, dir: &Path) -> Option<&Arc<Vec<Entry>>> {
        match self.dirs.get(dir)? {
            DirState::Loaded(e) | DirState::Loading(Some(e)) => Some(e),
            _ => None,
        }
    }

    fn rebuild(&mut self) {
        let mut rows = Vec::new();
        let root_loading = matches!(self.dirs.get(&self.root), Some(DirState::Loading(_)));
        rows.push(Row {
            path: self.root.clone(),
            name: self.root_name(),
            depth: 0,
            kind: EntryKind::Dir,
            expanded: self.expanded.contains(&self.root),
            loading: root_loading,
        });
        if self.expanded.contains(&self.root) {
            self.push_children(&self.root.clone(), 1, &mut rows);
        }
        self.rows = Arc::new(rows);
    }

    fn push_children(&self, dir: &Path, depth: usize, rows: &mut Vec<Row>) {
        let Some(children) = self.children(dir) else { return };
        for e in children.iter() {
            let expanded = e.kind == EntryKind::Dir && self.expanded.contains(&e.path);
            rows.push(Row {
                path: e.path.clone(),
                name: e.name.clone(),
                depth,
                kind: e.kind,
                expanded,
                loading: matches!(self.dirs.get(&e.path), Some(DirState::Loading(None))),
            });
            if expanded {
                self.push_children(&e.path, depth + 1, rows);
            }
        }
    }
}

/// フォルダ、ファイルの順に、名前（大文字小文字を区別しない）で並べる。
pub fn sort_entries(entries: &mut [Entry]) {
    entries.sort_by(|a, b| match (a.kind, b.kind) {
        (EntryKind::Dir, EntryKind::File) => Ordering::Less,
        (EntryKind::File, EntryKind::Dir) => Ordering::Greater,
        _ => a
            .name
            .to_lowercase()
            .cmp(&b.name.to_lowercase())
            .then(a.name.cmp(&b.name)),
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_errors_are_kept_per_directory() {
        let root = PathBuf::from("/r");
        let (mut tree, _) = ProjectTree::new(root.clone());
        tree.set_children(&root, Err(io::Error::other("denied")));
        assert!(matches!(tree.dirs.get(&root), Some(DirState::Error(_))));
        assert_eq!(tree.rows().len(), 1);
    }
}
