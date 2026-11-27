//! ワークスペース（メインウィンドウ）: TitleBar / DockArea / StatusBar と、ファイルを開く流れ。

mod assets;
mod close;
mod commands;
mod finder;
mod palette;
mod panels;
mod project_panel;
mod prompt;

use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::Arc,
};

use editor::{EditorEvent, EditorPanel, LoadError};
use gpui_kit::{
    App, AppContext as _, Context, Entity, FocusHandle, Focusable as _, InteractiveElement as _, IntoElement,
    KeyBinding, ParentElement, PathPromptOptions, Render, SharedString, Styled, Subscription, WeakEntity, Window,
    WindowBounds, WindowOptions, actions,
    assets::IconName,
    component::{
        Icon, Sizable as _, TitleBar, WindowExt as _,
        button::{Button, ButtonVariants as _},
        dock::{DockArea, DockLayout, DockPlacement, DockSkin, PanelId, panel_handle},
        h_flex,
        notification::Notification,
        status_bar::StatusBar,
        v_flex,
    },
    div, px, size,
};
use i18n::Translate as _;
use theme::{Token, TokenColors as _};

pub use assets::{Assets, fonts};
use palette::{Palette, PaletteKind};
use panels::{CorePanel, PanelKind};
use project_panel::{ProjectPanel, ProjectPanelEvent};

actions!(
    workspace,
    [
        ToggleLeftDock,
        ToggleBottomDock,
        ToggleRightDock,
        OpenFolder,
        Quit,
        CommandPalette,
        FindFile,
        CloseActiveEditor,
        SaveActiveEditor,
        SearchInActiveEditor,
        ToggleTheme,
        UseEnglish,
        UseJapanese,
    ]
);

const CONTEXT: &str = "Workspace";

pub fn init(cx: &mut App) {
    editor::init(cx);
    prompt::init(cx);
    cx.bind_keys([
        KeyBinding::new("ctrl-b", ToggleLeftDock, Some(CONTEXT)),
        KeyBinding::new("ctrl-j", ToggleBottomDock, Some(CONTEXT)),
        KeyBinding::new("ctrl-alt-b", ToggleRightDock, Some(CONTEXT)),
        KeyBinding::new("ctrl-o", OpenFolder, Some(CONTEXT)),
        KeyBinding::new("ctrl-shift-p", CommandPalette, Some(CONTEXT)),
        KeyBinding::new("f1", CommandPalette, Some(CONTEXT)),
        KeyBinding::new("ctrl-p", FindFile, Some(CONTEXT)),
        KeyBinding::new("ctrl-w", CloseActiveEditor, Some(CONTEXT)),
        KeyBinding::new("ctrl-s", SaveActiveEditor, Some(CONTEXT)),
        KeyBinding::new("ctrl-q", Quit, None),
    ]);
    cx.on_action(|_: &Quit, cx| cx.quit());
}

/// メインウィンドウを開く。`paths` はフォルダかファイル（起動時の引数）。
pub fn open_main_window(paths: Vec<PathBuf>, cx: &mut App) -> anyhow::Result<()> {
    let options = WindowOptions {
        window_bounds: Some(WindowBounds::centered(None, size(px(1440.), px(900.)), cx)),
        window_min_size: Some(size(px(800.), px(500.))),
        ..TitleBar::window_options()
    };
    gpui_kit::open_window(options, cx, |window, cx| {
        cx.new(|cx| {
            let mut ws = Workspace::new(window, cx);
            for p in paths {
                ws.open_path(p, window, cx);
            }
            ws
        })
    })?;
    Ok(())
}

pub struct Workspace {
    focus: FocusHandle,
    dock_area: Entity<DockArea>,
    project: Entity<ProjectPanel>,
    editors: HashMap<PathBuf, WeakEntity<EditorPanel>>,
    active: Option<WeakEntity<EditorPanel>>,
    palette: Option<Palette>,
    file_index: Option<Arc<Vec<String>>>,
    indexing: bool,
    /// 未保存の確認に答えた後は、ウィンドウを閉じる要求をそのまま通す。
    close_confirmed: bool,
    _subscriptions: Vec<Subscription>,
}

