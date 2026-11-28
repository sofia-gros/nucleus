//! Diff のタブ（Extension の `show-diff` で開く）と Workspace のつなぎ。

use std::path::{Path, PathBuf};

use editor::diff_view::{DiffView, DiffViewEvent};
use extension_host::{WasmHost, ui::UiEvent};
use gpui_kit::{
    AppContext as _, Context, Entity, WeakEntity, Window,
    component::{
        WindowExt as _,
        dock::{DockPlacement, PanelId, panel_handle},
        notification::Notification,
    },
};

use remote::RemoteExt as _;

use crate::Workspace;

/// 開いている Diff（ファイル → ビューと、開いた Extension）。
pub(crate) type DiffViews = Vec<(WeakEntity<DiffView>, String)>;

impl Workspace {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn show_diff(
        &mut self,
        extension: String,
        path: PathBuf,
        rel: String,
        base: String,
        base_label: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.diff_views.retain(|(v, _)| v.upgrade().is_some());
        if let Some(view) = self.diff_view_for(&path, cx) {
            view.update(cx, |v, cx| v.set_base(base_label, base, cx));
            let id = PanelId::from(view.entity_id());
            self.dock_area.update(cx, |a, cx| a.select_panel(id, window, cx));
            return;
        }
        // 開いていなければ、ホストから読んでから開く。
        let current = match self.open_text(&path, cx) {
            Some(t) => t,
            None => {
                let read = editor::file::load(cx.remote(), path.clone());
                cx.spawn_in(window, async move |this, cx| {
                    let current = read.await.map(|l| l.text).unwrap_or_default();
                    this.update_in(cx, |this, window, cx| {
                        this.open_diff_view(extension, path, rel, base, base_label, current, window, cx)
                    })
                    .ok();
                })
                .detach();
                return;
            }
        };
        self.open_diff_view(extension, path, rel, base, base_label, current, window, cx);
    }

    #[allow(clippy::too_many_arguments)]
    fn open_diff_view(
        &mut self,
        extension: String,
        path: PathBuf,
        rel: String,
        base: String,
        base_label: String,
        current: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let language = editor::language_for(&path, cx);
        let view = cx.new(|cx| DiffView::new(path, rel, base_label, base, current, language, true, cx));
        self._subscriptions
            .push(cx.subscribe_in(&view, window, |this, _, e, window, cx| {
                this.on_diff_event(e, window, cx)
            }));
        self.diff_views.push((view.downgrade(), extension));
        self.dock_area.update(cx, |a, cx| {
            a.add_panel_view(panel_handle(view), DockPlacement::Center, None, window, cx)
        });
    }

    fn diff_view_for(&self, path: &Path, cx: &gpui_kit::App) -> Option<Entity<DiffView>> {
        self.diff_views
            .iter()
            .filter_map(|(v, _)| v.upgrade())
            .find(|v| v.read(cx).path() == path)
    }

    /// エディタで開いていれば、その中身。
    fn open_text(&self, path: &Path, cx: &gpui_kit::App) -> Option<String> {
        let e = self.editors.get(path).and_then(WeakEntity::upgrade)?;
        Some(e.read(cx).state().read(cx).value().to_string())
    }

    /// エディタの中身が変わったら、開いている Diff を新しくする（開いていなければホストから読み直す）。
    pub(crate) fn refresh_diff(&mut self, path: &Path, cx: &mut Context<Self>) {
        let Some(view) = self.diff_view_for(path, cx) else {
            return;
        };
        if let Some(text) = self.open_text(path, cx) {
            view.update(cx, |v, cx| v.set_current(text, cx));
            return;
        }
        let read = editor::file::load(cx.remote(), path.to_path_buf());
        cx.spawn(async move |_, cx| {
            if let Ok(l) = read.await {
                view.update(cx, |v, cx| v.set_current(l.text, cx));
            }
        })
        .detach();
    }

    fn on_diff_event(&mut self, e: &DiffViewEvent, window: &mut Window, cx: &mut Context<Self>) {
        match e {
            DiffViewEvent::Open { path, line } => {
                let pos = lsp_types::Position::new(*line, 0);
                self.open_file_at(path.clone(), Some(pos), window, cx);
            }
            DiffViewEvent::Revert { path, text } => {
                if let Some(editor) = self.editors.get(path).and_then(WeakEntity::upgrade) {
                    // 開いているファイルはエディタの中で戻す（元に戻せる。保存は利用者が行う）。
                    let state = editor.read(cx).state().clone();
                    let text = text.clone();
                    state.update(cx, |s, cx| s.replace_all(text, window, cx));
                    self.refresh_diff(path, cx);
                } else {
                    // 開いていないファイルはホストで書き換える（改行の形は元のまま）。
                    let client = cx.remote();
                    let (path, text) = (path.clone(), text.clone());
                    cx.spawn_in(window, async move |this, cx| {
                        let result = async {
                            let le = match editor::file::load(client.clone(), path.clone()).await {
                                Ok(l) => l.line_ending,
                                Err(_) => editor::LineEnding::Lf,
                            };
                            let bytes = editor::file::encode(std::iter::once(text.as_str()), le);
                            client.write_file(&path, bytes).await
                        }
                        .await;
                        this.update_in(cx, |this, window, cx| match result {
                            Ok(()) => {
                                this.extension_file_saved(&path, cx);
                                this.refresh_diff(&path, cx);
                            }
                            Err(e) => {
                                window.push_notification(Notification::error(format!("{}: {e}", path.display())), cx)
                            }
                        })
                        .ok();
                    })
                    .detach();
                }
            }
            DiffViewEvent::StageHunk { path, patch } => {
                let Some(ext) = self
                    .diff_views
                    .iter()
                    .find(|(v, _)| v.upgrade().is_some_and(|v| v.read(cx).path() == path))
                    .map(|(_, e)| e.clone())
                else {
                    return;
                };
                let Some(running) = WasmHost::get(&ext, cx) else { return };
                let rx = running.ui_event("diff", "stage-hunk", UiEvent::Submit(patch.clone()));
                cx.spawn_in(window, async move |_, cx| {
                    if let Ok(Err(e)) = rx.await {
                        cx.update(|window, cx| window.push_notification(Notification::error(e), cx))
                            .ok();
                    }
                })
                .detach();
            }
        }
    }
}
