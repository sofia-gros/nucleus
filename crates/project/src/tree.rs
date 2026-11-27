use std::{
    cmp::Ordering,
    collections::{HashMap, HashSet},
    io,
    path::{Path, PathBuf},
    sync::Arc,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EntryKind {
    Dir,
    File,
}

#[derive(Clone, Debug, PartialEq, Eq)]
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

/// フォルダの直下を読む（`.gitignore` などを反映し、`.git` は除く）。バックグラウンドで呼ぶこと。
pub fn read_dir(dir: &Path) -> io::Result<Vec<Entry>> {
    let mut out = Vec::new();
    let walker = ignore::WalkBuilder::new(dir)
        .max_depth(Some(1))
        .hidden(false)
        .parents(true)
        .git_ignore(true)
        .git_global(false)
        .git_exclude(true)
        .require_git(false)
        .filter_entry(|e| e.file_name() != ".git")
        .build();
    for item in walker {
        let item = item.map_err(io::Error::other)?;
        if item.depth() == 0 {
            continue;
        }
        let kind = match item.file_type() {
            Some(t) if t.is_dir() => EntryKind::Dir,
            Some(t) if t.is_symlink() => {
                if item.path().is_dir() {
                    EntryKind::Dir
                } else {
                    EntryKind::File
                }
            }
            _ => EntryKind::File,
        };
        out.push(Entry {
            name: item.file_name().to_string_lossy().into_owned(),
            path: item.path().to_path_buf(),
            kind,
        });
    }
    out.sort_by(|a, b| match (a.kind, b.kind) {
        (EntryKind::Dir, EntryKind::File) => Ordering::Less,
        (EntryKind::File, EntryKind::Dir) => Ordering::Greater,
        _ => a
            .name
            .to_lowercase()
            .cmp(&b.name.to_lowercase())
            .then(a.name.cmp(&b.name)),
    });
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn fixture() -> tempfile::TempDir {
        let d = tempfile::tempdir().unwrap();
        let p = d.path();
        fs::create_dir_all(p.join("src/nested")).unwrap();
        fs::create_dir_all(p.join("target/debug")).unwrap();
        fs::create_dir_all(p.join(".git")).unwrap();
        fs::write(p.join(".gitignore"), "/target\n*.log\n").unwrap();
        fs::write(p.join("Cargo.toml"), "").unwrap();
        fs::write(p.join("b.rs"), "").unwrap();
        fs::write(p.join("A.md"), "").unwrap();
        fs::write(p.join("debug.log"), "").unwrap();
        fs::write(p.join("src/main.rs"), "").unwrap();
        fs::write(p.join("src/nested/deep.rs"), "").unwrap();
        d
    }

    fn names(entries: &[Entry]) -> Vec<&str> {
        entries.iter().map(|e| e.name.as_str()).collect()
    }

    #[test]
    fn reads_one_level_dirs_first_and_respects_gitignore() {
        let d = fixture();
        let entries = read_dir(d.path()).unwrap();
        assert_eq!(names(&entries), vec!["src", ".gitignore", "A.md", "b.rs", "Cargo.toml"]);
    }

    #[test]
    fn expands_lazily_and_flattens_rows() {
        let d = fixture();
        let root = d.path().to_path_buf();
        let (mut tree, to_load) = ProjectTree::new(root.clone());
        assert_eq!(to_load, root);
        assert_eq!(tree.rows().len(), 1);
        assert!(tree.rows()[0].loading);

        tree.set_children(&root, read_dir(&root));
        let rows = tree.rows();
        assert_eq!(rows.len(), 6);
        assert_eq!(
            (rows[1].name.as_str(), rows[1].depth, rows[1].expanded),
            ("src", 1, false)
        );

        let src = root.join("src");
        assert_eq!(tree.toggle(&src), Some(src.clone()));
        assert!(tree.rows()[1].expanded);
        assert!(tree.rows().len() == 6, "children are not shown until loaded");
        tree.set_children(&src, read_dir(&src));
        let rows = tree.rows();
        assert_eq!(
            rows.iter().map(|r| (r.name.as_str(), r.depth)).collect::<Vec<_>>()[1..4],
            [("src", 1), ("nested", 2), ("main.rs", 2)]
        );

        // 折りたたんで再度開いても読み直さない。
        assert_eq!(tree.toggle(&src), None);
        assert_eq!(tree.rows().len(), 6);
        assert_eq!(tree.toggle(&src), None);
        assert_eq!(tree.rows().len(), 8);
    }

    #[test]
    fn invalidation_keeps_old_children_visible_until_reloaded() {
        let d = fixture();
        let root = d.path().to_path_buf();
        let (mut tree, _) = ProjectTree::new(root.clone());
        tree.set_children(&root, read_dir(&root));
        fs::write(root.join("new.txt"), "").unwrap();

        assert!(tree.invalidate(&root));
        assert_eq!(tree.rows().len(), 6, "old entries stay while reloading");
        tree.set_children(&root, read_dir(&root));
        assert!(tree.rows().iter().any(|r| r.name == "new.txt"));
        assert!(!tree.invalidate(&root.join("never-loaded")));
    }

    #[test]
    fn read_errors_are_kept_per_directory() {
        let d = fixture();
        let root = d.path().to_path_buf();
        let (mut tree, _) = ProjectTree::new(root.clone());
        tree.set_children(&root, Err(io::Error::other("denied")));
        assert!(matches!(tree.dirs.get(&root), Some(DirState::Error(_))));
        assert_eq!(tree.rows().len(), 1);
    }
}
