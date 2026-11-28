//! Extension の管理画面（中央のタブ、`Ctrl+Shift+X`）。
//! 「インストール済み」と「探す」（公開リポジトリの一覧）の2つ。入れる前に権限を見せて確認する。

use std::{collections::HashSet, sync::Arc};

use extension_host::{
    ExtensionStore, Manifest,
    manifest::SettingKind,
    registry::{self, Index, IndexEntry},
};
use gpui_kit::{
    AnyElement, App, AppContext as _, Context, Entity, EventEmitter, FocusHandle, Focusable, InteractiveElement as _,
    IntoElement, ParentElement, PromptLevel, Render, SharedString, StatefulInteractiveElement as _, Styled,
    Subscription, Task, Window,
    assets::IconName,
    component::{
        Disableable as _, Icon, Sizable as _, WindowExt as _,
        button::{Button, ButtonVariants as _},
        dock::{BasePanel, Panel, PanelEvent},
        h_flex,
        input::{Input, InputEvent, InputState},
        notification::Notification,
        v_flex,
    },
    div,
    prelude::FluentBuilder as _,
    px,
};
use i18n::Translate as _;
use theme::{Token, TokenColors as _};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Tab {
    Installed,
    Browse,
}

enum IndexState {
    NotLoaded,
    Loading,
    Loaded(Arc<Index>),
    Failed(String),
}

pub enum ExtensionsPageEvent {
    /// 設定の `disabled_extensions` を書き換える。
    SetDisabled(Vec<String>),
    /// 設定の `extension_settings` を書き換える。
    SetExtensionSettings(serde_json::Value),
}

pub struct ExtensionsPage {
    focus: FocusHandle,
    tab: Tab,
    query: Entity<InputState>,
    index: IndexState,
    installing: HashSet<String>,
    /// 設定を開いている Extension。
    expanded: HashSet<String>,
    /// 消したので、再起動すると無くなるもの。
    removed: HashSet<String>,
    _load: Option<Task<()>>,
    _subscriptions: Vec<Subscription>,
}

impl EventEmitter<PanelEvent> for ExtensionsPage {}
impl EventEmitter<ExtensionsPageEvent> for ExtensionsPage {}

/// 公開リポジトリの一覧の場所。
fn registry_url(cx: &App) -> String {
    settings::ActiveSettings::get(cx)
        .extensions_registry
        .clone()
        .unwrap_or_else(|| registry::DEFAULT_INDEX_URL.to_string())
}

fn install_dir() -> Option<std::path::PathBuf> {
    extension_host::user_extensions_dir()
}