impl Workspace {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let project = cx.new(ProjectPanel::new);
        let project_handle = panel_handle(project.clone());
        let panel = |kind, cx: &mut Context<DockArea>| panel_handle(cx.new(|cx| CorePanel::new(kind, cx)));
        let mut skin_handle = None;
        let dock_area = cx.new(|cx| {
            let skin = DockSkin::new(cx);
            skin_handle = Some(skin.clone());
            let mut area = DockArea::new("nucleus.workspace", Some(1), window, cx).with_renderer(skin);
            let search = panel(PanelKind::Search, cx);
            let welcome = panel(PanelKind::Welcome, cx);
            let terminal = panel(PanelKind::Terminal, cx);
            let problems = panel(PanelKind::Problems, cx);
            let output = panel(PanelKind::Output, cx);
            let git = panel(PanelKind::Git, cx);
            let outline = panel(PanelKind::Outline, cx);

            area.set_center(DockLayout::tabs().panel_view(welcome, cx), window, cx);
            area.set_dock(
                DockPlacement::Left,
                DockLayout::tabs().panel_view(project_handle, cx).panel_view(search, cx),
                window,
                cx,
            );
            area.set_dock(
                DockPlacement::Bottom,
                DockLayout::tabs()
                    .panel_view(terminal, cx)
                    .panel_view(problems, cx)
                    .panel_view(output, cx),
                window,
                cx,
            );
            area.set_dock(
                DockPlacement::Right,
                DockLayout::tabs().panel_view(git, cx).panel_view(outline, cx),
                window,
                cx,
            );
            area.set_dock_size(DockPlacement::Left, px(280.), window, cx);
            area.set_dock_size(DockPlacement::Bottom, px(240.), window, cx);
            area.set_dock_size(DockPlacement::Right, px(320.), window, cx);
            // デザイン（02 Editor — main）と同じく、右ドックは閉じた状態で始める。
            area.toggle_dock(DockPlacement::Right, window, cx);
            area
        });
        // DockArea の作成後に設定する（作成中に呼ぶと DockArea の二重更新になる）。
        if let Some(skin) = skin_handle {
            skin.set_close_button_visible(true, cx);
        }
        let subs = vec![
            cx.subscribe_in(&project, window, |this, _, event, window, cx| match event {
                ProjectPanelEvent::OpenFile(path) => this.open_file(path.clone(), window, cx),
                ProjectPanelEvent::OpenFolderRequested => this.prompt_open_folder(window, cx),
            }),
        ];
        let focus = cx.focus_handle();
        focus.focus(window, cx);
        let this = cx.entity().downgrade();
        window.on_window_should_close(cx, move |window, cx| {
            this.update(cx, |ws, cx| ws.should_close_window(window, cx))
                .unwrap_or(true)
        });
        Workspace {
            focus,
            dock_area,
            project,
            editors: HashMap::new(),
            active: None,
            palette: None,
            file_index: None,
            indexing: false,
            close_confirmed: false,
            _subscriptions: subs,
        }
    }

    pub fn open_path(&mut self, path: PathBuf, window: &mut Window, cx: &mut Context<Self>) {
        let path = path.canonicalize().unwrap_or(path);
        if path.is_dir() {
            self.project.update(cx, |p, cx| p.set_root(path, cx));
        } else {
            if self.project.read(cx).root().is_none()
                && let Some(parent) = path.parent()
            {
                let parent = parent.to_path_buf();
                self.project.update(cx, |p, cx| p.set_root(parent, cx));
            }
            self.open_file(path, window, cx);
        }
        cx.notify();
    }

    fn prompt_open_folder(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let rx = cx.prompt_for_paths(PathPromptOptions {
            files: false,
            directories: true,
            multiple: false,
            prompt: Some(cx.t("core.dialog.open-folder")),
        });
        cx.spawn_in(window, async move |this, cx| {
            let Ok(Ok(Some(mut paths))) = rx.await else { return };
            let Some(path) = paths.pop() else { return };
            this.update_in(cx, |this, window, cx| this.open_path(path, window, cx))
                .ok();
        })
        .detach();
    }

    /// ファイルを開く。開いていればそのタブに切り替える。読み込みはバックグラウンド。
    fn open_file(&mut self, path: PathBuf, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(existing) = self.editors.get(&path).and_then(WeakEntity::upgrade) {
            let id = PanelId::from(existing.entity_id());
            self.dock_area.update(cx, |area, cx| area.select_panel(id, window, cx));
            let focus = existing.read(cx).focus_handle(cx);
            focus.focus(window, cx);
            return;
        }
        let load = {
            let path = path.clone();
            cx.background_spawn(async move { editor::file::load(&path) })
        };
        cx.spawn_in(window, async move |this, cx| {
            let result = load.await;
            this.update_in(cx, |this, window, cx| match result {
                Ok(loaded) => this.add_editor(path, loaded, window, cx),
                Err(e) => {
                    let file = display_name(&path);
                    let msg = match e {
                        LoadError::Binary => cx.t_args("core.notify.binary", &[("file", file.into())]),
                        LoadError::TooLarge(_) => cx.t_args("core.notify.too-large", &[("file", file.into())]),
                        LoadError::Io(e) => cx.t_args(
                            "core.notify.open-failed",
                            &[("file", file.into()), ("error", e.to_string().into())],
                        ),
                    };
                    window.push_notification(Notification::error(msg), cx);
                }
            })
            .ok();
        })
        .detach();
    }

    fn add_editor(&mut self, path: PathBuf, loaded: editor::Loaded, window: &mut Window, cx: &mut Context<Self>) {
        let panel = cx.new(|cx| EditorPanel::new(path.clone(), loaded, window, cx));
        let weak = panel.downgrade();
        self._subscriptions.push(
            cx.subscribe_in(&panel, window, move |this, panel, event, window, cx| match event {
                EditorEvent::Activated => {
                    this.active = Some(panel.downgrade());
                    cx.notify();
                }
                EditorEvent::Changed | EditorEvent::Saved => {
                    if this.active.as_ref().is_some_and(|a| a.entity_id() == panel.entity_id()) {
                        cx.notify();
                    }
                }
                EditorEvent::SaveFailed(error) => {
                    let file = display_name(panel.read(cx).path());
                    let msg = cx.t_args(
                        "core.notify.save-failed",
                        &[("file", file.into()), ("error", error.clone().into())],
                    );
                    window.push_notification(Notification::error(msg), cx);
                }
            }),
        );
        self.editors.insert(path, weak.clone());
        self.active = Some(weak);
        self.dock_area.update(cx, |area, cx| {
            area.add_panel_view(panel_handle(panel.clone()), DockPlacement::Center, None, window, cx)
        });
        let focus = panel.read(cx).focus_handle(cx);
        focus.focus(window, cx);
        cx.notify();
    }

    fn active_editor(&self, cx: &App) -> Option<Entity<EditorPanel>> {
        let e = self.active.as_ref()?.upgrade()?;
        // 閉じたタブは DockArea から外れている。
        let id = PanelId::from(e.entity_id());
        self.dock_area.read(cx).panel(id).is_some().then_some(e)
    }

    fn toggle(&mut self, placement: DockPlacement, window: &mut Window, cx: &mut Context<Self>) {
        self.dock_area
            .update(cx, |area, cx| area.toggle_dock(placement, window, cx));
    }

    fn render_title_bar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let menus = ["file", "edit", "view", "selection", "find", "extensions", "help"];
        let dock_button =
            |id: &'static str, icon: IconName, key: &str, placement: DockPlacement, cx: &mut Context<Self>| {
                Button::new(id)
                    .ghost()
                    .small()
                    .icon(Icon::new(icon).text_color(cx.token(Token::MutedForeground)))
                    .tooltip(cx.t(key))
                    .on_click(cx.listener(move |this, _, window, cx| this.toggle(placement, window, cx)))
            };
        let project_name = self.project.read(cx).root().map(|r| display_name(&r));
        let title: SharedString = match (self.active_editor(cx), project_name) {
            (Some(e), Some(p)) => cx.t_args(
                "core.window-title",
                &[
                    ("file", e.read(cx).file_name().to_string().into()),
                    ("project", p.into()),
                ],
            ),
            (None, Some(p)) => p.into(),
            (Some(e), None) => e.read(cx).file_name(),
            (None, None) => cx.t("core.window-title-empty"),
        };
        TitleBar::new()
            .child(
                h_flex()
                    .gap_0p5()
                    .child(
                        div().pr_2().child(
                            Icon::new(IconName::Atom)
                                .size(px(18.))
                                .text_color(cx.token(Token::PrimaryBackground)),
                        ),
                    )
                    .children(menus.map(|m| Button::new(m).ghost().small().label(cx.t(&format!("core.menu.{m}"))))),
            )
            .child(
                div()
                    .flex_1()
                    .flex()
                    .justify_center()
                    .text_sm()
                    .text_color(cx.token(Token::MutedForeground))
                    .child(title),
            )
            .child(
                h_flex()
                    .gap_1()
                    .pr_2()
                    .child(dock_button(
                        "toggle-left",
                        IconName::PanelLeft,
                        "core.dock.toggle-left",
                        DockPlacement::Left,
                        cx,
                    ))
                    .child(dock_button(
                        "toggle-bottom",
                        IconName::PanelBottom,
                        "core.dock.toggle-bottom",
                        DockPlacement::Bottom,
                        cx,
                    ))
                    .child(dock_button(
                        "toggle-right",
                        IconName::PanelRight,
                        "core.dock.toggle-right",
                        DockPlacement::Right,
                        cx,
                    )),
            )
    }

    fn render_status_bar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        fn item(icon: Option<IconName>, label: impl Into<SharedString>, cx: &App) -> impl IntoElement {
            h_flex()
                .gap_1()
                .px_2()
                .text_xs()
                .text_color(cx.token(Token::MutedForeground))
                .children(icon.map(|i| Icon::new(i).xsmall().text_color(cx.token(Token::MutedForeground))))
                .child(label.into())
        }
        let mut bar = StatusBar::new();
        if let Some(editor) = self.active_editor(cx) {
            let e = editor.read(cx);
            let root = self.project.read(cx).root();
            let rel = root
                .as_deref()
                .and_then(|r| e.path().strip_prefix(r).ok())
                .unwrap_or(e.path())
                .display()
                .to_string();
            let pos = e.cursor(cx);
            let cursor = cx.t_args(
                "core.status.cursor",
                &[("line", (pos.line + 1).into()), ("column", (pos.character + 1).into())],
            );
            let language = match e.language().as_ref() {
                "text" => cx.t("core.status.language-text"),
                l => language_label(l).into(),
            };
            bar = bar
                .left(item(None, rel, cx))
                .left(item(None, cursor, cx))
                .right(item(None, e.line_ending().label(), cx))
                .right(item(None, "UTF-8", cx))
                .right(item(None, language, cx));
        }
        bar.left(item(
            Some(IconName::CircleX),
            cx.t_args("core.status.errors", &[("count", 0.into())]),
            cx,
        ))
        .left(item(
            Some(IconName::TriangleAlert),
            cx.t_args("core.status.warnings", &[("count", 0.into())]),
            cx,
        ))
        .right(item(Some(IconName::Bell), "", cx))
    }
}

