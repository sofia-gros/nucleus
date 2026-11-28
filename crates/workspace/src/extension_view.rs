//! Extension のビュー（ui@1）をドックのパネルとして描く。
//!
//! Extension は要素の木（平らな配列）を返すだけで、ここで gpui-kit の部品に置き換える。
//! 色は役割（tone）から Token に、寸法は段階値から決まった値に変える。
//! 入力欄の中身と見出しの開閉は Nucleus 側で持ち、Extension には変化だけを知らせる。

use std::{cell::Cell, collections::HashMap, ops::Range, rc::Rc};

use gpui_kit::component::input::EditorState;
use gpui_kit::{ContentMask, TextAlign, TextRun, font};

use gpui_kit::{
    AnyElement, App, AppContext as _, Bounds, Context, ElementId, Entity, EventEmitter, FocusHandle, Focusable, Hsla,
    InteractiveElement as _, IntoElement, MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent, ParentElement,
    Pixels, Render, SharedString, StatefulInteractiveElement as _, Styled, Subscription, Task, Window, canvas,
    component::{
        Icon, Sizable as _, WindowExt as _,
        button::{Button, ButtonVariants as _},
        dock::{BasePanel, Panel, PanelEvent},
        h_flex,
        input::{Input, InputEvent, InputState, Textarea, TextareaState},
        notification::Notification,
        skeleton::Skeleton,
        v_flex,
    },
    div, fill, point,
    prelude::FluentBuilder as _,
    px, size,
};
use gpui_kit::{base::StyledExt as _, component::Disableable as _};
use i18n::Translate as _;
use theme::{Token, TokenColors as _};

use extension_host::{
    WasmHost,
    manifest::ViewContribution,
    ui::{
        ButtonProps, ButtonVariant, CanvasProps, Direction, InputProps, Kind, Node, PointerEvent, PointerKind, Size,
        TextLine, Tone, UiEvent,
    },
};

enum InputHandle {
    Single(Entity<InputState>),
    Multi(Entity<TextareaState>),
}

struct InputSlot {
    handle: InputHandle,
    /// Extension が前回返した revision（これが変わったときだけ入力欄を書き換える）。
    revision: u32,
    _sub: Subscription,
}

pub struct ExtensionPanel {
    pub extension: String,
    view: ViewContribution,
    focus: FocusHandle,
    nodes: Option<Vec<Node>>,
    /// canvas に足す文字（canvas の key ごと、1.1.0 の `canvas-text`）。
    texts: HashMap<String, Vec<TextLine>>,
    /// この Extension の設定（変わったら描き直す）。
    last_settings: Option<serde_json::Map<String, serde_json::Value>>,
    /// 前回描いた Extension（更新して起動し直したら描き直す）。
    running: Option<std::sync::Weak<extension_host::runtime::WasmExtension>>,
    error: Option<String>,
    inputs: HashMap<String, InputSlot>,
    collapsed: HashMap<String, bool>,
    rendering: bool,
    /// canvas の上でマウスを押している。
    dragging: bool,
    /// 描いている間にもう一度描く必要が出た。
    again: bool,
    /// `render` に渡す名前（エディタの横のビューなら `<view>@<editor>`）。
    render_key: String,
    /// エディタの横のビューのとき、そのエディタ。
    editor: Option<EditorSlot>,
    _render_task: Option<Task<()>>,
    _subscriptions: Vec<Subscription>,
}

struct EditorSlot {
    state: Entity<EditorState>,
    version: Rc<Cell<u64>>,
    /// 前回描いたときの（内容の版, 見えている行, 高さ）。
    last: (u64, Option<Range<usize>>, i32),
}

