//! 設定の読み込み・監視・反映（テーマ、表示言語、フォント、キーバインド）。

use std::{path::PathBuf, sync::Arc, time::Duration};

use futures::StreamExt as _;
use gpui_kit::KeyBindingContextPredicate;
use gpui_kit::{
    App, AppContext as _, Context, KeyBinding, Window, component::WindowExt as _, component::notification::Notification,
};
use i18n::Translate as _;
use remote::RemoteExt as _;
use settings::{ActiveSettings, Settings};

use crate::Workspace;

/// 起動時に一度だけ同期的に読む（ウィンドウを開く前。小さな JSON なので最初のフレームを遅らせない）。
pub fn init(cx: &mut App) -> Vec<String> {
    let loaded = settings::load(settings::user_settings_path().as_deref(), None);
    apply(&loaded.settings, None, cx);
    apply_keymap(&loaded.settings, cx);
    cx.set_global(ActiveSettings(Arc::new(loaded.settings)));
    for w in &loaded.warnings {
        log::warn!("{w}");
    }
    loaded.errors
}

/// 選べるテーマ（同梱テーマと、Extension が `theme.colors` で提供するもの）。
pub fn available_themes(cx: &App) -> Vec<theme::ThemeFile> {
    let mut all: Vec<theme::ThemeFile> = [theme::Mode::Dark, theme::Mode::Light]
        .into_iter()
        .map(theme::builtin)
        .collect();
    if let Some(store) = extension_host::ExtensionStore::get(cx) {
        all.extend(store.themes().cloned());
    }
    all
}

fn theme_file(name: &str, cx: &App) -> Option<theme::ThemeFile> {
    available_themes(cx).into_iter().find(|t| t.name == name)
}

/// Extension の読み込みが終わったら、テーマ（拡張 Token を含む）と言語の対応付けを反映し直す。
pub fn on_extensions_changed(cx: &mut App) {
    let Some(store) = extension_host::ExtensionStore::get(cx) else {
        return;
    };
    if store.stage != extension_host::Stage::Ready {
        return;
    }
    let overrides = editor::LanguageOverrides {
        by_extension: store.languages_by_extension.clone(),
        by_file_name: store.languages_by_file_name.clone(),
    };
    cx.set_global(overrides);
    let s = ActiveSettings::get(cx);
    apply_theme(&s, cx);
}

fn apply_theme(s: &Settings, cx: &mut App) {
    match theme_file(&s.theme, cx) {
        Some(mut file) => {
            let mut fonts = file.fonts.clone().unwrap_or_default();
            fonts.mono = s.editor.font_family.clone();
            // window.zoom_level はエディタの文字にも効かせる（画面のほかの部分は rem の大きさで変わる）。
            fonts.mono_size = s.editor.font_size * s.window.zoom();
            file.fonts = Some(fonts);
            let tokens = extension_host::ExtensionStore::get(cx)
                .map(|e| e.tokens.clone())
                .unwrap_or_default();
            theme::apply(&file, &tokens, cx);
        }
        None => {
            // Extension の読み込み前は、Extension のテーマはまだ見つからない（読み込み後に反映し直す）。
            let loading =
                extension_host::ExtensionStore::get(cx).is_none_or(|e| e.stage != extension_host::Stage::Ready);
            if !loading {
                log::warn!("テーマ `{}` が見つからない", s.theme);
            }
        }
    }
}

/// 設定を反映する。`prev` と同じ項目は触らない（言語の切り替えなどで全画面を描き直さないため）。
pub fn apply(s: &Settings, prev: Option<&Settings>, cx: &mut App) {
    let theme_changed = prev.is_none_or(|p| {
        p.theme != s.theme
            || p.editor.font_family != s.editor.font_family
            || p.editor.font_size != s.editor.font_size
            || p.window.zoom_level != s.window.zoom_level
    });
    if theme_changed {
        apply_theme(s, cx);
    }
    if prev.is_none_or(|p| p.language != s.language) {
        let lang = s
            .language
            .as_deref()
            .and_then(|l| l.parse().ok())
            .unwrap_or_else(i18n::detect_language);
        if cx.try_global::<i18n::I18n>().is_some() {
            i18n::set_language(lang, cx);
        }
    }
}

/// 設定のキーバインドを登録する（gpui は個別の解除ができないので、変更は再起動後に反映）。
fn apply_keymap(s: &Settings, cx: &mut App) {
    let mut bindings = Vec::new();
    for entry in &s.keymap {
        let action = match cx.build_action(&entry.command, None) {
            Ok(a) => a,
            Err(e) => {
                log::warn!("keymap: `{}` は使えない: {e}", entry.command);
                continue;
            }
        };
        let context = match entry.context.as_deref().map(KeyBindingContextPredicate::parse) {
            Some(Ok(p)) => Some(p.into()),
            Some(Err(e)) => {
                log::warn!("keymap: context `{:?}` が読めない: {e}", entry.context);
                continue;
            }
            None => None,
        };
        match KeyBinding::load(&entry.key, action, context, false, None, cx) {
            Ok(b) => bindings.push(b),
            Err(e) => log::warn!("keymap: `{}` が読めない: {e}", entry.key),
        }
    }
    cx.bind_keys(bindings);
}

