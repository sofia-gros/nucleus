//! ファイル検索（Ctrl+P）のあいまい検索。一覧はホストが作る（remote::Client::list_files）。

use nucleo_matcher::{
    Config, Matcher, Utf32Str,
    pattern::{CaseMatching, Normalization, Pattern},
};

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
