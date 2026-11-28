//! ファイルの中身とテキストの変換。読み書きそのものはホストが行う（remote::Client）。

use std::{io, path::Path};

/// これより大きいファイルは開かない（エディタの応答性を守るため）。
pub const MAX_FILE_SIZE: u64 = 32 * 1024 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LineEnding {
    Lf,
    Crlf,
}

impl LineEnding {
    pub fn label(self) -> &'static str {
        match self {
            LineEnding::Lf => "LF",
            LineEnding::Crlf => "CRLF",
        }
    }
}

#[derive(Debug)]
pub enum LoadError {
    TooLarge(u64),
    Binary,
    Io(io::Error),
}

impl std::fmt::Display for LoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LoadError::TooLarge(n) => write!(f, "file is too large ({n} bytes)"),
            LoadError::Binary => write!(f, "binary file"),
            LoadError::Io(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for LoadError {}

/// 読み込んだ内容。改行は `\n` にそろえてある。
#[derive(Debug, PartialEq)]
pub struct Loaded {
    pub text: String,
    pub line_ending: LineEnding,
}

/// ホストから読み込んでテキストにする。
pub async fn load(client: remote::Client, path: std::path::PathBuf) -> Result<Loaded, LoadError> {
    let bytes = client.read_file(&path, MAX_FILE_SIZE).await.map_err(|e| match e.kind {
        remote::ErrorKind::TooLarge(n) => LoadError::TooLarge(n),
        kind => {
            let io_kind = match kind {
                remote::ErrorKind::NotFound => io::ErrorKind::NotFound,
                remote::ErrorKind::PermissionDenied => io::ErrorKind::PermissionDenied,
                _ => io::ErrorKind::Other,
            };
            LoadError::Io(io::Error::new(io_kind, e.message))
        }
    })?;
    decode(bytes)
}

pub fn decode(bytes: Vec<u8>) -> Result<Loaded, LoadError> {
    if bytes[..bytes.len().min(8192)].contains(&0) {
        return Err(LoadError::Binary);
    }
    let mut text = String::from_utf8(bytes).map_err(|_| LoadError::Binary)?;
    if text.starts_with('\u{feff}') {
        text.remove(0);
    }
    // 最初の改行で判定する（混在していても保存時はこの形式にそろえる）。
    let line_ending = match text.find('\n') {
        Some(i) if i > 0 && text.as_bytes()[i - 1] == b'\r' => LineEnding::Crlf,
        _ => LineEnding::Lf,
    };
    if text.contains('\r') {
        text = text.replace("\r\n", "\n");
    }
    Ok(Loaded { text, line_ending })
}

/// 保存するバイト列にする（改行を `line_ending` にそろえる）。書き出しはホストが行う。
pub fn encode<'a>(chunks: impl Iterator<Item = &'a str>, line_ending: LineEnding) -> Vec<u8> {
    let mut out = Vec::new();
    for chunk in chunks {
        match line_ending {
            LineEnding::Lf => out.extend_from_slice(chunk.as_bytes()),
            LineEnding::Crlf => {
                let mut rest = chunk;
                while let Some(i) = rest.find('\n') {
                    out.extend_from_slice(&rest.as_bytes()[..i]);
                    out.extend_from_slice(b"\r\n");
                    rest = &rest[i + 1..];
                }
                out.extend_from_slice(rest.as_bytes());
            }
        }
    }
    out
}

/// 保存するときの手直し（`files.trim_trailing_whitespace` など）。
#[derive(Clone, Copy, Debug, Default)]
pub struct SaveFixes {
    pub trim_trailing_whitespace: bool,
    pub insert_final_newline: bool,
    pub trim_final_newlines: bool,
}

/// 手直しのための置き換え（バイトの範囲と新しい文字列）。後ろから順に並ぶ（そのまま順に当てられる）。
pub fn save_fixes(text: &str, fixes: SaveFixes) -> Vec<(std::ops::Range<usize>, String)> {
    let mut edits = Vec::new();
    // 末尾の改行の始まり（`trim_final_newlines` で消す範囲の先頭）。
    let body_end = text.trim_end_matches('\n').len();
    if fixes.trim_final_newlines && text.len() > body_end + 1 {
        edits.push((body_end + 1..text.len(), String::new()));
    }
    if fixes.insert_final_newline && !text.is_empty() && !text.ends_with('\n') {
        edits.push((text.len()..text.len(), "\n".into()));
    }
    if fixes.trim_trailing_whitespace {
        let mut start = 0;
        let mut lines = Vec::new();
        for line in text.split_inclusive('\n') {
            let content = line.strip_suffix('\n').unwrap_or(line);
            let kept = content.trim_end_matches([' ', '\t']).len();
            if kept < content.len() {
                lines.push((start + kept..start + content.len(), String::new()));
            }
            start += line.len();
        }
        edits.extend(lines.into_iter().rev());
    }
    // 重ならないように、後ろから並べる。
    edits.sort_by_key(|e| std::cmp::Reverse(e.0.start));
    edits
}

