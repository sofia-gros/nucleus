//! プロジェクト全体の検索。`.gitignore` に従ってファイルを辿り、行ごとに正規表現で探す。
//! 大きなファイル（2MB 超）とバイナリは飛ばす。

use std::{
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, Ordering},
};

use project::search::{FileMatches, SearchOptions, SearchResult, build_regex, search_text};

/// 1つのファイルの上限。
const MAX_FILE_SIZE: u64 = 2 << 20;

/// `root` の下を探す。`cancel` が立ったら途中で止める。一致が `limit` に達したら止める。
pub fn search(root: &Path, opts: &SearchOptions, limit: usize, cancel: &AtomicBool) -> Result<SearchResult, String> {
    let mut result = SearchResult::default();
    if opts.query.is_empty() {
        return Ok(result);
    }
    let re = build_regex(opts)?;
    let mut walker = ignore::WalkBuilder::new(root);
    walker.hidden(true).git_ignore(true).parents(true).require_git(false);
    let globs: Vec<&str> = opts
        .include
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .collect();
    if !globs.is_empty() {
        let mut ov = ignore::overrides::OverrideBuilder::new(root);
        for g in &globs {
            ov.add(g).map_err(|e| format!("`{g}`: {e}"))?;
        }
        walker.overrides(ov.build().map_err(|e| e.to_string())?);
    }
    let mut paths: Vec<PathBuf> = walker
        .build()
        .flatten()
        .filter(|e| e.file_type().is_some_and(|t| t.is_file()))
        .filter(|e| e.metadata().is_ok_and(|m| m.len() <= MAX_FILE_SIZE))
        .map(|e| e.into_path())
        .collect();
    paths.sort();
    for path in paths {
        if cancel.load(Ordering::Relaxed) {
            return Err("cancelled".into());
        }
        let Ok(bytes) = std::fs::read(&path) else { continue };
        if bytes[..bytes.len().min(8192)].contains(&0) {
            continue;
        }
        let text = String::from_utf8_lossy(&bytes);
        let Some(file) = search_text(&text, &re) else { continue };
        let rel = path
            .strip_prefix(root)
            .unwrap_or(&path)
            .to_string_lossy()
            .replace('\\', "/");
        result.total += file.1;
        result.files.push(FileMatches {
            path: crate::fs::norm(&path),
            rel,
            matches: file.0,
            count: file.1,
        });
        if result.total >= limit {
            result.truncated = true;
            break;
        }
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn opts(q: &str) -> SearchOptions {
        SearchOptions {
            query: q.into(),
            ..Default::default()
        }
    }

    #[test]
    fn finds_matches_respecting_gitignore_and_globs() {
        let d = tempfile::tempdir().unwrap();
        let r = d.path();
        fs::create_dir_all(r.join("src")).unwrap();
        fs::create_dir_all(r.join("target")).unwrap();
        fs::write(r.join(".gitignore"), "target/\n").unwrap();
        fs::write(
            r.join("src/a.rs"),
            "fn is_dirty() {}\n    if x.is_dirty() && y.IS_DIRTY() {}\n",
        )
        .unwrap();
        fs::write(r.join("b.txt"), "is_dirty\n").unwrap();
        fs::write(r.join("target/c.rs"), "is_dirty\n").unwrap();
        fs::write(r.join("bin.dat"), b"is_dirty\0\0").unwrap();
        let cancel = AtomicBool::new(false);

        let res = search(r, &opts("is_dirty"), 1000, &cancel).unwrap();
        let files: Vec<&str> = res.files.iter().map(|f| f.rel.as_str()).collect();
        assert_eq!(files, vec!["b.txt", "src/a.rs"]);
        assert_eq!(res.total, 4);
        let a = &res.files[1];
        assert_eq!(a.matches[1].line, 1);
        assert_eq!(a.matches[1].column, 9);
        assert_eq!(a.matches[1].text, "if x.is_dirty() && y.IS_DIRTY() {}");
        assert_eq!(a.matches[1].ranges, vec![5..13, 21..29]);

        let case = SearchOptions {
            case_sensitive: true,
            include: "src/**/*.rs".into(),
            ..opts("is_dirty")
        };
        let res = search(r, &case, 1000, &cancel).unwrap();
        assert_eq!(res.total, 2);
        assert_eq!(res.files.len(), 1);
    }
}