fn display_name(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.display().to_string())
}

fn language_label(name: &str) -> String {
    match name {
        "javascript" => "JavaScript".into(),
        "typescript" => "TypeScript".into(),
        "tsx" => "TSX".into(),
        "toml" => "TOML".into(),
        "yaml" => "YAML".into(),
        "json" => "JSON".into(),
        "html" => "HTML".into(),
        "css" => "CSS".into(),
        "cpp" => "C++".into(),
        "c" => "C".into(),
        other => {
            let mut c = other.chars();
            c.next()
                .map(|f| f.to_uppercase().collect::<String>() + c.as_str())
                .unwrap_or_default()
        }
    }
}

impl Render for Workspace {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .key_context(CONTEXT)
            .track_focus(&self.focus)
            .on_action(cx.listener(|this, _: &ToggleLeftDock, window, cx| this.toggle(DockPlacement::Left, window, cx)))
            .on_action(
                cx.listener(|this, _: &ToggleBottomDock, window, cx| this.toggle(DockPlacement::Bottom, window, cx)),
            )
            .on_action(
                cx.listener(|this, _: &ToggleRightDock, window, cx| this.toggle(DockPlacement::Right, window, cx)),
            )
            .on_action(cx.listener(|this, _: &OpenFolder, window, cx| this.prompt_open_folder(window, cx)))
            .on_action(cx.listener(|this, _: &CommandPalette, window, cx| {
                this.toggle_palette(PaletteKind::Commands, window, cx)
            }))
            .on_action(
                cx.listener(|this, _: &FindFile, window, cx| this.toggle_palette(PaletteKind::Files, window, cx)),
            )
            .on_action(cx.listener(|this, _: &CloseActiveEditor, window, cx| this.close_active_editor(window, cx)))
            .on_action(cx.listener(|this, _: &SaveActiveEditor, _, cx| {
                if let Some(e) = this.active_editor(cx) {
                    e.update(cx, |e, cx| e.save(cx)).detach();
                }
            }))
            .on_action(cx.listener(|this, _: &SearchInActiveEditor, window, cx| {
                if let Some(e) = this.active_editor(cx) {
                    e.update(cx, |e, cx| e.open_search(window, cx));
                }
            }))
            .on_action(cx.listener(|this, _: &ToggleTheme, _, cx| this.toggle_theme(cx)))
            .on_action(cx.listener(|_, _: &UseEnglish, _, cx| i18n::set_language(i18n::en(), cx)))
            .on_action(
                cx.listener(|_, _: &UseJapanese, _, cx| i18n::set_language("ja".parse().expect("valid tag"), cx)),
            )
            .on_action(cx.listener(|this, _: &Quit, window, cx| this.quit(window, cx)))
            .relative()
            .size_full()
            .bg(cx.token(Token::Background))
            .text_color(cx.token(Token::Foreground))
            .font_family(cx.global::<theme::ActiveTheme>().resolved.fonts.ui.clone())
            .child(self.render_title_bar(cx))
            .child(div().flex_1().min_h_0().child(self.dock_area.clone()))
            .child(self.render_status_bar(cx))
            .children(self.render_palette(cx))
    }
}
