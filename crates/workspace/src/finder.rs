//! ファイル検索（Ctrl+P）のための一覧とあいまい検索。どちらもバックグラウンドで呼ぶ。

use std::path::Path;

use nucleo_matcher::{
    Config, Matcher, Utf32Str,
    pattern::{CaseMatching, Normalization, Pattern},
};

/// これより多いファイルは一覧に入れない（巨大なリポジトリでも応答性を保つ）。
pub const MAX_FILES: usize = 200_000;

/// プロジェクト内のファイル（`.gitignore` を反映した相対パス）。
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
            out.push(rel.to_string_lossy().into_owned());
            if out.len() >= MAX_FILES {
                break;
            }
        }
    }
    out.sort();
    out
}

/// `query` に合うものを、点数の高い順に最大 `limit` 件返す（`files` の添字）。
/// 空のクエリなら先頭から `limit` 件。
pub fn fuzzy_top(files: &[String], query: &str, limit: usize) -> Vec<usize> {
    let query = query.trim();
    if query.is_empty() {
        return (0..files.len().min(limit)).collect();
    }
    let pattern = Pattern::parse(query, CaseMatching::Smart, Normalization::Smart);
    let mut matcher = Matcher::new(Config::DEFAULT.match_paths());
    let mut buf = Vec::new();
    let mut scored: Vec<(u32, usize)> = files
        .iter()
        .enumerate()
        .filter_map(|(i, f)| {
            let score = pattern.score(Utf32Str::new(f, &mut buf), &mut matcher)?;
            // ファイル名に一致した方を優先する。
            let name = f.rsplit('/').next().unwrap_or(f);
            let bonus = pattern
                .score(Utf32Str::new(name, &mut buf), &mut matcher)
                .map_or(0, |s| s / 2);
            Some((score + bonus, i))
        })
        .collect();
    scored.sort_by(|a, b| {
        b.0.cmp(&a.0)
            .then(files[a.1].len().cmp(&files[b.1].len()))
            .then(a.1.cmp(&b.1))
    });
    scored.into_iter().take(limit).map(|(_, i)| i).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lists_files_respecting_gitignore() {
        let d = tempfile::tempdir().unwrap();
        let p = d.path();
        std::fs::create_dir_all(p.join("src/a")).unwrap();
        std::fs::create_dir_all(p.join("target")).unwrap();
        std::fs::write(p.join(".gitignore"), "/target\n").unwrap();
        std::fs::write(p.join("src/a/display_map.rs"), "").unwrap();
        std::fs::write(p.join("src/main.rs"), "").unwrap();
        std::fs::write(p.join("target/out"), "").unwrap();
        assert_eq!(list_files(p), vec![".gitignore", "src/a/display_map.rs", "src/main.rs"]);
    }

    #[test]
    fn ranks_file_name_matches_first() {
        let files: Vec<String> = [
            "crates/editor/src/display_map.rs",
            "crates/dispatch/src/lib.rs",
            "docs/map/display.md",
            "crates/editor/tests/display_map_tests.rs",
        ]
        .map(String::from)
        .to_vec();
        let top = fuzzy_top(&files, "dismap", 10);
        assert_eq!(files[top[0]], "crates/editor/src/display_map.rs");
        assert!(top.contains(&3));
        assert!(!fuzzy_top(&files, "zzzz", 10).iter().any(|_| true));
        assert_eq!(fuzzy_top(&files, "", 2), vec![0, 1]);
    }
}
