//! 設定画面（中央のタブ、`Ctrl+,`、デザイン 11）。左に分類、右に項目。変えた値はすぐユーザー設定に書く。
//! 設定ファイル（JSON）は画面の上のボタンから開ける。

use std::collections::{HashMap, HashSet};

use gpui_kit::{
    AnyElement, App, AppContext as _, Context, Entity, EventEmitter, FocusHandle, Focusable, InteractiveElement as _,
    IntoElement, ParentElement, Render, SharedString, StatefulInteractiveElement as _, Styled, Subscription, Window,
    assets::IconName,
    component::{
        Icon, Sizable as _,
        button::{Button, ButtonVariants as _},
        dock::{BasePanel, Panel, PanelEvent},
        h_flex,
        input::{InputEvent, InputState},
        menu::{DropdownMenu as _, PopupMenu},
        switch::Switch,
        v_flex,
    },
    div,
    prelude::FluentBuilder as _,
    px,
};
use i18n::Translate as _;
use serde::Deserialize;
use serde_json::{Value, json};
use theme::{Token, TokenColors as _};

use crate::palette::{PaletteKind, choices};

pub enum SettingsPageEvent {
    /// トップレベルのキーに書く（`editor` はオブジェクトごと）。
    Set(String, Value),
    OpenJson,
}

/// ドロップダウンで選んだ値（メニューの項目はアクションで表す）。
#[derive(Clone, PartialEq, Eq, Deserialize, gpui_kit::Action)]
#[action(namespace = settings_page, no_json)]
pub struct ChooseSetting {
    pub key: String,
    /// JSON の値。
    pub value: String,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Category {
    General,
    Editor,
    Extensions,
}

impl Category {
    const ALL: [Category; 3] = [Category::General, Category::Editor, Category::Extensions];

    fn key(self) -> &'static str {
        match self {
            Category::General => "core.settings.general",
            Category::Editor => "core.settings.editor",
            Category::Extensions => "core.settings.extensions",
        }
    }

    fn icon(self) -> IconName {
        match self {
            Category::General => IconName::Settings2,
            Category::Editor => IconName::FileCode,
            Category::Extensions => IconName::Puzzle,
        }
    }
}

#[derive(Clone, Copy)]
enum Control {
    Switch,
    Choice(PaletteKind),
    Number { min: f64, max: f64, integer: bool },
    Text,
}

/// 1つの設定。`key` は `editor.font_size` のように `.` でつなぐ。
struct Item {
    category: Category,
    key: &'static str,
    control: Control,
}

const ITEMS: &[Item] = &[
    Item {
        category: Category::General,
        key: "theme",
        control: Control::Choice(PaletteKind::Themes),
    },
    Item {
        category: Category::General,
        key: "language",
        control: Control::Choice(PaletteKind::Languages),
    },
    Item {
        category: Category::General,
        key: "restore_session",
        control: Control::Switch,
    },
    Item {
        category: Category::Editor,
        key: "editor.font_family",
        control: Control::Text,
    },
    Item {
        category: Category::Editor,
        key: "editor.font_size",
        control: Control::Number {
            min: 8.0,
            max: 40.0,
            integer: false,
        },
    },
    Item {
        category: Category::Editor,
        key: "editor.tab_size",
        control: Control::Number {
            min: 1.0,
            max: 16.0,
            integer: true,
        },
    },
    Item {
        category: Category::Editor,
        key: "editor.hard_tabs",
        control: Control::Switch,
    },
    Item {
        category: Category::Editor,
        key: "editor.soft_wrap",
        control: Control::Switch,
    },
    Item {
        category: Category::Editor,
        key: "editor.line_numbers",
        control: Control::Switch,
    },
    Item {
        category: Category::Editor,
        key: "editor.indent_guides",
        control: Control::Switch,
    },
    Item {
        category: Category::Extensions,
        key: "check_extension_updates",
        control: Control::Switch,
    },
    Item {
        category: Category::Extensions,
        key: "extensions_registry",
        control: Control::Text,
    },
];

/// いまの設定を JSON で読む（`editor.x` は入れ子）。
fn current(key: &str, cx: &App) -> Value {
    let s = serde_json::to_value(settings::ActiveSettings::get(cx).clone()).unwrap_or_default();
    key.split('.').fold(s, |v, k| v.get(k).cloned().unwrap_or(Value::Null))
}

fn i18n_key(key: &str) -> String {
    format!("core.settings.item.{}", key.replace(['.', '_'], "-"))
}

pub struct SettingsPage {
    focus: FocusHandle,
    category: Category,
    search: Entity<InputState>,
    inputs: HashMap<&'static str, (Entity<InputState>, Subscription)>,
    errors: HashSet<&'static str>,
    _subscriptions: Vec<Subscription>,
}

impl EventEmitter<PanelEvent> for SettingsPage {}
impl EventEmitter<SettingsPageEvent> for SettingsPage {}