impl ExtensionPanel {
    pub fn new(extension: String, view: ViewContribution, _window: &mut Window, cx: &mut Context<Self>) -> Self {
        let mut this = ExtensionPanel {
            extension,
            view,
            focus: cx.focus_handle(),
            nodes: None,
            texts: HashMap::new(),
            last_settings: None,
            running: None,
            error: None,
            inputs: HashMap::new(),
            collapsed: HashMap::new(),
            rendering: false,
            dragging: false,
            again: false,
            render_key: String::new(),
            editor: None,
            _render_task: None,
            _subscriptions: Vec::new(),
        };
        this.render_key = this.view.id.clone();
        this.last_settings = settings::ActiveSettings::get(cx)
            .extension_settings
            .get(&this.extension)
            .cloned();
        // Extension の起動が終わったら最初の描画をする。
        this._subscriptions.push(cx.observe_global::<WasmHost>(|this, cx| {
            // 起動が終わったとき、更新で起動し直したときに描く。
            let now = WasmHost::get(&this.extension, cx);
            let same = match (&this.running, &now) {
                (Some(old), Some(now)) => std::ptr::eq(old.as_ptr(), std::sync::Arc::as_ptr(now)),
                (None, None) => true,
                _ => false,
            };
            if (!same || this.nodes.is_none()) && !this.rendering {
                this.refresh(cx);
            }
        }));
        // Extension の設定が変わったら描き直す。
        this._subscriptions.push(cx.observe_global::<settings::ActiveSettings>(|this, cx| {
            crate::extension_bridge::sync_extension_settings(cx);
            let mine = settings::ActiveSettings::get(cx).extension_settings.get(&this.extension).cloned();
            if mine != this.last_settings {
                this.last_settings = mine;
                this.refresh(cx);
            }
        }));
        this.refresh(cx);
        this
    }

    /// エディタの横に置くビュー（`location: editor`）。エディタの内容や見えている範囲が変わったら描き直す。
    pub fn for_editor(
        extension: String,
        view: ViewContribution,
        editor_id: u32,
        state: Entity<EditorState>,
        version: Rc<Cell<u64>>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let mut this = Self::new(extension, view, window, cx);
        this.render_key = format!("{}@{editor_id}", this.view.id);
        this._subscriptions
            .push(cx.observe(&state, |this, _, cx| this.editor_changed(cx)));
        this.editor = Some(EditorSlot {
            state,
            version,
            last: (u64::MAX, None, 0),
        });
        this.nodes = None;
        this.refresh(cx);
        this
    }

    fn editor_changed(&mut self, cx: &mut Context<Self>) {
        let Some(slot) = &mut self.editor else { return };
        let s = slot.state.read(cx);
        let height = s.line_height().map_or(0, |lh| {
            (s.visible_row_range().map_or(0, |r| r.len()) as f32 * f32::from(lh)) as i32
        });
        let now = (slot.version.get(), s.visible_row_range(), height);
        if now != slot.last {
            slot.last = now;
            self.refresh(cx);
        }
    }

    /// Extension にビューの中身を描いてもらう。
    pub fn refresh(&mut self, cx: &mut Context<Self>) {
        if self.rendering {
            self.again = true;
            return;
        }
        let Some(ext) = WasmHost::get(&self.extension, cx) else {
            if let Some(e) = WasmHost::failure(&self.extension, cx) {
                self.error = Some(e);
                cx.notify();
            }
            return;
        };
        crate::extension_bridge::sync_extension_settings(cx);
        self.running = Some(std::sync::Arc::downgrade(&ext));
        self.rendering = true;
        let rx = ext.render(&self.render_key);
        self._render_task = Some(cx.spawn(async move |this, cx| {
            let result = rx.await.unwrap_or_else(|_| Err("Extension が止まった".into()));
            this.update(cx, |this, cx| {
                this.rendering = false;
                match result {
                    Ok(rendered) => {
                        this.nodes = Some(rendered.nodes);
                        this.texts = rendered.texts;
                        this.error = None;
                    }
                    Err(e) => {
                        log::warn!("Extension `{}` のビュー `{}`: {e}", this.extension, this.view.id);
                        this.error = Some(e);
                    }
                }
                cx.notify();
                if std::mem::take(&mut this.again) {
                    this.refresh(cx);
                }
            })
            .ok();
        }));
    }

    fn send(&mut self, key: String, event: UiEvent, window: &mut Window, cx: &mut Context<Self>) {
        let Some(ext) = WasmHost::get(&self.extension, cx) else {
            return;
        };
        let rx = ext.ui_event(&self.render_key, &key, event);
        cx.spawn_in(window, async move |_, cx| {
            if let Ok(Err(e)) = rx.await {
                cx.update(|window, cx| window.push_notification(Notification::error(e), cx))
                    .ok();
            }
        })
        .detach();
    }

