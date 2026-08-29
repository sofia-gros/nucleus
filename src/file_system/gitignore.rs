/// .gitignore パターン解析およびファイル除外判定モジュール

use std::path::Path;

/// .gitignore ルール保持構造体
#[derive(Clone, Debug, Default)]
pub struct GitIgnore {
    patterns: Vec<String>,
}

impl GitIgnore {
    /// .gitignore ファイルの内容からルールを構築
    pub fn parse(content: &str) -> Self {
        let mut patterns = Vec::new();
        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }
            patterns.push(trimmed.to_string());
        }
        Self { patterns }
    }

    /// 指定されたパスが除外対象かどうかを判定
    pub fn is_ignored(&self, path: &Path) -> bool {
        let path_str = path.to_string_lossy().replace('\\', "/");
        let file_name = path.file_name().map(|n| n.to_string_lossy()).unwrap_or_default();

        // 常に無視する既定ディレクトリ
        if file_name == ".git" || file_name == "target" || file_name == "node_modules" || file_name == ".nucleus" {
            return true;
        }

        for pattern in &self.patterns {
            let pat = pattern.trim_end_matches('/');
            if pat.starts_with('*') {
                // 拡張子マッチング (*.log, *.o など)
                let ext = &pat[1..];
                if path_str.ends_with(ext) {
                    return true;
                }
            } else if path_str == *pat || path_str.ends_with(&format!("/{}", pat)) || file_name == *pat {
                return true;
            }
        }

        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_gitignore_basic() {
        let ignore = GitIgnore::parse("*.log\ntarget/\nbuild\n");
        assert!(ignore.is_ignored(&PathBuf::from("test.log")));
        assert!(ignore.is_ignored(&PathBuf::from("target")));
        assert!(ignore.is_ignored(&PathBuf::from("src/test.log")));
        assert!(!ignore.is_ignored(&PathBuf::from("src/main.rs")));
    }
}
