//! 解決済みテーマのキャッシュ。起動時の最初のフレームを、拡張を待たずに描くために使う。

use std::path::PathBuf;

use crate::{ResolvedTheme, resolve::Snapshot};

pub fn path() -> Option<PathBuf> {
    dirs::cache_dir().map(|d| d.join("nucleus").join("theme.json"))
}

pub fn load() -> Option<ResolvedTheme> {
    let bytes = std::fs::read(path()?).ok()?;
    let snap: Snapshot = serde_json::from_slice(&bytes).ok()?;
    ResolvedTheme::from_snapshot(&snap)
}

/// バックグラウンドで呼ぶこと（ファイル I/O）。
pub fn save(theme: &ResolvedTheme) -> anyhow::Result<()> {
    let path = path().ok_or_else(|| anyhow::anyhow!("キャッシュの場所がない"))?;
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, serde_json::to_vec(&theme.to_snapshot())?)?;
    std::fs::rename(tmp, path)?;
    Ok(())
}