    /// 入力欄を用意する（同じ key なら使い回し、Extension が値を変えたときだけ書き換える）。
    fn input(&mut self, p: &InputProps, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let (key, placeholder, multiline) = (p.key.as_str(), p.placeholder.as_str(), p.multiline);
        let needs_new = match self.inputs.get(key) {
            Some(slot) => matches!(slot.handle, InputHandle::Multi(_)) != multiline,
            None => true,
        };
        if needs_new {
            let k = key.to_string();
            let on_event =
                move |this: &mut Self, e: &InputEvent, text: String, window: &mut Window, cx: &mut Context<Self>| {
                    match e {
                        InputEvent::Change => this.send(k.clone(), UiEvent::Change(text), window, cx),
                        InputEvent::PressEnter { secondary, .. } if !multiline || *secondary => {
                            this.send(k.clone(), UiEvent::Submit(text), window, cx)
                        }
                        _ => {}
                    }
                };
            let (handle, sub) = if multiline {
                let state = cx.new(|cx| {
                    TextareaState::new(window, cx)
                        .auto_grow(2, 6)
                        .placeholder(placeholder.to_string())
                });
                let sub = cx.subscribe_in(&state, window, move |this, s, e: &InputEvent, window, cx| {
                    let text = s.read(cx).value().to_string();
                    on_event(this, e, text, window, cx)
                });
                (InputHandle::Multi(state), sub)
            } else {
                let state = cx.new(|cx| InputState::new(window, cx).placeholder(placeholder.to_string()));
                let sub = cx.subscribe_in(&state, window, move |this, s, e: &InputEvent, window, cx| {
                    let text = s.read(cx).value().to_string();
                    on_event(this, e, text, window, cx)
                });
                (InputHandle::Single(state), sub)
            };
            self.inputs.insert(
                key.to_string(),
                InputSlot {
                    handle,
                    // 作ったばかりの欄には必ず value を入れる。
                    revision: p.revision.wrapping_add(1),
                    _sub: sub,
                },
            );
        }
        let slot = self.inputs.get_mut(key).expect("just inserted");
        if slot.revision != p.revision {
            slot.revision = p.revision;
            let v = p.value.clone();
            match &slot.handle {
                InputHandle::Single(s) => s.update(cx, |s, cx| s.set_value(v, window, cx)),
                InputHandle::Multi(s) => s.update(cx, |s, cx| s.set_value(v, window, cx)),
            }
        }
        match &slot.handle {
            InputHandle::Single(s) => Input::new(s).small().into_any_element(),
            InputHandle::Multi(s) => Textarea::new(s).small().into_any_element(),
        }
    }

