//! WASM の Extension と UI のつなぎ（段階3）。
//! コマンドの実行、ショートカットの登録、Extension からのお願い（通知・エディタの読み書き）を UI スレッドで処理する。

use std::collections::HashSet;

use futures::StreamExt as _;
use gpui_kit::{
    App, Context, KeyBinding, Window,
    component::{WindowExt as _, notification::Notification},
};
use i18n::Translate as _;
use serde::Deserialize;

use std::path::Path;

use extension_host::{ActiveDocument, NoticeLevel, UiRequest, WasmHost, WorkspaceEvent, manifest::ViewLocation};
use gpui_kit::{
    AppContext as _, WeakEntity,
    component::dock::{DockLayout, DockPlacement, panel_handle},
};

use crate::{CONTEXT, Workspace, extension_view::ExtensionPanel};

/// Extension のコマンドを実行する（`contributes.commands`）。
#[derive(Clone, PartialEq, Eq, Deserialize, gpui_kit::Action)]
#[action(namespace = workspace, no_json)]
pub struct RunExtensionCommand {
    pub extension: String,
    pub command: String,
}

/// 有効な Extension のショートカットを登録する（まだのものだけ）。
pub fn bind_keys(bound: &mut HashSet<(String, String)>, cx: &mut App) {
    let Some(store) = extension_host::ExtensionStore::get(cx) else {
        return;
    };
    let mut bindings = Vec::new();
    for c in store.commands() {
        let Some(key) = c.command.key.clone() else { continue };
        if !bound.insert((c.extension.clone(), c.command.id.clone())) {
            continue;
        }
        match KeyBinding::load(
            &key,
            Box::new(RunExtensionCommand {
                extension: c.extension.clone(),
                command: c.command.id.clone(),
            }),
            gpui_kit::KeyBindingContextPredicate::parse(CONTEXT)
                .ok()
                .map(std::rc::Rc::new),
            false,
            None,
            cx,
        ) {
            Ok(b) => bindings.push(b),
            Err(e) => log::warn!("Extension `{}` のショートカット `{key}` を読めない: {e}", c.extension),
        }
    }
    cx.bind_keys(bindings);
}

/// コマンドの表示名。`%key%` なら Extension の文言から引く。
pub fn command_title(extension: &str, title: &str, cx: &App) -> String {
    match title.strip_prefix('%').and_then(|t| t.strip_suffix('%')) {
        Some(key) => {
            let full = format!("{extension}.{key}");
            let s = cx.t(&full);
            if s.as_ref() == full {
                title.to_string()
            } else {
                s.to_string()
            }
        }
        None => title.to_string(),
    }
}

impl Workspace {
    /// Extension からのお願いを受け取り始める（ウィンドウごとではなく最初の1つだけ）。
    pub(crate) fn start_extension_bridge(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(mut rx) = WasmHost::take_requests(cx) else {
            return;
        };
        cx.spawn_in(window, async move |this, cx| {
            while let Some(req) = rx.next().await {
                if this
                    .update_in(cx, |this, window, cx| this.handle_extension_request(req, window, cx))
                    .is_err()
                {
                    break;
                }
            }
        })
        .detach();
    }

    /// Extension の基準のフォルダをいまのプロジェクトに合わせる。
    pub(crate) fn sync_extension_root(&self, cx: &App) {
        if let Some(h) = cx.try_global::<WasmHost>() {
            h.context.set_workspace_root(self.project.read(cx).root());
        }
    }

