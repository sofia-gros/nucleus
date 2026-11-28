//! プロジェクト全体の検索と置換（左ドックの Search）の、条件と結果と、1つのテキストの中の検索。
//!
//! ファイルを辿るのはホスト（host クレート）。

use std::{ops::Range, path::PathBuf};

use serde::{Deserialize, Serialize};

use regex::{Regex, RegexBuilder};

/// 表示する1行の長さの上限（文字）。
pub const MAX_LINE_CHARS: usize = 240;

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SearchOptions {
    pub query: String,
    pub regex: bool,
    pub case_sensitive: bool,
    pub whole_word: bool,
    /// 対象のファイル（`,` 区切りの glob。`!` で除外）。空ならすべて。
    pub include: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileMatches {
    pub path: PathBuf,
    /// ルートからの相対パス（`/` 区切り）。
    pub rel: String,
    pub matches: Vec<LineMatch>,
    /// このファイルの一致の数（1行に複数あれば数える）。
    pub count: usize,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
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

    fn opts(q: &str) -> SearchOptions {
        SearchOptions {
            query: q.into(),
            ..Default::default()
        }
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