    fn render_node(&mut self, ix: usize, nodes: &[Node], window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let Some(node) = nodes.get(ix) else {
            return div().into_any_element();
        };
        let children = |this: &mut Self, window: &mut Window, cx: &mut Context<Self>| -> Vec<AnyElement> {
            node.children
                .iter()
                .map(|&c| this.render_node(c as usize, nodes, window, cx))
                .collect()
        };
        match &node.kind {
            Kind::Stack(p) => {
                let kids = children(self, window, cx);
                let base = match p.direction {
                    Direction::Horizontal => h_flex().items_center(),
                    Direction::Vertical => v_flex(),
                };
                base.gap(step(p.gap))
                    .p(step(p.padding))
                    .min_w_0()
                    .when(p.grow, |s| s.flex_1().min_h_0())
                    .when(p.spread, |s| s.justify_between())
                    .children(kids)
                    .into_any_element()
            }
            Kind::Scroll => {
                let kids = children(self, window, cx);
                div()
                    .id(node_id(ix))
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .child(v_flex().children(kids))
                    .into_any_element()
            }
            Kind::Text(p) => div()
                .min_w_0()
                .text_color(tone(p.tone, cx))
                .map(|d| match p.size {
                    Size::None | Size::Xs => d.text_xs(),
                    Size::Sm => d.text_sm(),
                    Size::Md => d.text_base(),
                    Size::Lg => d.text_lg(),
                })
                .when(p.bold, |d| d.font_semibold())
                .when(p.mono, |d| d.font_family("JetBrains Mono"))
                .child(p.content.clone())
                .into_any_element(),
            Kind::Icon(p) => icon(&p.name).small().text_color(tone(p.tone, cx)).into_any_element(),
            Kind::Button(p) => self.button(p, ix, false, cx).into_any_element(),
            Kind::Input(p) => self.input(p, window, cx),
            Kind::ListItem(p) => {
                let key = p.key.clone();
                let group: SharedString = format!("ext-row-{ix}").into();
                let actions: Vec<AnyElement> = p
                    .actions
                    .iter()
                    .enumerate()
                    .map(|(i, b)| self.button(b, ix * 64 + i, true, cx).into_any_element())
                    .collect();
                h_flex()
                    .id(node_id(ix))
                    .group(group.clone())
                    .h(px(24.))
                    .px_2()
                    .gap_1p5()
                    .rounded_sm()
                    .cursor_pointer()
                    .text_sm()
                    .text_color(cx.token(Token::SidebarForeground))
                    .when(p.selected, |s| s.bg(cx.token(Token::ListActiveBackground)))
                    .hover(|s| s.bg(cx.token(Token::ListHoverBackground)))
                    .when_some(p.icon.as_ref(), |s, name| {
                        s.child(icon(name).small().text_color(tone(p.icon_tone, cx)))
                    })
                    .child(div().flex_shrink_0().child(p.label.clone()))
                    .when_some(p.detail.as_ref(), |s, d| {
                        s.child(
                            div()
                                .min_w_0()
                                .flex_1()
                                .truncate()
                                .text_xs()
                                .text_color(cx.token(Token::MutedForeground))
                                .child(d.clone()),
                        )
                    })
                    .when(p.detail.is_none(), |s| s.child(div().flex_1()))
                    .child(
                        h_flex()
                            .gap_0p5()
                            .invisible()
                            .group_hover(group, |s| s.visible())
                            .children(actions),
                    )
                    .when_some(p.badge.as_ref(), |s, b| {
                        s.child(
                            div()
                                .w(px(14.))
                                .text_xs()
                                .font_semibold()
                                .text_color(tone(p.badge_tone, cx))
                                .child(b.clone()),
                        )
                    })
                    .on_click(
                        cx.listener(move |this, _, window, cx| this.send(key.clone(), UiEvent::Click, window, cx)),
                    )
                    .into_any_element()
            }
            Kind::Section(p) => {
                let key = p.key.clone();
                let collapsed = *self.collapsed.entry(key.clone()).or_insert(p.collapsed);
                let actions: Vec<AnyElement> = p
                    .actions
                    .iter()
                    .enumerate()
                    .map(|(i, b)| self.button(b, ix * 64 + i, true, cx).into_any_element())
                    .collect();
                let kids = if collapsed {
                    Vec::new()
                } else {
                    children(self, window, cx)
                };
                v_flex()
                    .child(
                        h_flex()
                            .id(node_id(ix))
                            .h(px(28.))
                            .px_1()
                            .gap_1()
                            .cursor_pointer()
                            .text_xs()
                            .font_semibold()
                            .text_color(cx.token(Token::MutedForeground))
                            .child(
                                Icon::new(if collapsed {
                                    gpui_kit::assets::IconName::ChevronRight
                                } else {
                                    gpui_kit::assets::IconName::ChevronDown
                                })
                                .xsmall(),
                            )
                            .child(p.title.to_uppercase())
                            .when_some(p.count, |s, n| {
                                s.child(
                                    div()
                                        .px_1p5()
                                        .rounded_full()
                                        .bg(cx.token(Token::MutedBackground))
                                        .child(n.to_string()),
                                )
                            })
                            .child(div().flex_1())
                            .child(h_flex().gap_0p5().children(actions))
                            .on_click(cx.listener(move |this, _, window, cx| {
                                let c = this.collapsed.entry(key.clone()).or_default();
                                *c = !*c;
                                let open = !*c;
                                this.send(key.clone(), UiEvent::Toggle(open), window, cx);
                                cx.notify();
                            })),
                    )
                    .children(kids)
                    .into_any_element()
            }
            Kind::Divider => div().h(px(1.)).w_full().bg(cx.token(Token::Border)).into_any_element(),
            Kind::Skeleton => skeleton().into_any_element(),
            Kind::Canvas(p) => self.canvas(p, ix, cx),
        }
    }

