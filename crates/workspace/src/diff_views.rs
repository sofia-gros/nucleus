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
        let current = self.current_text(&path, cx);
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

    /// エディタで開いていればその中身、なければディスクの中身。
    fn current_text(&self, path: &Path, cx: &gpui_kit::App) -> String {
        match self.editors.get(path).and_then(WeakEntity::upgrade) {
            Some(e) => e.read(cx).state().read(cx).value().to_string(),
            None => std::fs::read_to_string(path).unwrap_or_default(),
        }
    }

    /// エディタの中身が変わったら、開いている Diff を新しくする。
    pub(crate) fn refresh_diff(&mut self, path: &Path, cx: &mut Context<Self>) {
        if let Some(view) = self.diff_view_for(path, cx) {
            let text = self.current_text(path, cx);
            view.update(cx, |v, cx| v.set_current(text, cx));
        }
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
                } else if let Err(e) = std::fs::write(path, text) {
                    window.push_notification(Notification::error(format!("{}: {e}", path.display())), cx);
                    return;
                } else {
                    self.extension_file_saved(path, cx);
                }
                self.refresh_diff(path, cx);
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
