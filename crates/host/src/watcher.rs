//! 読み込み済みのフォルダだけを（再帰せずに）監視する。node_modules などを丸ごと監視しないため。

use std::{
    collections::HashSet,
    path::{Path, PathBuf},
};

use futures::channel::mpsc::{UnboundedReceiver, UnboundedSender, unbounded};
use notify::{EventKind, RecommendedWatcher, RecursiveMode, Watcher as _};

pub struct DirWatcher {
    watcher: RecommendedWatcher,
    watched: HashSet<PathBuf>,
}

impl DirWatcher {
    /// 変更のあったフォルダ（変更されたパスの親）が受信側に届く。
    pub fn new() -> anyhow::Result<(DirWatcher, UnboundedReceiver<PathBuf>)> {
        let (tx, rx): (UnboundedSender<PathBuf>, _) = unbounded();
        let watcher = notify::recommended_watcher(move |res: notify::Result<notify::Event>| {
            let Ok(event) = res else { return };
            if matches!(event.kind, EventKind::Access(_)) {
                return;
            }
            for path in event.paths {
                if let Some(parent) = path.parent() {
                    let _ = tx.unbounded_send(parent.to_path_buf());
                }
            }
        })?;
        Ok((
            DirWatcher {
                watcher,
                watched: HashSet::new(),
            },
            rx,
        ))
    }

    pub fn watch(&mut self, dir: &Path) {
        if self.watched.insert(dir.to_path_buf())
            && let Err(e) = self.watcher.watch(dir, RecursiveMode::NonRecursive)
        {
            log::warn!("{} を監視できない: {e}", dir.display());
            self.watched.remove(dir);
        }
    }

    pub fn unwatch(&mut self, dir: &Path) {
        if self.watched.remove(dir) {
            let _ = self.watcher.unwatch(dir);
        }
    }
}