impl ExtensionsPage {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let query = cx.new(|cx| InputState::new(window, cx).placeholder(cx.t("core.extensions.search")));
        // 起動時の更新の確認で読んだ一覧があれば使う。
        let index = cx
            .try_global::<ExtensionUpdates>()
            .and_then(|u| u.index.clone())
            .map_or(IndexState::NotLoaded, IndexState::Loaded);
        let subs = vec![
            cx.observe_global::<ExtensionUpdates>(|_, cx| cx.notify()),
            cx.subscribe(&query, |_, _, e: &InputEvent, cx| {
                if matches!(e, InputEvent::Change) {
                    cx.notify();
                }
            }),
            cx.observe_global::<ExtensionStore>(|_, cx| cx.notify()),
            cx.observe_global::<settings::ActiveSettings>(|_, cx| cx.notify()),
        ];
        ExtensionsPage {
            focus: cx.focus_handle(),
            tab: Tab::Installed,
            query,
            index,
            expanded: HashSet::new(),
            installing: HashSet::new(),
            removed: HashSet::new(),
            _load: None,
            _subscriptions: subs,
        }
    }

    fn set_tab(&mut self, tab: Tab, cx: &mut Context<Self>) {
        self.tab = tab;
        if tab == Tab::Browse && matches!(self.index, IndexState::NotLoaded | IndexState::Failed(_)) {
            self.load_index(cx);
        }
        cx.notify();
    }

    fn load_index(&mut self, cx: &mut Context<Self>) {
        self.index = IndexState::Loading;
        let url = registry_url(cx);
        let task = cx.background_spawn(async move { registry::fetch_index(&url) });
        self._load = Some(cx.spawn(async move |this, cx| {
            let r = task.await;
            this.update(cx, |this, cx| {
                this.index = match r {
                    Ok(i) => {
                        let i = Arc::new(i);
                        set_index(i.clone(), cx);
                        IndexState::Loaded(i)
                    }
                    Err(e) => IndexState::Failed(e),
                };
                cx.notify();
            })
            .ok();
        }));
        cx.notify();
    }

    fn matches(&self, texts: &[&str], cx: &App) -> bool {
        let q = self.query.read(cx).value().to_lowercase();
        q.trim().is_empty() || texts.iter().any(|t| t.to_lowercase().contains(q.trim()))
    }

    /// 入れる前に権限を見せて確認する。
    fn confirm_install(&mut self, entry: IndexEntry, window: &mut Window, cx: &mut Context<Self>) {
        let m = manifest_like(&entry);
        let mut lines: Vec<String> = m
            .permission_list()
            .into_iter()
            .map(|p| {
                let mark = if p.strong { " ⚠" } else { "" };
                format!("• {}: {}{mark}", p.category, p.value)
            })
            .collect();
        if lines.is_empty() {
            lines.push(cx.t("core.extensions.no-permissions").to_string());
        }
        let detail = format!(
            "{}\n{}\n\n{}",
            cx.t("core.extensions.permissions"),
            lines.join("\n"),
            cx.t("core.extensions.strong-note")
        );
        let message = cx.t_args(
            "core.extensions.confirm-install",
            &[
                ("name", entry.name.clone().into()),
                ("version", entry.version.clone().into()),
            ],
        );
        let answers = [cx.t("core.extensions.install"), cx.t("core.prompt.cancel")];
        let answer = window.prompt(
            PromptLevel::Info,
            &message,
            Some(&detail),
            &answers.iter().map(|a| a.as_ref()).collect::<Vec<&str>>(),
            cx,
        );
        cx.spawn_in(window, async move |this, cx| {
            if answer.await == Ok(0) {
                this.update_in(cx, |this, window, cx| this.install(entry, window, cx))
                    .ok();
            }
        })
        .detach();
    }

    fn install(&mut self, entry: IndexEntry, window: &mut Window, cx: &mut Context<Self>) {
        let Some(dest) = install_dir() else { return };
        let url = registry_url(cx);
        self.installing.insert(entry.id.clone());
        self.removed.remove(&entry.id);
        cx.notify();
        let e = entry.clone();
        let updating = ExtensionStore::get(cx).is_some_and(|s| s.extensions.iter().any(|x| x.manifest.id == entry.id));
        let task = cx.background_spawn(async move { registry::install(&e, &url, &dest) });
        cx.spawn_in(window, async move |this, cx| {
            let r = task.await;
            this.update_in(cx, |this, window, cx| {
                this.installing.remove(&entry.id);
                let r = r.and_then(|dir| extension_host::add_installed(dir, cx));
                recompute_updates(cx);
                let n = match r {
                    Ok(()) => Notification::success(cx.t_args(
                        if updating {
                            "core.extensions.updated-notify"
                        } else {
                            "core.extensions.installed-notify"
                        },
                        &[
                            ("name", entry.name.clone().into()),
                            ("version", entry.version.clone().into()),
                        ],
                    )),
                    Err(err) => Notification::error(cx.t_args(
                        "core.extensions.install-failed",
                        &[("name", entry.name.clone().into()), ("error", err.into())],
                    )),
                };
                window.push_notification(n, cx);
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    fn uninstall(&mut self, id: String, name: String, window: &mut Window, cx: &mut Context<Self>) {
        let message = cx.t_args("core.extensions.confirm-uninstall", &[("name", name.clone().into())]);
        let answers = [cx.t("core.extensions.uninstall"), cx.t("core.prompt.cancel")];
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
                let Some(dir) = install_dir() else { return };
                match registry::uninstall(&id, &dir) {
                    Ok(()) => {
                        this.removed.insert(id.clone());
                        let msg = cx.t_args("core.extensions.uninstalled-notify", &[("name", name.clone().into())]);
                        window.push_notification(Notification::info(msg), cx);
                    }
                    Err(e) => window.push_notification(Notification::error(e), cx),
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    fn toggle_enabled(&mut self, id: String, cx: &mut Context<Self>) {
        let mut disabled = settings::ActiveSettings::get(cx).disabled_extensions.clone();
        if disabled.contains(&id) {
            disabled.retain(|d| *d != id);
        } else {
            disabled.push(id);
        }
        cx.emit(ExtensionsPageEvent::SetDisabled(disabled));
    }

    fn render_installed(&mut self, cx: &mut Context<Self>) -> Vec<AnyElement> {
        let Some(store) = ExtensionStore::get(cx) else {
            return Vec::new();
        };
        let disabled = settings::ActiveSettings::get(cx).disabled_extensions.clone();
        let mut list: Vec<_> = store.extensions.to_vec();
        list.sort_by_key(|e| e.manifest.name.to_lowercase());
        let failed: Vec<_> = store.failed.to_vec();
        // 読み込み時点で無効だったもの（設定と違えば再起動で反映）。
        let loaded_disabled = store.disabled.clone();
        let mut out = Vec::new();
        for (ix, e) in list.into_iter().enumerate() {
            let m = &e.manifest;
            if !self.matches(&[&m.id, &m.name, m.description.as_deref().unwrap_or("")], cx) {
                continue;
            }
            let id = m.id.clone();
            let is_disabled = disabled.contains(&id);
            let removed = self.removed.contains(&id);
            let mut badges = Vec::new();
            if e.builtin {
                badges.push((cx.t("core.extensions.builtin"), Token::MutedForeground));
            }
            if is_disabled {
                badges.push((cx.t("core.extensions.disabled"), Token::HighlightModified));
            }
            if removed || is_disabled != loaded_disabled.contains(&id) {
                badges.push((cx.t("core.extensions.restart-required"), Token::HighlightModified));
            }
            let mut actions: Vec<AnyElement> = Vec::new();
            let update = cx.try_global::<ExtensionUpdates>().and_then(|u| {
                let up = u.available.iter().find(|x| x.id == id)?;
                u.index.as_ref()?.extensions.iter().find(|x| x.id == up.id).cloned()
            });
            if let Some(entry) = update.filter(|_| !removed) {
                let busy = self.installing.contains(&id);
                let b = Button::new(("update", ix))
                    .small()
                    .primary()
                    .disabled(busy)
                    .label(if busy {
                        cx.t("core.extensions.installing")
                    } else {
                        cx.t_args("core.extensions.update", &[("version", entry.version.clone().into())])
                    })
                    .on_click(cx.listener(move |this, _, window, cx| this.confirm_install(entry.clone(), window, cx)));
                actions.push(b.into_any_element());
            }
            if !m.contributes.settings.is_empty() {
                let sid = id.clone();
                let open = self.expanded.contains(&id);
                actions.push(
                    Button::new(("settings", ix))
                        .small()
                        .ghost()
                        .icon(IconName::Settings2)
                        .tooltip(cx.t("core.extensions.settings"))
                        .map(|b| if open { b.primary() } else { b })
                        .on_click(cx.listener(move |this, _, _, cx| {
                            if !this.expanded.remove(&sid) {
                                this.expanded.insert(sid.clone());
                            }
                            cx.notify();
                        }))
                        .into_any_element(),
                );
            }
            if !removed {
                let toggle_id = id.clone();
                actions.push(
                    Button::new(("toggle", ix))
                        .small()
                        .outline()
                        .label(if is_disabled {
                            cx.t("core.extensions.enable")
                        } else {
                            cx.t("core.extensions.disable")
                        })
                        .on_click(cx.listener(move |this, _, _, cx| this.toggle_enabled(toggle_id.clone(), cx)))
                        .into_any_element(),
                );
                if !e.builtin {
                    let (uid, name) = (id.clone(), m.name.clone());
                    actions.push(
                        Button::new(("uninstall", ix))
                            .small()
                            .ghost()
                            .label(cx.t("core.extensions.uninstall"))
                            .on_click(cx.listener(move |this, _, window, cx| {
                                this.uninstall(uid.clone(), name.clone(), window, cx)
                            }))
                            .into_any_element(),
                    );
                }
            }
            out.push(card(m, badges, actions, cx));
            if self.expanded.contains(&id) {
                out.push(self.settings_form(m, ix, cx));
            }
        }
        for f in failed {
            out.push(
                v_flex()
                    .p_3()
                    .gap_1()
                    .border_b_1()
                    .border_color(cx.token(Token::Border))
                    .text_sm()
                    .child(
                        div()
                            .text_color(cx.token(Token::HighlightError))
                            .child(f.location.clone()),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(cx.token(Token::MutedForeground))
                            .child(f.errors.join(" / ")),
                    )
                    .into_any_element(),
            );
        }
        out
    }

    /// Extension の設定（`contributes.settings`）。選んだ値はユーザー設定の `extension_settings` に書く。
    fn settings_form(&self, m: &Manifest, ix: usize, cx: &mut Context<Self>) -> AnyElement {
        let current = settings::ActiveSettings::get(cx)
            .extension_settings
            .get(&m.id)
            .cloned()
            .unwrap_or_default();
        let mut rows = Vec::new();
        for (si, s) in m.contributes.settings.iter().enumerate() {
            let value = current
                .get(&s.key)
                .filter(|v| s.accepts(v))
                .cloned()
                .unwrap_or_else(|| s.default.clone());
            let choices: Vec<(serde_json::Value, String)> = match s.kind {
                SettingKind::Enum => s
                    .options
                    .iter()
                    .map(|o| (serde_json::Value::String(o.value.clone()), o.label.clone()))
                    .collect(),
                SettingKind::Boolean => vec![
                    (serde_json::Value::Bool(true), cx.t("core.extensions.on").to_string()),
                    (serde_json::Value::Bool(false), cx.t("core.extensions.off").to_string()),
                ],
            };
            let buttons: Vec<AnyElement> = choices
                .into_iter()
                .enumerate()
                .map(|(ci, (v, label))| {
                    let selected = v == value;
                    let (id, key) = (m.id.clone(), s.key.clone());
                    Button::new(("setting", ix * 10_000 + si * 100 + ci))
                        .small()
                        .label(label)
                        .map(|b| if selected { b.primary() } else { b.outline() })
                        .on_click(cx.listener(move |this, _, _, cx| this.set_setting(&id, &key, v.clone(), cx)))
                        .into_any_element()
                })
                .collect();
            rows.push(
                h_flex()
                    .gap_4()
                    .items_start()
                    .child(
                        v_flex()
                            .flex_1()
                            .gap_0p5()
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(cx.token(Token::Foreground))
                                    .child(s.title.clone()),
                            )
                            .when_some(s.description.clone(), |d, desc| {
                                d.child(div().text_xs().text_color(cx.token(Token::MutedForeground)).child(desc))
                            }),
                    )
                    .child(h_flex().gap_1().children(buttons))
                    .into_any_element(),
            );
        }
        v_flex()
            .ml(px(64.))
            .mr_3()
            .mb_3()
            .p_3()
            .gap_3()
            .rounded_lg()
            .bg(cx.token(Token::MutedBackground))
            .children(rows)
            .into_any_element()
    }

    fn set_setting(&mut self, id: &str, key: &str, value: serde_json::Value, cx: &mut Context<Self>) {
        let mut all = settings::ActiveSettings::get(cx).extension_settings.clone();
        all.entry(id.to_string()).or_default().insert(key.to_string(), value);
        if let Ok(v) = serde_json::to_value(all) {
            cx.emit(ExtensionsPageEvent::SetExtensionSettings(v));
        }
    }

    fn render_browse(&mut self, cx: &mut Context<Self>) -> Vec<AnyElement> {
        let index = match &self.index {
            IndexState::Loaded(i) => i.clone(),
            IndexState::Loading | IndexState::NotLoaded => {
                return vec![message(cx.t("core.extensions.loading"), None, cx)];
            }
            IndexState::Failed(e) => {
                let retry = Button::new("retry")
                    .small()
                    .outline()
                    .label(cx.t("core.extensions.retry"))
                    .on_click(cx.listener(|this, _, _, cx| this.load_index(cx)))
                    .into_any_element();
                return vec![message(
                    cx.t_args("core.extensions.load-failed", &[("error", e.clone().into())]),
                    Some(retry),
                    cx,
                )];
            }
        };
        let installed: Vec<(String, String)> = ExtensionStore::get(cx)
            .map(|s| {
                s.extensions
                    .iter()
                    .map(|e| (e.manifest.id.clone(), e.manifest.version.clone()))
                    .collect()
            })
            .unwrap_or_default();
        let mut out = Vec::new();
        for (ix, entry) in index.extensions.iter().enumerate() {
            if !self.matches(
                &[&entry.id, &entry.name, entry.description.as_deref().unwrap_or("")],
                cx,
            ) {
                continue;
            }
            let have = installed.iter().find(|(id, _)| *id == entry.id).map(|(_, v)| v.clone());
            let busy = self.installing.contains(&entry.id);
            let mut badges = Vec::new();
            if have.is_some() {
                badges.push((cx.t("core.extensions.installed"), Token::HighlightCreated));
            }
            let action = match (&have, busy) {
                (_, true) => Some(
                    Button::new(("busy", ix))
                        .small()
                        .outline()
                        .disabled(true)
                        .label(cx.t("core.extensions.installing")),
                ),
                (Some(v), false) if *v == entry.version => None,
                (Some(_), false) => Some(
                    Button::new(("update", ix))
                        .small()
                        .primary()
                        .label(cx.t_args("core.extensions.update", &[("version", entry.version.clone().into())])),
                ),
                (None, false) => Some(
                    Button::new(("install", ix))
                        .small()
                        .primary()
                        .label(cx.t("core.extensions.install")),
                ),
            };
            let actions = action
                .map(|b| {
                    let e = entry.clone();
                    vec![
                        b.on_click(cx.listener(move |this, _, window, cx| this.confirm_install(e.clone(), window, cx)))
                            .into_any_element(),
                    ]
                })
                .unwrap_or_default();
            out.push(card(&manifest_like(entry), badges, actions, cx));
        }
        if out.is_empty() {
            out.push(message(cx.t("core.extensions.empty"), None, cx));
        }
        out
    }
}

/// 一覧の項目を、表示と権限の確認のために Manifest の形にする。
fn manifest_like(e: &IndexEntry) -> Manifest {
    Manifest {
        id: e.id.clone(),
        name: e.name.clone(),
        version: e.version.clone(),
        publisher: e.publisher.clone(),
        description: e.description.clone(),
        api: e.api.clone(),
        permissions: e.permissions.clone(),
        ..Default::default()
    }
}

fn message(text: SharedString, action: Option<AnyElement>, cx: &App) -> AnyElement {
    v_flex()
        .p_6()
        .gap_3()
        .items_center()
        .text_sm()
        .text_color(cx.token(Token::MutedForeground))
        .child(text)
        .children(action)
        .into_any_element()
}

/// 1つの Extension の行（デザインの Packages 画面: アイコン・名前・版・作者・説明・権限・ボタン）。
fn card(m: &Manifest, badges: Vec<(SharedString, Token)>, actions: Vec<AnyElement>, cx: &App) -> AnyElement {
    let perms = m.permission_list();
    h_flex()
        .items_start()
        .gap_3()
        .p_3()
        .border_b_1()
        .border_color(cx.token(Token::Border))
        .child(
            div()
                .size(px(40.))
                .flex_shrink_0()
                .rounded_lg()
                .bg(cx.token(Token::MutedBackground))
                .flex()
                .items_center()
                .justify_center()
                .child(
                    Icon::new(IconName::Puzzle)
                        .size(px(20.))
                        .text_color(cx.token(Token::ListHighlightForeground)),
                ),
        )
        .child(
            v_flex()
                .flex_1()
                .min_w_0()
                .gap_1()
                .child(
                    h_flex()
                        .gap_2()
                        .child(
                            div()
                                .text_sm()
                                .font_weight(gpui_kit::FontWeight::SEMIBOLD)
                                .text_color(cx.token(Token::Foreground))
                                .child(m.name.clone()),
                        )
                        .child(
                            div()
                                .text_xs()
                                .text_color(cx.token(Token::MutedForeground))
                                .child(m.version.clone()),
                        )
                        .children(badges.into_iter().map(|(t, color)| {
                            div()
                                .px_1p5()
                                .rounded_sm()
                                .border_1()
                                .border_color(cx.token(color))
                                .text_xs()
                                .text_color(cx.token(color))
                                .child(t)
                        })),
                )
                .child(
                    div()
                        .text_xs()
                        .text_color(cx.token(Token::MutedForeground))
                        .child(format!("{} · {}", m.publisher, m.id)),
                )
                .when_some(m.description.clone(), |d, desc| {
                    d.child(
                        div()
                            .text_sm()
                            .text_color(cx.token(Token::SidebarForeground))
                            .child(desc),
                    )
                })
                .when(!perms.is_empty(), |d| {
                    d.child(h_flex().flex_wrap().gap_1().pt_1().children(perms.into_iter().map(|p| {
                        let color = if p.strong {
                            Token::HighlightModified
                        } else {
                            Token::MutedForeground
                        };
                        div()
                            .px_1p5()
                            .rounded_sm()
                            .bg(cx.token(Token::MutedBackground))
                            .text_xs()
                            .text_color(cx.token(color))
                            .child(format!("{}: {}", p.category, p.value))
                    })))
                }),
        )
        .child(h_flex().gap_1().flex_shrink_0().children(actions))
        .into_any_element()
}

impl Focusable for ExtensionsPage {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}

impl BasePanel for ExtensionsPage {
    fn panel_name(&self) -> &'static str {
        "nucleus.extensions"
    }
}

impl Panel for ExtensionsPage {
    fn title(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        h_flex()
            .gap_2()
            .child(
                Icon::new(IconName::Puzzle)
                    .small()
                    .text_color(cx.token(Token::ListHighlightForeground)),
            )
            .child(cx.t("core.extensions.title"))
    }

    fn inner_padding(&self, _: &App) -> bool {
        false
    }
}

impl Render for ExtensionsPage {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let tab = |id: &'static str, t: Tab, key: &str, current: Tab, cx: &mut Context<Self>| {
            Button::new(id)
                .small()
                .label(cx.t(key))
                .map(|b| if t == current { b.primary() } else { b.ghost() })
                .on_click(cx.listener(move |this, _, _, cx| this.set_tab(t, cx)))
        };
        let rows = match self.tab {
            Tab::Installed => self.render_installed(cx),
            Tab::Browse => self.render_browse(cx),
        };
        let current = self.tab;
        v_flex()
            .track_focus(&self.focus)
            .size_full()
            .bg(cx.token(Token::Background))
            .child(
                v_flex()
                    .max_w(px(880.))
                    .w_full()
                    .mx_auto()
                    .px_4()
                    .pt_4()
                    .gap_3()
                    .child(
                        h_flex()
                            .gap_2()
                            .child(
                                div()
                                    .text_lg()
                                    .font_weight(gpui_kit::FontWeight::SEMIBOLD)
                                    .text_color(cx.token(Token::Foreground))
                                    .child(cx.t("core.extensions.title")),
                            )
                            .child(div().flex_1())
                            .child(tab(
                                "tab-installed",
                                Tab::Installed,
                                "core.extensions.tab-installed",
                                current,
                                cx,
                            ))
                            .child(tab(
                                "tab-browse",
                                Tab::Browse,
                                "core.extensions.tab-browse",
                                current,
                                cx,
                            ))
                            .when(current == Tab::Browse, |d| {
                                d.child(
                                    Button::new("reload")
                                        .small()
                                        .ghost()
                                        .icon(IconName::RefreshCw)
                                        .tooltip(cx.t("core.extensions.retry"))
                                        .on_click(cx.listener(|this, _, _, cx| this.load_index(cx))),
                                )
                            }),
                    )
                    .child(
                        Input::new(&self.query).small().prefix(
                            Icon::new(IconName::Search)
                                .small()
                                .text_color(cx.token(Token::MutedForeground)),
                        ),
                    ),
            )
            .child(
                div()
                    .id("extensions-list")
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .child(v_flex().max_w(px(880.)).w_full().mx_auto().px_4().pb_4().children(rows)),
            )
    }
}

/// 起動時に確かめた Extension の更新（gpui の Global）。
#[derive(Default)]
pub struct ExtensionUpdates {
    /// 最後に読んだ一覧。
    pub index: Option<Arc<Index>>,
    pub available: Vec<Update>,
}

impl gpui_kit::Global for ExtensionUpdates {}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Update {
    pub id: String,
    pub name: String,
    pub from: String,
    pub to: String,
}

/// `a` が `b` より新しい版か（`1.10.0` > `1.9.0`。数字でない部分は文字列で比べる）。
pub fn is_newer(a: &str, b: &str) -> bool {
    let parse = |v: &str| -> Vec<Result<u64, String>> {
        v.split(['.', '-', '+'])
            .map(|p| p.parse::<u64>().map_err(|_| p.to_string()))
            .collect()
    };
    let (a, b) = (parse(a), parse(b));
    for (x, y) in a.iter().zip(b.iter()) {
        let ord = match (x, y) {
            (Ok(x), Ok(y)) => x.cmp(y),
            (Ok(_), Err(_)) => std::cmp::Ordering::Greater,
            (Err(_), Ok(_)) => std::cmp::Ordering::Less,
            (Err(x), Err(y)) => x.cmp(y),
        };
        if ord != std::cmp::Ordering::Equal {
            return ord == std::cmp::Ordering::Greater;
        }
    }
    // 同じところまで等しい: 数字が続く方が新しい（1.0.1 > 1.0）、文字が続く方は古い（1.0.0-beta < 1.0.0）。
    match a.len().cmp(&b.len()) {
        std::cmp::Ordering::Greater => a[b.len()].is_ok(),
        std::cmp::Ordering::Less => b[a.len()].is_err(),
        std::cmp::Ordering::Equal => false,
    }
}

/// 一覧と、入れてある Extension（組み込みは除く）を比べる。
pub fn compute_updates(index: &Index, cx: &App) -> Vec<Update> {
    let Some(store) = ExtensionStore::get(cx) else {
        return Vec::new();
    };
    store
        .extensions
        .iter()
        .filter(|e| !e.builtin)
        .filter_map(|e| {
            let entry = index.extensions.iter().find(|x| x.id == e.manifest.id)?;
            is_newer(&entry.version, &e.manifest.version).then(|| Update {
                id: entry.id.clone(),
                name: entry.name.clone(),
                from: e.manifest.version.clone(),
                to: entry.version.clone(),
            })
        })
        .collect()
}

/// 一覧を新しくし、更新を数え直す。
pub fn set_index(index: Arc<Index>, cx: &mut App) {
    let available = compute_updates(&index, cx);
    cx.set_global(ExtensionUpdates {
        index: Some(index),
        available,
    });
}

/// 入れたり更新したりした後に数え直す。
pub fn recompute_updates(cx: &mut App) {
    if let Some(index) = cx.try_global::<ExtensionUpdates>().and_then(|u| u.index.clone()) {
        set_index(index, cx);
    }
}

#[cfg(test)]
mod tests {
    use super::is_newer;

    #[test]
    fn versions_compare_numerically() {
        assert!(is_newer("1.1.0", "1.0.0"));
        assert!(is_newer("1.10.0", "1.9.3"));
        assert!(!is_newer("1.0.0", "1.0.0"));
        assert!(!is_newer("0.9", "1.0.0"));
        assert!(is_newer("1.0.0", "1.0.0-beta"));
    }
}
