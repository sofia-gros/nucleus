//! Search パネルと Workspace のつなぎ（開く・すべて置き換える）。

use std::path::PathBuf;

use gpui_kit::{
    Context, PromptLevel, WeakEntity, Window,
    component::{
        WindowExt as _,
        dock::{DockPlacement, PanelId},
        notification::Notification,
    },
};
use i18n::Translate as _;
use project::search::{self, SearchOptions};
use remote::RemoteExt as _;

use crate::{Workspace, search_panel::SearchPanelEvent};

impl Workspace {
    pub(crate) fn on_search_event(&mut self, e: &SearchPanelEvent, window: &mut Window, cx: &mut Context<Self>) {
        match e {
            SearchPanelEvent::Open { path, line, column } => {
                let pos = lsp_types::Position::new(*line, *column);
                self.open_file_at(path.clone(), Some(pos), window, cx);
            }
            SearchPanelEvent::ReplaceAll {
                options,
                replacement,
                files,
                count,
            } => self.confirm_replace(options.clone(), replacement.clone(), files.clone(), *count, window, cx),
        }
    }

    /// 左ドックの Search を開いて検索欄にフォーカスする（Ctrl+Shift+F）。
    pub(crate) fn open_project_search(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let id = PanelId::from(self.search.entity_id());
        self.dock_area.update(cx, |area, cx| {
            if !area.is_dock_open(DockPlacement::Left) {
                area.toggle_dock(DockPlacement::Left, window, cx);
            }
            area.select_panel(id, window, cx);
        });
        self.search.update(cx, |s, cx| s.focus_query(None, window, cx));
    }

    fn confirm_replace(
        &mut self,
        options: SearchOptions,
        replacement: String,
        files: Vec<PathBuf>,
        count: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let message = cx.t_args(
            "core.search.confirm-replace",
            &[
                ("count", count.into()),
                ("files", files.len().into()),
                ("replacement", replacement.clone().into()),
            ],
        );
        let answers = [cx.t("core.search.replace-all"), cx.t("core.prompt.cancel")];
        let answer = window.prompt(
            PromptLevel::Warning,
            &message,
            None,
            &answers.iter().map(|a| a.as_ref()).collect::<Vec<&str>>(),
            cx,
        );
        cx.spawn_in(window, async move |this, cx| {
            if answer.await != Ok(0) {
                return;
            }
            this.update_in(cx, |this, window, cx| {
                this.replace_in_files(&options, &replacement, &files, window, cx)
            })
            .ok();
        })
        .detach();
    }

    /// 開いているファイルはエディタの中で（元に戻せる）、それ以外はディスク上で置き換える。
    fn replace_in_files(
        &mut self,
        options: &SearchOptions,
        replacement: &str,
        files: &[PathBuf],
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let re = match search::build_regex(options) {
            Ok(re) => re,
            Err(e) => {
                window.push_notification(Notification::error(e), cx);
                return;
            }
        };
        let (mut total, mut changed) = (0usize, 0usize);
        let mut on_disk = Vec::new();
        for path in files {
            if let Some(editor) = self.editors.get(path).and_then(WeakEntity::upgrade) {
                let state = editor.read(cx).state().clone();
                let text = state.read(cx).value().to_string();
                let (new, n) = search::replace_text(&text, &re, replacement, options.regex);
                if n > 0 {
                    state.update(cx, |s, cx| s.replace_all(new, window, cx));
                    total += n;
                    changed += 1;
                }
                continue;
            }
            on_disk.push(path.clone());
        }
        // 開いていないファイルはホストで読み書きする（改行の形は元のまま）。
        let client = cx.remote();
        let (replacement, regex_mode) = (replacement.to_string(), options.regex);
        cx.spawn_in(window, async move |this, cx| {
            let mut failed = Vec::new();
            for path in on_disk {
                let loaded = match editor::file::load(client.clone(), path.clone()).await {
                    Ok(l) => l,
                    Err(e) => {
                        failed.push(format!("{}: {e}", path.display()));
                        continue;
                    }
                };
                let (new, n) = search::replace_text(&loaded.text, &re, &replacement, regex_mode);
                if n == 0 {
                    continue;
                }
                let bytes = editor::file::encode(std::iter::once(new.as_str()), loaded.line_ending);
                match client.write_file(&path, bytes).await {
                    Ok(()) => {
                        total += n;
                        changed += 1;
                    }
                    Err(e) => failed.push(format!("{}: {e}", path.display())),
                }
            }
            this.update_in(cx, |this, window, cx| {
                let msg = cx.t_args(
                    "core.search.replaced",
                    &[("count", total.into()), ("files", changed.into())],
                );
                window.push_notification(Notification::success(msg), cx);
                for f in failed {
                    window.push_notification(Notification::error(f), cx);
                }
                this.search.update(cx, |s, cx| s.rerun(cx));
            })
            .ok();
        })
        .detach();
    }
}
