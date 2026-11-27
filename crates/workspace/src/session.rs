//! セッション: 開いていたフォルダ・ファイル・ドックの状態を保存し、次回の起動で復元する。

use std::path::PathBuf;

use gpui_kit::{App, AppContext as _, Context, Window, component::dock::DockPlacement, px};
use serde::{Deserialize, Serialize};

use crate::Workspace;

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Session {
    pub root: Option<PathBuf>,
    pub open_files: Vec<PathBuf>,
    pub active: Option<PathBuf>,
    pub docks: Vec<DockSession>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DockSession {
    pub placement: String,
    pub open: bool,
    pub size: f32,
}

const DOCKS: [(DockPlacement, &str); 3] = [
    (DockPlacement::Left, "left"),
    (DockPlacement::Bottom, "bottom"),
    (DockPlacement::Right, "right"),
];

pub fn path() -> Option<PathBuf> {
    dirs::data_dir().map(|d| d.join("nucleus").join("session.json"))
}

/// 起動時に一度だけ読む（小さな JSON）。
pub fn load() -> Option<Session> {
    let bytes = std::fs::read(path()?).ok()?;
    serde_json::from_slice(&bytes).ok()
}

fn write(session: &Session) -> anyhow::Result<()> {
    let path = path().ok_or_else(|| anyhow::anyhow!("保存先がない"))?;
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, serde_json::to_vec_pretty(session)?)?;
    std::fs::rename(tmp, path)?;
    Ok(())
}

impl Workspace {
    pub(crate) fn session(&self, cx: &App) -> Session {
        let area = self.dock_area.read(cx);
        Session {
            root: self.project.read(cx).root(),
            open_files: self.open_order.clone(),
            active: self.active_editor(cx).map(|e| e.read(cx).path().to_path_buf()),
            docks: DOCKS
                .iter()
                .filter_map(|(p, name)| {
                    Some(DockSession {
                        placement: name.to_string(),
                        open: area.is_dock_open(*p),
                        size: f32::from(area.dock_size(*p)?),
                    })
                })
                .collect(),
        }
    }

    /// バックグラウンドで保存する（ファイルを開いた・閉じたときなど）。
    pub(crate) fn save_session(&mut self, cx: &mut Context<Self>) {
        let session = self.session(cx);
        if self.last_saved_session.as_ref() == Some(&session) {
            return;
        }
        self.last_saved_session = Some(session.clone());
        cx.background_spawn(async move {
            if let Err(e) = write(&session) {
                log::warn!("セッションを保存できなかった: {e}");
            }
        })
        .detach();
    }

    /// 終了時に保存する（プロセスが終わる前に書き終えるため同期で書く）。
    pub(crate) fn save_session_now(&mut self, cx: &mut Context<Self>) {
        let session = self.session(cx);
        if let Err(e) = write(&session) {
            log::warn!("セッションを保存できなかった: {e}");
        }
    }

    pub(crate) fn restore_session(&mut self, session: Session, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(root) = session.root.filter(|r| r.is_dir()) {
            self.open_path(root, window, cx);
        }
        for file in session.open_files.iter().filter(|f| f.is_file()) {
            self.open_file(file.clone(), window, cx);
        }
        // 最後に開くファイルがアクティブになるよう、アクティブなものを最後にもう一度開く。
        if let Some(active) = session.active.filter(|f| f.is_file()) {
            self.pending_active = Some(active);
        }
        self.dock_area.update(cx, |area, cx| {
            for d in &session.docks {
                let Some((p, _)) = DOCKS.iter().find(|(_, n)| *n == d.placement) else {
                    continue;
                };
                area.set_dock_size(*p, px(d.size.max(120.)), window, cx);
                if area.is_dock_open(*p) != d.open {
                    area.toggle_dock(*p, window, cx);
                }
            }
        });
    }
}
