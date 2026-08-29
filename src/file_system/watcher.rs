/// notify クレートを用いたバックグラウンドファイル監視モジュール

use anyhow::{Context, Result};
use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::time::Duration;

/// ファイル監視イベントの種類
#[derive(Debug, Clone)]
pub enum FileWatchEvent {
    Created(PathBuf),
    Modified(PathBuf),
    Removed(PathBuf),
    Renamed { from: PathBuf, to: PathBuf },
}

/// バックグラウンドファイルウォッチャー
pub struct FileWatcher {
    _watcher: RecommendedWatcher,
    pub event_rx: Receiver<FileWatchEvent>,
}

impl FileWatcher {
    /// 新規ウォッチャーを作成し、指定パスの再帰監視を開始
    pub fn watch(path: &Path) -> Result<Self> {
        let (tx, rx) = channel();
        let event_tx: Sender<FileWatchEvent> = tx;

        let mut watcher = RecommendedWatcher::new(
            move |res: notify::Result<Event>| {
                if let Ok(event) = res {
                    match event.kind {
                        EventKind::Create(_) => {
                            for path in event.paths {
                                let _ = event_tx.send(FileWatchEvent::Created(path));
                            }
                        }
                        EventKind::Modify(_) => {
                            for path in event.paths {
                                let _ = event_tx.send(FileWatchEvent::Modified(path));
                            }
                        }
                        EventKind::Remove(_) => {
                            for path in event.paths {
                                let _ = event_tx.send(FileWatchEvent::Removed(path));
                            }
                        }
                        _ => {}
                    }
                }
            },
            Config::default().with_poll_interval(Duration::from_millis(500)),
        ).context("Failed to initialize notify RecommendedWatcher")?;

        watcher.watch(path, RecursiveMode::Recursive)
            .context("Failed to watch directory path")?;

        Ok(Self {
            _watcher: watcher,
            event_rx: rx,
        })
    }
}
