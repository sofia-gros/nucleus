//! Extension の管理画面（中央のタブ、`Ctrl+Shift+X`）。
//! 「インストール済み」と「探す」（公開リポジトリの一覧）の2つ。入れる前に権限を見せて確認する。

use std::{collections::HashSet, sync::Arc};

use extension_host::{
    ExtensionStore, Manifest,
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
}

pub struct ExtensionsPage {
    focus: FocusHandle,
    tab: Tab,
    query: Entity<InputState>,
    index: IndexState,
    installing: HashSet<String>,
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
        let subs = vec![
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
            index: IndexState::NotLoaded,
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
                    Ok(i) => IndexState::Loaded(Arc::new(i)),
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
        let task = cx.background_spawn(async move { registry::install(&e, &url, &dest) });
        cx.spawn_in(window, async move |this, cx| {
            let r = task.await;
            this.update_in(cx, |this, window, cx| {
                this.installing.remove(&entry.id);
                let r = r.and_then(|dir| extension_host::add_installed(dir, cx));
                let n = match r {
                    Ok(()) => Notification::success(cx.t_args(
                        "core.extensions.installed-notify",
                        &[("name", entry.name.clone().into())],
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
            if removed || is_disabled != store.disabled.contains(&id) {
                badges.push((cx.t("core.extensions.restart-required"), Token::HighlightModified));
            }
            let mut actions: Vec<AnyElement> = Vec::new();
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