    /// 四角形を描く。マウスの操作は canvas の左上からの位置で Extension に送る。
    fn canvas(&self, p: &CanvasProps, ix: usize, cx: &mut Context<Self>) -> AnyElement {
        let rects: Vec<(Bounds<Pixels>, Hsla)> = p
            .rects
            .iter()
            .map(|r| {
                let color = Token::from_key(&r.color).map_or(cx.token(Token::MutedForeground), |t| cx.token(t));
                (
                    Bounds::new(point(px(r.x), px(r.y)), size(px(r.width), px(r.height))),
                    color.opacity(r.alpha.clamp(0., 1.)),
                )
            })
            .collect();
        // 文字の行（1.1.0）: 行ごとに、色の違う部分を1つの行としてまとめて組む。
        let mono: SharedString = "JetBrains Mono".into();
        let texts: Vec<(gpui_kit::Point<Pixels>, Pixels, SharedString, Vec<TextRun>)> = self
            .texts
            .get(&p.key)
            .map(|lines| {
                lines
                    .iter()
                    .filter(|l| l.size > 0.5 && l.size < 64.)
                    .map(|l| {
                        let mut text = String::new();
                        let mut runs = Vec::new();
                        for s in &l.spans {
                            if s.text.is_empty() || s.text.contains('\n') {
                                continue;
                            }
                            let color = Token::from_key(&s.color)
                                .map_or(cx.token(Token::MutedForeground), |t| cx.token(t))
                                .opacity(l.alpha.clamp(0., 1.));
                            text.push_str(&s.text);
                            runs.push(TextRun {
                                len: s.text.len(),
                                font: font(mono.clone()),
                                color,
                                background_color: None,
                                underline: None,
                                strikethrough: None,
                            });
                        }
                        (point(px(l.x), px(l.y)), px(l.size), text.into(), runs)
                    })
                    .collect()
            })
            .unwrap_or_default();
        let area: Rc<Cell<Bounds<Pixels>>> = Rc::default();
        let paint_area = area.clone();
        let key = p.key.clone();
        let pointer = move |kind: PointerKind, pos: gpui_kit::Point<Pixels>, area: &Rc<Cell<Bounds<Pixels>>>| {
            let b = area.get();
            UiEvent::Pointer(PointerEvent {
                kind,
                x: f32::from(pos.x - b.origin.x),
                y: f32::from(pos.y - b.origin.y),
                height: f32::from(b.size.height),
            })
        };
        let (k1, k2, k3) = (key.clone(), key.clone(), key);
        let (a1, a2, a3) = (area.clone(), area.clone(), area);
        div()
            .id(node_id(ix))
            .h_full()
            .flex_shrink_0()
            .map(|d| if p.width > 0. { d.w(px(p.width)) } else { d.w_full() })
            .cursor_pointer()
            .child(
                canvas(
                    move |bounds, _, _| paint_area.set(bounds),
                    move |bounds, _, window, cx| {
                        for (r, color) in &rects {
                            let b = Bounds::new(bounds.origin + r.origin, r.size);
                            window.paint_quad(fill(b, *color));
                        }
                        window.with_content_mask(Some(ContentMask { bounds }), |window| {
                            for (origin, size, text, runs) in &texts {
                                if runs.is_empty() {
                                    continue;
                                }
                                let line = window.text_system().shape_line(text.clone(), *size, runs, None);
                                let _ = line.paint(
                                    bounds.origin + *origin,
                                    *size,
                                    TextAlign::Left,
                                    None,
                                    window,
                                    cx,
                                );
                            }
                        });
                    },
                )
                .size_full(),
            )
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |this, e: &MouseDownEvent, window, cx| {
                    this.dragging = true;
                    this.send(k1.clone(), pointer(PointerKind::Down, e.position, &a1), window, cx)
                }),
            )
            .on_mouse_move(cx.listener(move |this, e: &MouseMoveEvent, window, cx| {
                if this.dragging && e.pressed_button == Some(MouseButton::Left) {
                    this.send(k2.clone(), pointer(PointerKind::Drag, e.position, &a2), window, cx)
                }
            }))
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(move |this, e: &MouseUpEvent, window, cx| {
                    this.dragging = false;
                    this.send(k3.clone(), pointer(PointerKind::Up, e.position, &a3), window, cx)
                }),
            )
            .into_any_element()
    }

    fn button(&self, p: &ButtonProps, ix: usize, compact: bool, cx: &mut Context<Self>) -> Button {
        let key = p.key.clone();
        let mut b = Button::new(node_id(ix)).disabled(p.disabled);
        b = match p.style {
            ButtonVariant::Primary => b.primary(),
            ButtonVariant::Secondary => b.outline(),
            ButtonVariant::Ghost => b.ghost(),
            ButtonVariant::Danger => b.danger(),
        };
        b = if compact { b.xsmall() } else { b.small() };
        if let Some(name) = &p.icon {
            b = b.icon(icon(name));
        }
        if let Some(l) = &p.label {
            b = b.label(l.clone());
        }
        if let Some(t) = &p.tooltip {
            b = b.tooltip(t.clone());
        }
        b.on_click(cx.listener(move |this, _, window, cx| {
            cx.stop_propagation();
            this.send(key.clone(), UiEvent::Click, window, cx)
        }))
    }
}

