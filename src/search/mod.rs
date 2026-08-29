/// プロジェクト全体テキスト検索および一括置換エンジンモジュール

use std::fs;
use std::path::{Path, PathBuf};
use crate::file_system::gitignore::GitIgnore;

/// 1行の中でのマッチ箇所
#[derive(Clone, Debug, PartialEq)]
pub struct SearchMatch {
    pub line_number: usize,
    pub line_text: String,
    pub match_start: usize,
    pub match_len: usize,
}

/// 1ファイルに対する検索結果
#[derive(Clone, Debug, PartialEq)]
pub struct SearchResult {
    pub file_path: PathBuf,
    pub relative_path: String,
    pub matches: Vec<SearchMatch>,
}

/// プロジェクト内の全テキストファイルを走査して検索
pub fn search_in_project(root: &Path, query: &str, case_sensitive: bool) -> Vec<SearchResult> {
    if query.is_empty() {
        return Vec::new();
    }

    let gitignore = if let Ok(content) = fs::read_to_string(root.join(".gitignore")) {
        GitIgnore::parse(&content)
    } else {
        GitIgnore::default()
    };

    let mut results = Vec::new();
    let mut stack = vec![root.to_path_buf()];

    let query_lower = query.to_lowercase();

    while let Some(dir) = stack.pop() {
        if let Ok(entries) = fs::read_dir(&dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if gitignore.is_ignored(&path) {
                    continue;
                }

                if path.is_dir() {
                    stack.push(path);
                } else if path.is_file() {
                    // テキストファイルの走査
                    if let Ok(content) = fs::read_to_string(&path) {
                        let mut matches = Vec::new();
                        for (idx, line) in content.lines().enumerate() {
                            let line_num = idx + 1;
                            let line_to_check = if case_sensitive { line.to_string() } else { line.to_lowercase() };
                            let q_to_check = if case_sensitive { query } else { &query_lower };

                            if let Some(pos) = line_to_check.find(q_to_check) {
                                matches.push(SearchMatch {
                                    line_number: line_num,
                                    line_text: line.to_string(),
                                    match_start: pos,
                                    match_len: query.len(),
                                });
                            }
                        }

                        if !matches.is_empty() {
                            let rel_path = path.strip_prefix(root).unwrap_or(&path).to_string_lossy().replace('\\', "/");
                            results.push(SearchResult {
                                file_path: path,
                                relative_path: rel_path,
                                matches,
                            });
                        }
                    }
                }
            }
        }
    }

    // 相対パス順にソート
    results.sort_by(|a, b| a.relative_path.cmp(&b.relative_path));
    results
}

/// ファイル内のテキストを一括置換
pub fn replace_in_file(path: &Path, query: &str, replacement: &str) -> anyhow::Result<usize> {
    let content = fs::read_to_string(path)?;
    let count = content.matches(query).count();
    if count > 0 {
        let new_content = content.replace(query, replacement);
        fs::write(path, new_content)?;
    }
    Ok(count)
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_search_matches() {
        let text = "hello world\nHELLO Rust\nanother line";
        let mut matches = Vec::new();
        for (idx, line) in text.lines().enumerate() {
            if line.to_lowercase().contains("hello") {
                matches.push(idx + 1);
            }
        }
        assert_eq!(matches, vec![1, 2]);
    }
}