impl SettingsPage {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let search = cx.new(|cx| InputState::new(window, cx).placeholder(cx.t("core.settings.search")));
        let subs = vec![
            cx.subscribe(&search, |_, _, e: &InputEvent, cx| {
                if matches!(e, InputEvent::Change) {
                    cx.notify();
                }
            }),
            cx.observe_global::<settings::ActiveSettings>(|this, cx| {
                // ファイルを直接書き換えたときなどに、入力欄を今の値に戻す（入力中でなければ）。
                this.errors.clear();
                cx.notify();
            }),
        ];
        SettingsPage {
            focus: cx.focus_handle(),
            category: Category::General,
            search,
            inputs: HashMap::new(),
            errors: HashSet::new(),
            _subscriptions: subs,
        }
    }

    /// 値を書く。`editor.x` なら editor のオブジェクトを丸ごと書く。
    fn set(&mut self, key: &str, value: Value, cx: &mut Context<Self>) {
        match key.split_once('.') {
            Some((parent, child)) => {
                let mut obj = current(parent, cx);
                if let Some(map) = obj.as_object_mut() {
                    map.insert(child.to_string(), value);
                }
                cx.emit(SettingsPageEvent::Set(parent.to_string(), obj));
            }
            None => cx.emit(SettingsPageEvent::Set(key.to_string(), value)),
        }
    }

    fn input(&mut self, item: &Item, window: &mut Window, cx: &mut Context<Self>) -> Entity<InputState> {
        if let Some((i, _)) = self.inputs.get(item.key) {
            return i.clone();
        }
        let text = match current(item.key, cx) {
            Value::String(s) => s,
            Value::Number(n) => n.to_string(),
            _ => String::new(),
        };
        let input = cx.new(|cx| InputState::new(window, cx).default_value(text));
        let (key, control) = (item.key, item.control);
        let sub = cx.subscribe_in(&input, window, move |this, input, e: &InputEvent, _, cx| {
            if !matches!(e, InputEvent::PressEnter { .. } | InputEvent::Blur) {
                return;
            }
            let text = input.read(cx).value().trim().to_string();
            let value = match control {
                Control::Number { min, max, integer } => text
                    .parse::<f64>()
                    .ok()
                    .filter(|n| n.is_finite() && *n >= min && *n <= max && (!integer || n.fract() == 0.0))
                    .map(|n| if integer { json!(n as i64) } else { json!(n) }),
                // 空の一覧の場所は「公式」に戻す。
                Control::Text if key == "extensions_registry" && text.is_empty() => Some(Value::Null),
                Control::Text if text.is_empty() => None,
                Control::Text => Some(json!(text)),
                _ => None,
            };
            match value {
                Some(v) => {
                    this.errors.remove(key);
                    if current(key, cx) != v {
                        this.set(key, v, cx);
                    }
                }
                None => {
                    this.errors.insert(key);
                }
            }
            cx.notify();
        });
        self.inputs.insert(item.key, (input.clone(), sub));
        input
    }

    fn control(&mut self, item: &Item, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let key = item.key;
        match item.control {
            Control::Switch => {
                let on = current(key, cx).as_bool().unwrap_or(false);
                Switch::new(SharedString::from(key))
                    .checked(on)
                    .on_click(cx.listener(move |this, checked: &bool, _, cx| this.set(key, json!(*checked), cx)))
                    .into_any_element()
            }
            Control::Choice(kind) => {
                let focus = self.focus.clone();
                let options = choices(kind, cx);
                let selected = options
                    .iter()
                    .find(|(_, _, current)| *current)
                    .map(|(_, label, _)| label.clone())
                    .unwrap_or_default();
                let label = if kind == PaletteKind::Languages && current(key, cx).is_null() {
                    format!("{} ({selected})", cx.t("core.settings.system-language"))
                } else {
                    selected
                };
                Button::new(SharedString::from(key))
                    .small()
                    .outline()
                    .label(label)
                    .icon(IconName::ChevronDown)
                    .dropdown_menu(move |m: PopupMenu, _, _| {
                        let mut m = m.action_context(focus.clone());
                        for (value, label, checked) in &options {
                            m = m.menu_with_check(
                                label.clone(),
                                *checked,
                                Box::new(ChooseSetting {
                                    key: key.to_string(),
                                    value: json!(value).to_string(),
                                }),
                            );
                        }
                        m
                    })
                    .into_any_element()
            }
            Control::Number { .. } | Control::Text => {
                let input = self.input(item, window, cx);
                let wide = matches!(item.control, Control::Text);
                let invalid = self.errors.contains(key);
                v_flex()
                    .items_end()
                    .gap_0p5()
                    .child(
                        div()
                            .w(px(if wide { 280. } else { 96. }))
                            .child(editor::fields::input(&input, window, cx).small()),
                    )
                    .when(invalid, |d| {
                        d.child(
                            div()
                                .text_xs()
                                .text_color(cx.token(Token::HighlightError))
                                .child(cx.t("core.settings.invalid")),
                        )
                    })
                    .into_any_element()
            }
        }
    }

    fn row(&mut self, item: &Item, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let base = i18n_key(item.key);
        let control = self.control(item, window, cx);
        h_flex()
            .gap_6()
            .py_3()
            .items_start()
            .border_b_1()
            .border_color(cx.token(Token::Border))
            .child(
                v_flex()
                    .flex_1()
                    .gap_1()
                    .child(
                        h_flex()
                            .gap_2()
                            .child(
                                div()
                                    .text_sm()
                                    .font_weight(gpui_kit::FontWeight::MEDIUM)
                                    .text_color(cx.token(Token::Foreground))
                                    .child(cx.t(&base)),
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .font_family("JetBrains Mono")
                                    .text_color(cx.token(Token::MutedForeground))
                                    .child(item.key),
                            ),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(cx.token(Token::MutedForeground))
                            .child(cx.t(&format!("{base}-description"))),
                    ),
            )
            .child(control)
            .into_any_element()
    }
}