fn node_id(ix: usize) -> ElementId {
    ElementId::NamedInteger("ext-node".into(), ix as u64)
}

fn step(s: Size) -> Pixels {
    px(match s {
        Size::None => 0.,
        Size::Xs => 4.,
        Size::Sm => 8.,
        Size::Md => 12.,
        Size::Lg => 16.,
    })
}

fn tone(t: Tone, cx: &App) -> gpui_kit::Hsla {
    cx.token(match t {
        Tone::Default => Token::SidebarForeground,
        Tone::Muted => Token::MutedForeground,
        Tone::Accent => Token::ListHighlightForeground,
        Tone::Success => Token::HighlightCreated,
        Tone::Warning => Token::HighlightModified,
        Tone::Danger => Token::HighlightDeleted,
        Tone::Info => Token::HighlightInfo,
    })
}

/// Lucide のアイコン（kebab-case の名前）。Nucleus が持っていない名前なら何も描かれない。
fn icon(name: &str) -> Icon {
    let ok = !name.is_empty()
        && name
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-');
    let name = if ok { name } else { "circle-help" };
    Icon::default().path(format!("icons/{name}.svg"))
}

fn skeleton() -> impl IntoElement {
    v_flex()
        .gap_2()
        .p_3()
        .child(Skeleton::new().h(px(14.)).w(px(140.)))
        .child(Skeleton::new().h(px(14.)).w_full())
        .child(Skeleton::new().h(px(14.)).w(px(200.)))
}

impl EventEmitter<PanelEvent> for ExtensionPanel {}

impl Focusable for ExtensionPanel {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}

impl BasePanel for ExtensionPanel {
    fn panel_name(&self) -> &'static str {
        "nucleus.extension-view"
    }

    fn closable(&self, _: &App) -> bool {
        false
    }
}

impl Panel for ExtensionPanel {
    fn title(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let title = crate::extension_bridge::command_title(&self.extension, &self.view.title, cx);
        h_flex()
            .gap_2()
            .when_some(self.view.icon.as_ref(), |s, name| {
                s.child(icon(name).small().text_color(cx.token(Token::ListHighlightForeground)))
            })
            .child(title)
    }

    fn inner_padding(&self, _: &App) -> bool {
        false
    }
}

impl Render for ExtensionPanel {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if self.editor.is_some() {
            // エディタの横: 幅は中身が決める。描けるまで（または失敗したら）何も出さない。
            let body = self.nodes.take().map(|nodes| {
                let el = self.render_node(0, &nodes, window, cx);
                self.nodes = Some(nodes);
                el
            });
            return div().h_full().flex().flex_row().children(body).into_any_element();
        }
        // 前回の中身があれば、エラーのときもそれを出し続ける。
        let body = if let Some(nodes) = self.nodes.take() {
            let el = self.render_node(0, &nodes, window, cx);
            self.nodes = Some(nodes);
            el
        } else if let Some(e) = self.error.clone() {
            v_flex()
                .p_3()
                .gap_2()
                .text_xs()
                .text_color(cx.token(Token::MutedForeground))
                .child(cx.t("core.extension.view-failed"))
                .child(div().text_color(cx.token(Token::HighlightError)).child(e))
                .into_any_element()
        } else {
            skeleton().into_any_element()
        };
        v_flex()
            .size_full()
            .bg(cx.token(Token::SidebarBackground))
            .track_focus(&self.focus)
            .child(body)
            .into_any_element()
    }
}