impl Workspace {
    /// ユーザー設定とワークスペース設定のファイルを監視し、保存されたら反映する。
    pub(crate) fn watch_settings(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let user = settings::user_settings_path();
        let workspace = self
            .project
            .read(cx)
            .root()
            .map(|r| settings::workspace_settings_path(&r));
        let client = cx.remote();
        let mut rx = client.subscribe_where(|e| matches!(e, remote::Event::DirChanged(_)));
        let dirs: Vec<PathBuf> = [&user, &workspace]
            .into_iter()
            .flatten()
            .filter_map(|p| p.parent().map(PathBuf::from))
            .collect();
        for dir in &dirs {
            client.watch(dir);
        }
        let ours = move |e: &remote::Event| match e {
            remote::Event::DirChanged(d) => dirs.iter().any(|x| *x == remote::unwire(d)),
            _ => false,
        };
        self.reload_settings(user.clone(), workspace.clone(), window, cx);
        self._settings_task = Some(cx.spawn_in(window, async move |this, cx| {
            while let Some(e) = rx.next().await {
                if !ours(&e) {
                    continue;
                }
                cx.background_executor().timer(Duration::from_millis(100)).await;
                while rx.try_recv().is_ok() {}
                let (u, w) = (user.clone(), workspace.clone());
                if this
                    .update_in(cx, |this, window, cx| this.reload_settings(u, w, window, cx))
                    .is_err()
                {
                    break;
                }
            }
        }));
    }

    fn reload_settings(
        &mut self,
        user: Option<PathBuf>,
        workspace: Option<PathBuf>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let load = cx.background_spawn(async move { settings::load(user.as_deref(), workspace.as_deref()) });
        cx.spawn_in(window, async move |_, cx| {
            let loaded = load.await;
            cx.update(|window, cx| {
                for w in &loaded.warnings {
                    log::warn!("{w}");
                }
                for e in &loaded.errors {
                    let msg = cx.t_args("core.notify.settings-error", &[("error", e.clone().into())]);
                    window.push_notification(Notification::error(msg), cx);
                }
                let prev = ActiveSettings::get(cx);
                if *prev != loaded.settings {
                    if prev.keymap != loaded.settings.keymap {
                        window.push_notification(Notification::info(cx.t("core.notify.keymap-restart")), cx);
                    }
                    apply(&loaded.settings, Some(&prev), cx);
                    cx.set_global(ActiveSettings(Arc::new(loaded.settings)));
                }
            })
            .ok();
        })
        .detach();
    }

    /// ユーザー設定ファイルを開く（なければひな形を作る）。
    pub(crate) fn open_user_settings(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(path) = settings::user_settings_path() else {
            return;
        };
        let create = {
            let path = path.clone();
            cx.background_spawn(async move {
                if !path.exists() {
                    if let Some(dir) = path.parent() {
                        std::fs::create_dir_all(dir)?;
                    }
                    std::fs::write(&path, settings::USER_TEMPLATE)?;
                }
                anyhow::Ok(())
            })
        };
        cx.spawn_in(window, async move |this, cx| {
            if let Err(e) = create.await {
                log::error!("設定ファイルを作れなかった: {e}");
                return;
            }
            this.update_in(cx, |this, window, cx| {
                this.open_file(path, window, cx);
                // ファイルを新しく作った場合に備えて監視を張り直す。
                this.watch_settings(window, cx);
            })
            .ok();
        })
        .detach();
    }

    /// ワークスペース設定ファイルを開く（なければ空のファイルを作る）。
    pub(crate) fn open_workspace_settings(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(root) = self.project.read(cx).root() else {
            return;
        };
        let path = settings::workspace_settings_path(&root);
        let create = {
            let path = path.clone();
            cx.background_spawn(async move {
                if !path.exists() {
                    if let Some(dir) = path.parent() {
                        std::fs::create_dir_all(dir)?;
                    }
                    std::fs::write(&path, "// このフォルダだけの設定（ユーザー設定を上書きします）\n{\n}\n")?;
                }
                anyhow::Ok(())
            })
        };
        cx.spawn_in(window, async move |this, cx| {
            if create.await.is_ok() {
                this.update_in(cx, |this, window, cx| {
                    this.open_file(path, window, cx);
                    this.watch_settings(window, cx);
                })
                .ok();
            }
        })
        .detach();
    }
}
