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
    fn picks_languages() {
        assert_eq!(language_for(Path::new("src/main.rs")), "rust");
        assert_eq!(language_for(Path::new("Cargo.lock")), "toml");
        assert_eq!(language_for(Path::new("a.TSX")), "tsx");
        assert_eq!(language_for(Path::new("LICENSE")), "text");
    }
}
