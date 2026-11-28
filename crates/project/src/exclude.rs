//! 出さないファイル（`files.exclude`、`search.exclude`）。VSCode と同じく glob で書く。
//!
//! パターンはプロジェクトのルートからの相対パス（`/` 区切り）に当てる。フォルダに当たれば、その中もすべて除く。

use globset::{Glob, GlobSet, GlobSetBuilder};

#[derive(Clone, Debug, Default)]
pub struct Excludes {
    set: Option<GlobSet>,
}

impl Excludes {
    /// 読めないパターンは無視する（ログに出す）。
    pub fn new<'a>(patterns: impl IntoIterator<Item = &'a String>) -> Excludes {
        let mut b = GlobSetBuilder::new();
        let mut any = false;
        for p in patterns {
            let p = p.trim().trim_start_matches("./");
            if p.is_empty() {
                continue;
            }
            match Glob::new(p) {
                Ok(g) => {
                    b.add(g);
                    any = true;
                }
                Err(e) => log::warn!("除外のパターン `{p}` が読めない: {e}"),
            }
        }
        Excludes {
            set: any.then(|| b.build().ok()).flatten(),
        }
    }

    /// `rel`（ルートからの相対パス、`/` 区切り）を除くか。途中のフォルダが当たっても除く。
    pub fn is_excluded(&self, rel: &str) -> bool {
        let Some(set) = &self.set else { return false };
        let mut end = 0;
        for part in rel.split('/') {
            end += part.len();
            if set.is_match(&rel[..end]) {
                return true;
            }
            end += 1;
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_files_and_everything_inside_folders() {
        let pats: Vec<String> = ["**/node_modules", "**/*.log", "target", "**/.DS_Store"]
            .map(String::from)
            .to_vec();
        let e = Excludes::new(&pats);
        assert!(e.is_excluded("node_modules"));
        assert!(e.is_excluded("web/node_modules/react/index.js"));
        assert!(e.is_excluded("a/b/debug.log"));
        assert!(e.is_excluded("target/debug/x"));
        assert!(!e.is_excluded("src/target.rs"));
        assert!(!e.is_excluded("src/main.rs"));
        assert!(!Excludes::default().is_excluded("x"));
    }
}