/// 中身からインデントを推し量る（`editor.detect_indentation`）。分からなければ None。
/// 戻り値は（タブ幅, タブ文字を使うか）。
pub fn detect_indentation(text: &str) -> Option<(usize, bool)> {
    let (mut tabs, mut spaces) = (0usize, 0usize);
    // 隣り合う行の、字下げの差の数（2 か 4 か 8 か）。
    let mut deltas = [0usize; 9];
    let mut prev = 0usize;
    for line in text.lines().take(10_000) {
        if line.trim().is_empty() {
            continue;
        }
        if line.starts_with('\t') {
            tabs += 1;
            continue;
        }
        let n = line.len() - line.trim_start_matches(' ').len();
        if n > 0 {
            spaces += 1;
        }
        let d = n.abs_diff(prev);
        if (2..=8).contains(&d) {
            deltas[d] += 1;
        }
        prev = n;
    }
    if tabs == 0 && spaces == 0 {
        return None;
    }
    if tabs > spaces {
        return Some((4, true));
    }
    let (width, _) = [2usize, 4, 8, 3]
        .iter()
        .map(|&w| (w, deltas[w]))
        .max_by_key(|&(_, n)| n)?;
    (deltas[width] > 0).then_some((width, false))
}

/// ファイル名から gpui-kit の言語名を決める（tree-sitter の文法の名前）。
pub fn language_for(path: &Path) -> String {
    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("").to_lowercase();
    match name.as_str() {
        "cargo.lock" => return "toml".into(),
        "makefile" | "gnumakefile" => return "make".into(),
        "cmakelists.txt" => return "cmake".into(),
        "dockerfile" => return "bash".into(),
        _ => {}
    }
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
    match ext.as_str() {
        "rs" => "rust",
        "toml" => "toml",
        "json" | "jsonc" | "json5" => "json",
        "md" | "markdown" | "mdx" => "markdown",
        "yml" | "yaml" => "yaml",
        "js" | "mjs" | "cjs" => "javascript",
        "jsx" => "javascript",
        "ts" | "mts" | "cts" => "typescript",
        "tsx" => "tsx",
        "py" | "pyi" => "python",
        "sh" | "bash" | "zsh" => "bash",
        "go" => "go",
        "c" | "h" => "c",
        "cc" | "cpp" | "cxx" | "hpp" | "hh" => "cpp",
        "css" | "scss" => "css",
        "html" | "htm" => "html",
        "" => "text",
        other => return other.to_string(),
    }
    .into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_line_endings_and_bom() {
        let l = decode(b"\xef\xbb\xbfa\r\nb\r\n".to_vec()).unwrap();
        assert_eq!(
            l,
            Loaded {
                text: "a\nb\n".into(),
                line_ending: LineEnding::Crlf
            }
        );
        assert_eq!(decode(b"a\nb".to_vec()).unwrap().line_ending, LineEnding::Lf);
        assert_eq!(decode(Vec::new()).unwrap().text, "");
    }

    #[test]
    fn rejects_binary() {
        assert!(matches!(
            decode(vec![0x7f, b'E', b'L', b'F', 0, 1]),
            Err(LoadError::Binary)
        ));
        assert!(matches!(decode(vec![0xff, 0xfe, b'a']), Err(LoadError::Binary)));
    }

    #[test]
    fn encodes_with_the_original_line_ending() {
        assert_eq!(
            encode(["one\ntw", "o\n"].into_iter(), LineEnding::Crlf),
            b"one\r\ntwo\r\n"
        );
        assert_eq!(encode(["x\ny"].into_iter(), LineEnding::Lf), b"x\ny");
        let l = decode(b"fn main() {}\r\n".to_vec()).unwrap();
        assert_eq!(
            encode([l.text.as_str()].into_iter(), l.line_ending),
            b"fn main() {}\r\n"
        );
    }

    #[test]
    fn save_fixes_trim_and_add_newlines() {
        let all = SaveFixes {
            trim_trailing_whitespace: true,
            insert_final_newline: true,
            trim_final_newlines: true,
        };
        let apply = |text: &str, f| {
            let mut t = text.to_string();
            for (r, s) in save_fixes(text, f) {
                t.replace_range(r, &s);
            }
            t
        };
        assert_eq!(apply("a  \nb\t\nc", all), "a\nb\nc\n");
        assert_eq!(apply("a\n\n\n", all), "a\n");
        assert_eq!(apply("a  ", SaveFixes::default()), "a  ");
        assert_eq!(apply("", all), "");
        assert!(save_fixes("clean\n", all).is_empty());
    }

    #[test]
    fn detects_indentation() {
        assert_eq!(
            detect_indentation("fn a() {\n  b();\n  if x {\n    c();\n  }\n}\n"),
            Some((2, false))
        );
        assert_eq!(detect_indentation("a:\n    b\n    c:\n        d\n"), Some((4, false)));
        assert_eq!(detect_indentation("a {\n\tb\n\tc\n}\n"), Some((4, true)));
        assert_eq!(detect_indentation("plain\ntext\n"), None);
    }

    #[test]
    fn picks_languages() {
        assert_eq!(language_for(Path::new("src/main.rs")), "rust");
        assert_eq!(language_for(Path::new("Cargo.lock")), "toml");
        assert_eq!(language_for(Path::new("a.TSX")), "tsx");
        assert_eq!(language_for(Path::new("LICENSE")), "text");
    }
}
