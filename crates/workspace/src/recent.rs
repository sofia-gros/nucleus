//! 最近開いたフォルダ（Welcome とメニューの「最近のプロジェクト」）。`~/.local/share/nucleus/recent.json`。

use std::path::{Path, PathBuf};

use gpui_kit::{App, AppContext as _, Global};

/// 覚えておく数。
const MAX: usize = 10;

#[derive(Default)]
pub struct RecentProjects(pub Vec<PathBuf>);

impl Global for RecentProjects {}

fn path() -> Option<PathBuf> {
    dirs::data_dir().map(|d| d.join("nucleus").join("recent.json"))
}

/// 起動時に読む。もう無いフォルダは除く。
pub fn init(cx: &mut App) {
    let list: Vec<PathBuf> = path()
        .and_then(|p| std::fs::read(p).ok())
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_default();
    cx.set_global(RecentProjects(list.into_iter().filter(|p| p.is_dir()).collect()));
}

/// 開いたフォルダを先頭に足す（書き込みはバックグラウンド）。
pub fn add(folder: &Path, cx: &mut App) {
    let mut list = cx
        .try_global::<RecentProjects>()
        .map(|r| r.0.clone())
        .unwrap_or_default();
    list.retain(|p| p != folder);
    list.insert(0, folder.to_path_buf());
    list.truncate(MAX);
    let save = list.clone();
    cx.set_global(RecentProjects(list));
    cx.background_spawn(async move {
        let Some(path) = path() else { return };
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        if let Ok(json) = serde_json::to_vec_pretty(&save) {
            let tmp = path.with_extension("json.tmp");
            if std::fs::write(&tmp, json).is_ok() {
                let _ = std::fs::rename(tmp, path);
            }
        }
    })
    .detach();
}

/// 表示用: ホームを `~` にしたパス。
pub fn display(path: &Path) -> String {
    if let Some(home) = dirs::home_dir()
        && let Ok(rest) = path.strip_prefix(&home)
    {
        return format!("~/{}", rest.display());
    }
    path.display().to_string()
}
