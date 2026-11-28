//! タブ・ウィンドウを閉じる流れと、未保存の確認。テーマや表示言語の切り替えもここに置く。

use editor::EditorPanel;
use gpui_kit::{
    App, AppContext as _, Context, Entity, PromptLevel, Task, WeakEntity, Window, component::dock::PanelId,
};
use i18n::Translate as _;

use crate::Workspace;

impl Workspace {
    pub(crate) fn dirty_editors(&self, cx: &App) -> Vec<Entity<EditorPanel>> {
        self.editors
            .values()
            .filter_map(WeakEntity::upgrade)
            .filter(|e| e.read(cx).is_dirty() && self.dock_area.read(cx).panel(PanelId::from(e.entity_id())).is_some())
            .collect()
    }

    /// 未保存のファイルがあれば確認する。確認中は閉じない（false）。
    pub(crate) fn should_close_window(&mut self, window: &mut Window, cx: &mut Context<Self>) -> bool {
        if self.close_confirmed {
            self.save_session_now(cx);
            return true;
        }
        let dirty = self.dirty_editors(cx);
        if dirty.is_empty() {
            self.save_session_now(cx);
            return true;
        }
        let message = if dirty.len() == 1 {
            cx.t_args(
                "core.prompt.unsaved-one",
                &[("file", dirty[0].read(cx).file_name().to_string().into())],
            )
        } else {
            cx.t_args("core.prompt.unsaved-many", &[("count", dirty.len().into())])
        };
        let detail = cx.t("core.prompt.unsaved-detail");
        let answers = [
            cx.t("core.prompt.save-all"),
            cx.t("core.prompt.dont-save"),
            cx.t("core.prompt.cancel"),
        ];
        let answer = window.prompt(
            PromptLevel::Warning,
            &message,
            Some(&detail),
            &answers.iter().map(|a| a.as_ref()).collect::<Vec<&str>>(),
            cx,
        );
        cx.spawn_in(window, async move |this, cx| {
            let Ok(choice) = answer.await else { return };
            let saves: Vec<Task<Result<(), String>>> = match choice {
                0 => {
                    let Ok(tasks) = this.update(cx, |_, cx| {
                        dirty.iter().map(|e| e.update(cx, |e, cx| e.save(cx))).collect()
                    }) else {
                        return;
                    };
                    tasks
                }
                1 => Vec::new(),
                _ => return,
            };
            for t in saves {
                if t.await.is_err() {
                    return; // 保存に失敗したら閉じない（通知はワークスペースが出す）。
                }
            }
            this.update_in(cx, |this, window, cx| {
                this.close_confirmed = true;
                this.save_session_now(cx);
                window.remove_window();
            })
            .ok();
        })
        .detach();
        false
    }

    /// アクティブなタブを閉じる。未保存なら「保存 / 保存しない / キャンセル」を確認する。
    pub(crate) fn close_active_editor(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(editor) = self.active_editor(cx) else { return };
        if !editor.read(cx).is_dirty() {
            self.remove_editor(editor, window, cx);
            return;
        }
        let message = cx.t_args(
            "core.prompt.unsaved-one",
            &[("file", editor.read(cx).file_name().to_string().into())],
        );
        let detail = cx.t("core.prompt.unsaved-detail");
        let answers = [
            cx.t("core.prompt.save"),
            cx.t("core.prompt.dont-save"),
            cx.t("core.prompt.cancel"),
        ];
        let answer = window.prompt(
            PromptLevel::Warning,
            &message,
            Some(&detail),
            &answers.iter().map(|a| a.as_ref()).collect::<Vec<&str>>(),
            cx,
        );
        cx.spawn_in(window, async move |this, cx| {
            let Ok(choice) = answer.await else { return };
            match choice {
                0 => {
                    let Ok(save) = this.update(cx, |_, cx| editor.update(cx, |e, cx| e.save(cx))) else {
                        return;
                    };
                    if save.await.is_err() {
                        return;
                    }
                }
                1 => {}
                _ => return,
            }
            this.update_in(cx, |this, window, cx| this.remove_editor(editor, window, cx))
                .ok();
        })
        .detach();
    }

    pub(crate) fn remove_editor(&mut self, editor: Entity<EditorPanel>, window: &mut Window, cx: &mut Context<Self>) {
        let path = editor.read(cx).path().to_path_buf();
        self.editors.remove(&path);
        self.open_order.retain(|p| *p != path);
        self.lsp_closed(&path);
        if self
            .active
            .as_ref()
            .is_some_and(|a| a.entity_id() == editor.entity_id())
        {
            self.active = None;
        }
        self.dock_area
            .update(cx, |area, cx| area.remove_panel(editor, window, cx));
        cx.notify();
    }

    pub(crate) fn quit(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.should_close_window(window, cx) {
            cx.quit();
        } else {
            // 確認ダイアログの後でウィンドウが閉じたら終了する。
            cx.on_window_closed(|cx, _| cx.quit()).detach();
        }
    }

    /// Nucleus One の Dark と Light を切り替え、ユーザー設定に書き込む。
    pub(crate) fn toggle_theme(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let mode = match cx.global::<theme::ActiveTheme>().resolved.mode {
            theme::Mode::Dark => theme::Mode::Light,
            theme::Mode::Light => theme::Mode::Dark,
        };
        self.choose_setting("theme", serde_json::json!(theme::builtin(mode).name), window, cx);
    }

    /// 設定を書き込み、監視を張り直して反映する（設定ファイルが新しく作られる場合に備える）。
    pub(crate) fn choose_setting(
        &mut self,
        key: impl Into<String>,
        value: serde_json::Value,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        /// 書き込みは読んで書き換えるので、続けて変えたときに前の変更を失わないよう1つずつ行う。
        static WRITE: std::sync::Mutex<()> = std::sync::Mutex::new(());
        let Some(path) = settings::user_settings_path() else {
            return;
        };
        let key: String = key.into();
        let write = cx.background_spawn(async move {
            let _guard = WRITE.lock().unwrap_or_else(|e| e.into_inner());
            settings::set_value(&path, &key, &value)
        });
        cx.spawn_in(window, async move |this, cx| match write.await {
            Ok(()) => {
                this.update_in(cx, |this, window, cx| this.watch_settings(window, cx))
                    .ok();
            }
            Err(e) => log::error!("設定を書き込めなかった: {e}"),
        })
        .detach();
    }
}
