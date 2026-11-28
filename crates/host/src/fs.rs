//! ファイルとフォルダの操作。どれも呼び出したスレッドで（同期的に）行う。

use std::{
    fs,
    io::{self, Write as _},
    path::Path,
};

use project::{Entry, EntryKind, sort_entries};

/// フォルダの直下を読む（`.gitignore` などを反映し、`.git` は除く）。
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
            path: norm(item.path()),
            kind,
        });
    }
    sort_entries(&mut out);
    Ok(out)
}

/// 画面へ送るパスの形（Windows でも区切りを `/` にそろえる。remote::proto を参照）。
pub fn norm(path: &Path) -> std::path::PathBuf {
    std::path::PathBuf::from(remote::wire(path))
}

/// これより多いファイルは一覧に入れない（巨大なリポジトリでも応答性を保つ）。
pub const MAX_FILES: usize = 200_000;

/// プロジェクト内のファイル（`.gitignore` を反映した相対パス、`/` 区切り）。
pub fn list_files(root: &Path) -> Vec<String> {
    let mut out = Vec::new();
    let walker = ignore::WalkBuilder::new(root)
        .hidden(false)
        .git_ignore(true)
        .git_global(false)
        .require_git(false)
        .filter_entry(|e| e.file_name() != ".git")
        .build();
    for entry in walker.flatten() {
        if !entry.file_type().is_some_and(|t| t.is_file()) {
            continue;
        }
        if let Ok(rel) = entry.path().strip_prefix(root) {
            out.push(rel.to_string_lossy().replace('\\', "/"));
            if out.len() >= MAX_FILES {
                break;
            }
        }
    }
    out.sort();
    out
}

/// 大きさを確かめてから読む。大きすぎれば `Err(Ok(size))`。
pub fn read_limited(path: &Path, max_size: u64) -> Result<Vec<u8>, Result<u64, io::Error>> {
    let meta = fs::metadata(path).map_err(Err)?;
    if meta.len() > max_size {
        return Err(Ok(meta.len()));
    }
    fs::read(path).map_err(Err)
}

/// 一時ファイルに書いてから置き換える（途中で失敗しても元のファイルを壊さない。元の権限を保つ）。
pub fn write_atomic(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let dir = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let tmp = dir.join(format!(".{name}.nucleus-save"));
    {
        let mut out = fs::File::create(&tmp)?;
        out.write_all(bytes)?;
        out.sync_all()?;
    }
    if let Ok(meta) = fs::metadata(path) {
        let _ = fs::set_permissions(&tmp, meta.permissions());
    }
    fs::rename(&tmp, path).inspect_err(|_| {
        let _ = fs::remove_file(&tmp);
    })
}

/// 空のファイルを作る。すでにあれば `AlreadyExists`。
pub fn create_file(path: &Path) -> io::Result<()> {
    fs::OpenOptions::new().write(true).create_new(true).open(path).map(drop)
}

/// 名前の変更。移動先がすでにあれば `AlreadyExists`（黙って上書きしない）。
pub fn rename(from: &Path, to: &Path) -> io::Result<()> {
    if to.exists() && !same_file_ignoring_case(from, to) {
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            format!("{} already exists", to.display()),
        ));
    }
    fs::rename(from, to)
}

/// 大文字小文字だけを変える名前の変更（`a.rs` → `A.rs`）は、大文字小文字を区別しない OS でも許す。
fn same_file_ignoring_case(a: &Path, b: &Path) -> bool {
    a.to_string_lossy().to_lowercase() == b.to_string_lossy().to_lowercase()
}

pub fn trash(path: &Path) -> io::Result<()> {
    trash::delete(path).map_err(io::Error::other)
}

#[cfg(test)]
mod tests {
    use super::*;
    use project::ProjectTree;

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
    fn lists_files_relative_and_ignores_gitignored() {
        let d = fixture();
        let files = list_files(d.path());
        assert!(files.contains(&"src/nested/deep.rs".to_string()));
        assert!(!files.iter().any(|f| f.starts_with("target")));
        assert!(!files.iter().any(|f| f.ends_with(".log")));
    }

    #[test]
    fn writes_atomically_and_refuses_to_clobber() {
        let d = tempfile::tempdir().unwrap();
        let p = d.path().join("a.txt");
        fs::write(&p, "old").unwrap();
        write_atomic(&p, b"new").unwrap();
        assert_eq!(fs::read(&p).unwrap(), b"new");
        assert_eq!(fs::read_dir(d.path()).unwrap().count(), 1, "no temp file left behind");

        assert_eq!(create_file(&p).unwrap_err().kind(), io::ErrorKind::AlreadyExists);
        let q = d.path().join("b.txt");
        create_file(&q).unwrap();
        assert_eq!(rename(&q, &p).unwrap_err().kind(), io::ErrorKind::AlreadyExists);
        rename(&q, &d.path().join("c.txt")).unwrap();

        assert!(matches!(read_limited(&p, 2), Err(Ok(3))));
        assert_eq!(read_limited(&p, 3).unwrap(), b"new");
    }
}