    fn handle_extension_request(&mut self, req: UiRequest, window: &mut Window, cx: &mut Context<Self>) {
        match req {
            UiRequest::Notify {
                extension,
                level,
                message,
            } => {
                let text = format!("{extension}: {message}");
                let n = match level {
                    NoticeLevel::Info => Notification::info(text),
                    NoticeLevel::Warning => Notification::warning(text),
                    NoticeLevel::Error => Notification::error(text),
                };
                window.push_notification(n, cx);
            }
            UiRequest::ActiveDocument { reply } => {
                let doc = self.active_editor(cx).map(|e| {
                    let e = e.read(cx);
                    let root = self.project.read(cx).root();
                    let path = root
                        .as_deref()
                        .and_then(|r| e.path().strip_prefix(r).ok())
                        .unwrap_or(e.path());
                    ActiveDocument {
                        path: path.display().to_string(),
                        language: e.language().to_string(),
                        text: e.state().read(cx).value().to_string(),
                    }
                });
                let _ = reply.send(doc);
            }
            UiRequest::OpenFile { path, line } => {
                let pos = line.map(|l| lsp_types::Position::new(l.saturating_sub(1), 0));
                self.open_file_at(path, pos, window, cx);
            }
            UiRequest::Invalidate { extension } => {
                self.extension_panels.retain(|p| p.upgrade().is_some());
                for p in self.extension_panels.iter().filter_map(WeakEntity::upgrade) {
                    if p.read(cx).extension == extension {
                        p.update(cx, |p, cx| p.refresh(cx));
                    }
                }
            }
            UiRequest::ReplaceActiveText { text, reply } => {
                let r = match self.active_editor(cx) {
                    Some(e) => {
                        let state = e.read(cx).state().clone();
                        state.update(cx, |s, cx| s.replace_all(text, window, cx));
                        Ok(())
                    }
                    None => Err("ファイルが開かれていない".to_string()),
                };
                let _ = reply.send(r);
            }
        }
    }

    /// Extension のビューをドックに置く（段階2: 中身は Extension の起動まで Skeleton）。
    pub(crate) fn add_extension_views(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(store) = extension_host::ExtensionStore::get(cx) else {
            return;
        };
        let mut right = Vec::new();
        for v in store.views() {
            let placement = match v.view.location {
                ViewLocation::Left => DockPlacement::Left,
                ViewLocation::Right => DockPlacement::Right,
                ViewLocation::Bottom => DockPlacement::Bottom,
            };
            let panel = cx.new(|cx| ExtensionPanel::new(v.extension.clone(), v.view.clone(), window, cx));
            self.extension_panels.push(panel.downgrade());
            if placement == DockPlacement::Right {
                // 右ドックは Extension のビュー（Git など）を先頭にする（デザイン 08）。
                right.push(panel_handle(panel));
            } else {
                self.dock_area.update(cx, |area, cx| {
                    area.add_panel_view(panel_handle(panel), placement, None, window, cx);
                });
            }
        }
        if right.is_empty() {
            return;
        }
        let outline = self.outline_panel.clone();
        self.dock_area.update(cx, |area, cx| {
            let open = area.is_dock_open(DockPlacement::Right);
            let mut layout = DockLayout::tabs();
            for h in right.into_iter().chain(outline) {
                layout = layout.panel_view(h, cx);
            }
            area.set_dock(DockPlacement::Right, layout, window, cx);
            area.set_dock_size(DockPlacement::Right, gpui_kit::px(320.), window, cx);
            if area.is_dock_open(DockPlacement::Right) != open {
                area.toggle_dock(DockPlacement::Right, window, cx);
            }
        });
    }

    /// 保存したファイルを Extension に知らせる（ワークスペースからの相対パス）。
    pub(crate) fn extension_file_saved(&self, path: &Path, cx: &App) {
        let root = self.project.read(cx).root();
        let rel = root.as_deref().and_then(|r| path.strip_prefix(r).ok()).unwrap_or(path);
        WasmHost::broadcast(WorkspaceEvent::FileSaved(rel.display().to_string()), cx);
    }

    pub(crate) fn run_extension_command(
        &mut self,
        a: &RunExtensionCommand,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(rx) = WasmHost::run_command(&a.extension, &a.command, cx) else {
            let msg = cx.t_args(
                "core.notify.extension-not-running",
                &[("id", a.extension.clone().into())],
            );
            window.push_notification(Notification::warning(msg), cx);
            return;
        };
        let name = a.command.clone();
        cx.spawn_in(window, async move |_, cx| {
            let result = rx.await.unwrap_or_else(|_| Err("Extension が止まった".into()));
            if let Err(e) = result {
                cx.update(|window, cx| {
                    let msg = cx.t_args(
                        "core.notify.extension-command-failed",
                        &[("command", name.into()), ("error", e.into())],
                    );
                    window.push_notification(Notification::error(msg), cx);
                })
                .ok();
            }
        })
        .detach();
    }
}