impl Focusable for SettingsPage {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}

impl BasePanel for SettingsPage {
    fn panel_name(&self) -> &'static str {
        "nucleus.settings"
    }
}

impl Panel for SettingsPage {
    fn title(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        h_flex()
            .gap_2()
            .child(
                Icon::new(IconName::Settings2)
                    .small()
                    .text_color(cx.token(Token::ListHighlightForeground)),
            )
            .child(cx.t("core.settings.title"))
    }

    fn inner_padding(&self, _: &App) -> bool {
        false
    }
}

impl Render for SettingsPage {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let q = self.search.read(cx).value().trim().to_lowercase();
        // 検索中は分類をまたいで探す。
        let items: Vec<&Item> = ITEMS
            .iter()
            .filter(|i| {
                if q.is_empty() {
                    return i.category == self.category;
                }
                let title = cx.t(&i18n_key(i.key)).to_lowercase();
                title.contains(&q) || i.key.contains(&q)
            })
            .collect();
        let rows: Vec<AnyElement> = items.into_iter().map(|i| self.row(i, window, cx)).collect();
        let current = self.category;
        let nav = Category::ALL.map(|c| {
            let selected = c == current && q.is_empty();
            h_flex()
                .id(SharedString::from(c.key()))
                .h(px(32.))
                .px_3()
                .gap_2()
                .rounded_md()
                .cursor_pointer()
                .text_sm()
                .text_color(if selected {
                    cx.token(Token::SidebarAccentForeground)
                } else {
                    cx.token(Token::SidebarForeground)
                })
                .when(selected, |d| d.bg(cx.token(Token::ListActiveBackground)))
                .when(!selected, |d| d.hover(|d| d.bg(cx.token(Token::ListHoverBackground))))
                .child(Icon::new(c.icon()).small())
                .child(cx.t(c.key()))
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.category = c;
                    cx.notify();
                }))
        });
        h_flex()
            .track_focus(&self.focus)
            .on_action(cx.listener(|this, a: &ChooseSetting, _, cx| {
                if let Ok(v) = serde_json::from_str::<Value>(&a.value) {
                    this.set(&a.key.clone(), v, cx);
                }
            }))
            .size_full()
            .items_start()
            .bg(cx.token(Token::Background))
            .child(
                v_flex()
                    .w(px(220.))
                    .h_full()
                    .p_3()
                    .gap_0p5()
                    .bg(cx.token(Token::SidebarBackground))
                    .border_r_1()
                    .border_color(cx.token(Token::Border))
                    .children(nav),
            )
            .child(
                v_flex()
                    .flex_1()
                    .h_full()
                    .min_w_0()
                    .child(
                        h_flex()
                            .px_6()
                            .pt_5()
                            .pb_3()
                            .gap_3()
                            .child(
                                div().flex_1().child(
                                    editor::fields::input(&self.search, window, cx).small().prefix(
                                        Icon::new(IconName::Search)
                                            .small()
                                            .text_color(cx.token(Token::MutedForeground)),
                                    ),
                                ),
                            )
                            .child(
                                Button::new("open-json")
                                    .small()
                                    .ghost()
                                    .icon(IconName::FileCode)
                                    .label(cx.t("core.settings.open-json"))
                                    .on_click(cx.listener(|_, _, _, cx| cx.emit(SettingsPageEvent::OpenJson))),
                            ),
                    )
                    .child(
                        div()
                            .id("settings-rows")
                            .flex_1()
                            .min_h_0()
                            .overflow_y_scroll()
                            .px_6()
                            .pb_6()
                            .child(v_flex().max_w(px(820.)).children(rows).when(q.is_empty(), |d| {
                                d.child(
                                    div()
                                        .pt_4()
                                        .text_xs()
                                        .text_color(cx.token(Token::MutedForeground))
                                        .child(cx.t("core.settings.more-in-json")),
                                )
                            })),
                    ),
            )
    }
}
