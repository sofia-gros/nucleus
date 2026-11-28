//! プロジェクト全体の検索と置換（左ドックの Search）。バックグラウンドで呼ぶ。
//!
//! `.gitignore` に従ってファイルを辿り、行ごとに正規表現で探す。大きなファイル（2MB 超）とバイナリは飛ばす。

use std::{
    ops::Range,
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, Ordering},
};

use regex::{Regex, RegexBuilder};

/// 1つのファイルの上限。
const MAX_FILE_SIZE: u64 = 2 << 20;
/// 表示する1行の長さの上限（文字）。
const MAX_LINE_CHARS: usize = 240;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SearchOptions {
    pub query: String,
    pub regex: bool,
    pub case_sensitive: bool,
    pub whole_word: bool,
    /// 対象のファイル（`,` 区切りの glob。`!` で除外）。空ならすべて。
    pub include: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LineMatch {
    /// 0 始まりの行番号。
    pub line: u32,
    /// 最初の一致の列（0 始まり、文字単位）。
    pub column: u32,
    /// 表示用の行（先頭の空白を除き、長ければ切り詰めたもの）。
    pub text: String,
    /// `text` の中の一致の位置（バイト）。
    pub ranges: Vec<Range<usize>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileMatches {
    pub path: PathBuf,
    /// ルートからの相対パス（`/` 区切り）。
    pub rel: String,
    pub matches: Vec<LineMatch>,
    /// このファイルの一致の数（1行に複数あれば数える）。
    pub count: usize,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SearchResult {
    pub files: Vec<FileMatches>,
    pub total: usize,
    /// 上限に達して途中で止めた。
    pub truncated: bool,
}

/// 検索条件から正規表現を作る。
pub fn build_regex(opts: &SearchOptions) -> Result<Regex, String> {
    let pattern = if opts.regex {
        opts.query.clone()
    } else {
        regex::escape(&opts.query)
    };
    let pattern = if opts.whole_word {
        format!(r"\b(?:{pattern})\b")
    } else {
        pattern
    };
    RegexBuilder::new(&pattern)
        .case_insensitive(!opts.case_sensitive)
        .multi_line(true)
        .size_limit(1 << 22)
        .build()
        .map_err(|e| e.to_string())
}

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
            path: path.clone(),
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

/// 1つのテキストの中の一致（行ごと）と、一致の総数。
pub fn search_text(text: &str, re: &Regex) -> Option<(Vec<LineMatch>, usize)> {
    let mut out = Vec::new();
    let mut count = 0;
    for (line_no, line) in text.lines().enumerate() {
        let ranges: Vec<Range<usize>> = re
            .find_iter(line)
            .filter(|m| !m.is_empty())
            .map(|m| m.range())
            .collect();
        if ranges.is_empty() {
            continue;
        }
        count += ranges.len();
        out.push(display_line(line, line_no as u32, &ranges));
    }
    (!out.is_empty()).then_some((out, count))
}

fn display_line(line: &str, line_no: u32, ranges: &[Range<usize>]) -> LineMatch {
    let column = line[..ranges[0].start].chars().count() as u32;
    // 先頭の空白を除く。一致が後ろにあって長い行は、一致の少し前から見せる。
    let trimmed_start = line.len() - line.trim_start().len();
    let mut start = trimmed_start;
    let lead = 12;
    if line[start..ranges[0].start].chars().count() > lead {
        start = line[..ranges[0].start]
            .char_indices()
            .rev()
            .nth(lead - 1)
            .map_or(start, |(i, _)| i);
    }
    let mut end = line.len();
    if line[start..].chars().count() > MAX_LINE_CHARS {
        end = line[start..]
            .char_indices()
            .nth(MAX_LINE_CHARS)
            .map_or(line.len(), |(i, _)| start + i);
    }
    let prefix = if start > trimmed_start { "…" } else { "" };
    let text = format!("{prefix}{}", &line[start..end]);
    let shift = prefix.len() as isize - start as isize;
    let ranges = ranges
        .iter()
        .filter(|r| r.start >= start && r.end <= end)
        .map(|r| (r.start as isize + shift) as usize..(r.end as isize + shift) as usize)
        .collect();
    LineMatch {
        line: line_no,
        column,
        text,
        ranges,
    }
}

/// テキストの中を置き換える。戻り値は（新しいテキスト, 置き換えた数）。正規表現なら `$1` などが使える。
pub fn replace_text(text: &str, re: &Regex, replacement: &str, regex_mode: bool) -> (String, usize) {
    let count = re.find_iter(text).filter(|m| !m.is_empty()).count();
    if count == 0 {
        return (text.to_string(), 0);
    }
    let out = if regex_mode {
        re.replace_all(text, replacement).into_owned()
    } else {
        re.replace_all(text, regex::NoExpand(replacement)).into_owned()
    };
    (out, count)
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

    #[test]
    fn whole_word_and_regex() {
        let ww = SearchOptions {
            whole_word: true,
            ..opts("dirty")
        };
        let re = build_regex(&ww).unwrap();
        assert_eq!(search_text("is_dirty dirty", &re).unwrap().1, 1);
        let rx = SearchOptions {
            regex: true,
            ..opts(r"fn (\w+)")
        };
        let re = build_regex(&rx).unwrap();
        let (out, n) = replace_text("fn a() fn b()", &re, "func $1", true);
        assert_eq!((out.as_str(), n), ("func a() func b()", 2));
        assert!(
            build_regex(&SearchOptions {
                regex: true,
                ..opts("(")
            })
            .is_err()
        );
        // 文字どおりの置き換えでは $ を展開しない
        let re = build_regex(&opts("a")).unwrap();
        assert_eq!(replace_text("a", &re, "$0", false).0, "$0");
    }

    #[test]
    fn long_lines_are_trimmed_around_the_match() {
        let re = build_regex(&opts("needle")).unwrap();
        let line = format!("{}needle{}", "x".repeat(100), "y".repeat(500));
        let (m, _) = search_text(&line, &re).unwrap();
        assert!(m[0].text.starts_with('…'));
        assert_eq!(&m[0].text[m[0].ranges[0].clone()], "needle");
        assert!(m[0].text.chars().count() <= MAX_LINE_CHARS + 1);
    }
}
