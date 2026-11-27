//! ファイルの読み書き。どちらもバックグラウンドで呼ぶこと。

use std::{
    fs,
    io::{self, Write as _},
    path::Path,
};

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

pub fn load(path: &Path) -> Result<Loaded, LoadError> {
    let meta = fs::metadata(path).map_err(LoadError::Io)?;
    if meta.len() > MAX_FILE_SIZE {
        return Err(LoadError::TooLarge(meta.len()));
    }
    let bytes = fs::read(path).map_err(LoadError::Io)?;
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

/// 一時ファイルに書いてから置き換える（途中で失敗しても元のファイルを壊さない）。
pub fn save<'a>(path: &Path, chunks: impl Iterator<Item = &'a str>, line_ending: LineEnding) -> io::Result<()> {
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
        let mut out = io::BufWriter::new(fs::File::create(&tmp)?);
        for chunk in chunks {
            match line_ending {
                LineEnding::Lf => out.write_all(chunk.as_bytes())?,
                LineEnding::Crlf => {
                    let mut rest = chunk;
                    while let Some(i) = rest.find('\n') {
                        out.write_all(&rest.as_bytes()[..i])?;
                        out.write_all(b"\r\n")?;
                        rest = &rest[i + 1..];
                    }
                    out.write_all(rest.as_bytes())?;
                }
            }
        }
        let file = out.into_inner().map_err(|e| e.into_error())?;
        file.sync_all()?;
    }
    if let Ok(meta) = fs::metadata(path) {
        let _ = fs::set_permissions(&tmp, meta.permissions());
    }
    fs::rename(&tmp, path).inspect_err(|_| {
        let _ = fs::remove_file(&tmp);
    })
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
    fn saves_atomically_with_the_original_line_ending() {
        let d = tempfile::tempdir().unwrap();
        let p = d.path().join("a.txt");
        fs::write(&p, "old").unwrap();
        save(&p, ["one\ntw", "o\n"].into_iter(), LineEnding::Crlf).unwrap();
        assert_eq!(fs::read(&p).unwrap(), b"one\r\ntwo\r\n");
        save(&p, ["x\ny"].into_iter(), LineEnding::Lf).unwrap();
        assert_eq!(fs::read_to_string(&p).unwrap(), "x\ny");
        assert_eq!(fs::read_dir(d.path()).unwrap().count(), 1, "no temp file left behind");
    }

    #[test]
    fn round_trips_through_load() {
        let d = tempfile::tempdir().unwrap();
        let p = d.path().join("w.rs");
        fs::write(&p, "fn main() {}\r\n").unwrap();
        let l = load(&p).unwrap();
        save(&p, [l.text.as_str()].into_iter(), l.line_ending).unwrap();
        assert_eq!(fs::read(&p).unwrap(), b"fn main() {}\r\n");
    }

    #[test]
    fn picks_languages() {
        assert_eq!(language_for(Path::new("src/main.rs")), "rust");
        assert_eq!(language_for(Path::new("Cargo.lock")), "toml");
        assert_eq!(language_for(Path::new("a.TSX")), "tsx");
        assert_eq!(language_for(Path::new("LICENSE")), "text");
    }
}
