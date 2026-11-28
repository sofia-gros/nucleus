//! ワークスペース（メインウィンドウ）: TitleBar / DockArea / StatusBar と、ファイルを開く流れ。

mod assets;
mod close;
mod commands;
mod diff_views;
mod extension_bridge;
mod extension_view;
mod extensions_page;
mod finder;
mod language_support;
mod menus;
mod palette;
mod panels;
mod problems;
mod project_actions;
mod project_panel;
mod project_search;
mod prompt;
mod quick_diff;
mod recent;
mod search_panel;
mod session;
mod settings_page;
mod settings_sync;

use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
    sync::Arc,
};

use editor::{EditorEvent, EditorPanel, LoadError};
use gpui_kit::{
    App, AppContext as _, Context, Entity, FocusHandle, Focusable as _, InteractiveElement as _, IntoElement,
    KeyBinding, ParentElement, PathPromptOptions, Render, SharedString, StatefulInteractiveElement as _, Styled,
    Subscription, Task, WeakEntity, Window, WindowBounds, WindowOptions, actions,
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
        SelectTheme,
        SelectLanguage,
        OpenSettings,
        OpenWorkspaceSettings,
        NewTerminal,
        ToggleTerminal,
        ProjectSearch,
        OpenExtensions,
        OpenSettingsJson,
    ]
);

const CONTEXT: &str = "Workspace";

pub fn init(cx: &mut App) {
    editor::init(cx);
    prompt::init(cx);
    terminal::init(cx);
    cx.bind_keys([
        KeyBinding::new("ctrl-b", ToggleLeftDock, Some(CONTEXT)),
        KeyBinding::new("ctrl-j", ToggleBottomDock, Some(CONTEXT)),
        KeyBinding::new("ctrl-alt-b", ToggleRightDock, Some(CONTEXT)),
        KeyBinding::new("ctrl-o", OpenFolder, Some(CONTEXT)),
        KeyBinding::new("ctrl-shift-p", CommandPalette, Some(CONTEXT)),
        KeyBinding::new("f1", CommandPalette, Some(CONTEXT)),
        KeyBinding::new("ctrl-p", FindFile, Some(CONTEXT)),
        KeyBinding::new("ctrl-n", menus::NewFile, Some(CONTEXT)),
        KeyBinding::new("ctrl-shift-f", ProjectSearch, Some(CONTEXT)),
        KeyBinding::new("ctrl-shift-x", OpenExtensions, Some(CONTEXT)),
        KeyBinding::new("ctrl-w", CloseActiveEditor, Some(CONTEXT)),
        KeyBinding::new("ctrl-s", SaveActiveEditor, Some(CONTEXT)),
        KeyBinding::new("ctrl-,", OpenSettings, Some(CONTEXT)),
        KeyBinding::new("ctrl-`", ToggleTerminal, Some(CONTEXT)),
        KeyBinding::new("ctrl-shift-`", NewTerminal, Some(CONTEXT)),
        KeyBinding::new("ctrl-~", NewTerminal, Some(CONTEXT)),
        KeyBinding::new("ctrl-shift-t", NewTerminal, Some(CONTEXT)),
        KeyBinding::new("ctrl-q", Quit, None),
    ]);
    cx.on_action(|_: &Quit, cx| cx.quit());
    recent::init(cx);
}

/// メインウィンドウを開く。`paths` はフォルダかファイル（起動時の引数）。
/// 設定を読み込んで反映する（ウィンドウを開く前に呼ぶ）。読めなかった層のエラーを返す。
pub fn init_settings(cx: &mut App) -> Vec<String> {
    settings_sync::init(cx)
}

/// Extension の読み込み（段階1: データ系の権限）を始める。終わるとテーマや言語の対応付けを反映し直す。
pub fn start_extensions(cx: &mut App) {
    let disabled = settings::ActiveSettings::get(cx).disabled_extensions.clone();
    cx.observe_global::<extension_host::ExtensionStore>(settings_sync::on_extensions_changed)
        .detach();
    extension_host::start(disabled, cx);
}

/// メインウィンドウを開く。`paths` が空なら、設定に従って前回のセッションを復元する。
pub fn open_main_window(paths: Vec<PathBuf>, cx: &mut App) -> anyhow::Result<()> {
    let session = (paths.is_empty() && settings::ActiveSettings::get(cx).restore_session)
        .then(session::load)
        .flatten();
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
            if let Some(session) = session {
                ws.restore_session(session, window, cx);
            }
            ws.watch_settings(window, cx);
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
    extension_errors_shown: bool,
    /// 登録済みの Extension のショートカット（Extension ID, コマンド ID）。
    extension_keys: HashSet<(String, String)>,
    /// 置いた Extension のビュー（`<extension>/<view>`、エディタの横なら `@<番号>` つき）。
    ext_views_added: HashSet<String>,
    /// 右ドックに置いた Extension のビュー（並べ直すときに使う）。
    right_ext_panels: Vec<std::sync::Arc<dyn gpui_kit::base::dock::PanelView>>,
    /// エディタの横のビューをつないだエディタ（番号 → エディタ）。
    editor_slots: HashMap<u32, WeakEntity<EditorPanel>>,
    next_editor_slot: u32,
    /// 設定画面（開いていれば）。
    settings_page: Option<WeakEntity<settings_page::SettingsPage>>,
    /// Extension の管理画面（開いていれば）。
    extensions_page: Option<WeakEntity<extensions_page::ExtensionsPage>>,
    /// Extension が付けたプロジェクトツリーの印（Extension の ID → パス → 状態）。
    file_marks: HashMap<String, HashMap<PathBuf, extension_host::FileState>>,
    /// 開いている Diff のタブ。
    diff_views: diff_views::DiffViews,
    /// ドックに置いた Extension のビュー。
    extension_panels: Vec<WeakEntity<extension_view::ExtensionPanel>>,
    /// 右ドックの Core のパネル（Extension のビューを前に並べ直すときに使う）。
    outline_panel: Option<std::sync::Arc<dyn gpui_kit::base::dock::PanelView>>,
    terminals: Vec<WeakEntity<terminal::TerminalView>>,
    /// 開いているファイル（タブを開いた順。セッションの保存用）。
    open_order: Vec<PathBuf>,
    /// 読み込み中のファイル（同じファイルを二重に開かないため）。
    loading: HashSet<PathBuf>,
    /// セッション復元で最後にアクティブにするファイル。
    pending_active: Option<PathBuf>,
    /// 読み込み後に移動する位置（定義へのジャンプ、Problems から開いたとき）。
    pending_positions: HashMap<PathBuf, lsp_types::Position>,
    lsp: language_support::LanguageSupport,
    problems: Entity<problems::ProblemsPanel>,
    search: Entity<search_panel::SearchPanel>,
    last_saved_session: Option<session::Session>,
    settings_watcher: Option<project::DirWatcher>,
    _settings_task: Option<Task<()>>,
    _subscriptions: Vec<Subscription>,
}

impl Workspace {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let project = cx.new(ProjectPanel::new);
        let project_handle = panel_handle(project.clone());
        let panel = |kind, cx: &mut Context<DockArea>| panel_handle(cx.new(|cx| CorePanel::new(kind, cx)));
        let mut skin_handle = None;
        let mut first_terminal = None;
        let mut outline_handle = None;
        let search_panel = cx.new(|cx| search_panel::SearchPanel::new(window, cx));
        let problems_panel = cx.new(problems::ProblemsPanel::new);
        let problems_handle = panel_handle(problems_panel.clone());
        let dock_area = cx.new(|cx| {
            let skin = DockSkin::new(cx);
            skin_handle = Some(skin.clone());
            let mut area = DockArea::new("nucleus.workspace", Some(1), window, cx).with_renderer(skin);
            let search = panel_handle(search_panel.clone());
            let welcome = panel(PanelKind::Welcome, cx);
            let terminal_view = cx.new(|cx| terminal::TerminalView::new(None, cx));
            first_terminal = Some(terminal_view.downgrade());
            let terminal = panel_handle(terminal_view);
            let problems = problems_handle;
            let output = panel(PanelKind::Output, cx);
            let outline = panel(PanelKind::Outline, cx);
            outline_handle = Some(outline.clone());

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
                DockLayout::tabs().panel_view(outline, cx),
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
                ProjectPanelEvent::OpenTerminal(dir) => this.new_terminal_in(Some(dir.clone()), window, cx),
                ProjectPanelEvent::Renamed(old, new) => this.file_moved(old, Some(new), window, cx),
                ProjectPanelEvent::Deleted(old) => this.file_moved(old, None, window, cx),
            }),
        ];
        let mut subs = subs;
        subs.push(
            cx.observe_global_in::<extension_host::ExtensionStore>(window, |this, window, cx| {
                this.on_extensions_loaded(window, cx);
            }),
        );
        subs.push(cx.observe_global::<extensions_page::ExtensionUpdates>(|_, cx| cx.notify()));
        subs.push(cx.subscribe_in(&search_panel, window, |this, _, e, window, cx| {
            this.on_search_event(e, window, cx)
        }));
        subs.push(cx.subscribe_in(
            &problems_panel,
            window,
            |this, _, e: &problems::ProblemsEvent, window, cx| match e {
                problems::ProblemsEvent::Open(path, pos) => this.open_file_at(path.clone(), Some(*pos), window, cx),
            },
        ));
        let focus = cx.focus_handle();
        focus.focus(window, cx);
        cx.defer_in(window, |this: &mut Workspace, window, cx| {
            this.start_extension_bridge(window, cx)
        });
        subs.push(cx.observe_window_activation(window, |this, window, cx| {
            if window.is_window_active() {
                extension_host::WasmHost::broadcast(extension_host::WorkspaceEvent::WindowFocused, cx);
                this.refresh_all_diff_bases(cx);
            }
        }));
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
            extension_errors_shown: false,
            extension_keys: HashSet::new(),
            extension_panels: Vec::new(),
            diff_views: Vec::new(),
            file_marks: HashMap::new(),
            extensions_page: None,
            settings_page: None,
            ext_views_added: HashSet::new(),
            right_ext_panels: Vec::new(),
            editor_slots: HashMap::new(),
            next_editor_slot: 1,
            outline_panel: outline_handle,
            terminals: first_terminal.into_iter().collect(),
            open_order: Vec::new(),
            loading: HashSet::new(),
            pending_active: None,
            pending_positions: HashMap::new(),
            lsp: Default::default(),
            problems: problems_panel,
            search: search_panel,
            last_saved_session: None,
            settings_watcher: None,
            _settings_task: None,
            _subscriptions: subs,
        }
    }

    pub fn open_path(&mut self, path: PathBuf, window: &mut Window, cx: &mut Context<Self>) {
        let path = path.canonicalize().unwrap_or(path);
        if path.is_dir() {
            self.project.update(cx, |p, cx| p.set_root(path.clone(), cx));
            for t in self.terminals.iter().filter_map(WeakEntity::upgrade) {
                t.update(cx, |t, _| t.set_cwd(Some(path.clone())));
            }
            self.file_index = None;
            recent::add(&path, cx);
            self.sync_extension_root(cx);
            let root = self.project.read(cx).root();
            self.search.update(cx, |s, cx| s.set_root(root, cx));
            extension_host::WasmHost::broadcast(
                extension_host::WorkspaceEvent::RootChanged(Some(path.display().to_string())),
                cx,
            );
            self.watch_settings(window, cx);
            self.save_session(cx);
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
        if !self.loading.insert(path.clone()) {
            return;
        }
        let load = {
            let path = path.clone();
            cx.background_spawn(async move { editor::file::load(&path) })
        };
        cx.spawn_in(window, async move |this, cx| {
            let result = load.await;
            this.update_in(cx, |this, window, cx| {
                this.loading.remove(&path);
                if this.loading.is_empty() {
                    // 復元中に読み込んだファイルがすべてそろったら、前回アクティブだったタブを選ぶ。
                    if let Some(active) = this.pending_active.take()
                        && active != path
                    {
                        this.open_file(active, window, cx);
                    }
                }
                match result {
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
                EditorEvent::Edited => {
                    this.lsp_edited(cx);
                    let path = panel.read(cx).path().to_path_buf();
                    this.refresh_diff(&path, cx);
                }
                EditorEvent::Saved => {
                    let path = panel.read(cx).path().to_path_buf();
                    this.lsp_saved(&path);
                    this.refresh_diff_base(&panel, cx);
                    this.extension_file_saved(&path, cx);
                    cx.notify();
                }
                EditorEvent::Changed => {
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
        self.editors.insert(path.clone(), weak.clone());
        self.open_order.retain(|p| *p != path);
        self.open_order.push(path);
        self.active = Some(weak);
        self.dock_area.update(cx, |area, cx| {
            area.add_panel_view(panel_handle(panel.clone()), DockPlacement::Center, None, window, cx)
        });
        let focus = panel.read(cx).focus_handle(cx);
        focus.focus(window, cx);
        if let Some(pos) = self.pending_positions.remove(panel.read(cx).path()) {
            panel.update(cx, |e, cx| e.go_to(pos, window, cx));
        }
        self.attach_editor_views(&panel, window, cx);
        self.refresh_diff_base(&panel, cx);
        self.lsp_attach(panel, window, cx);
        self.save_session(cx);
        cx.notify();
    }

    /// ファイルを開き、指定した位置へ移動する。
    pub(crate) fn open_file_at(
        &mut self,
        path: PathBuf,
        position: Option<lsp_types::Position>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(existing) = self.editors.get(&path).and_then(WeakEntity::upgrade) {
            let id = PanelId::from(existing.entity_id());
            self.dock_area.update(cx, |area, cx| area.select_panel(id, window, cx));
            if let Some(pos) = position {
                existing.update(cx, |e, cx| e.go_to(pos, window, cx));
            }
            return;
        }
        if let Some(pos) = position {
            self.pending_positions.insert(path.clone(), pos);
        }
        self.open_file(path, window, cx);
    }

    /// Problems パネルを最新の診断で描き直す。
    pub(crate) fn refresh_problems(&mut self, cx: &mut Context<Self>) {
        let root = self.project.read(cx).root();
        let items: Vec<_> = self
            .lsp
            .diagnostics
            .iter()
            .map(|(p, d)| (p.clone(), d.clone()))
            .collect();
        self.problems.update(cx, |p, cx| p.set(root, items, cx));
    }

    fn active_editor(&self, cx: &App) -> Option<Entity<EditorPanel>> {
        let e = self.active.as_ref()?.upgrade()?;
        // 閉じたタブは DockArea から外れている。
        let id = PanelId::from(e.entity_id());
        self.dock_area.read(cx).panel(id).is_some().then_some(e)
    }

    /// Extension の読み込みが終わったとき: 読めなかったものを通知し、ステータスバーを更新する。
    fn on_extensions_loaded(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(store) = extension_host::ExtensionStore::get(cx) else {
            return;
        };
        if store.stage == extension_host::Stage::Ready {
            // あとから入れた Extension の分も足す（どれも、置いたものは置き直さない）。
            extension_bridge::bind_keys(&mut self.extension_keys, cx);
            self.add_extension_views(window, cx);
            let open: Vec<_> = self.editors.values().filter_map(WeakEntity::upgrade).collect();
            for e in &open {
                self.attach_editor_views(e, window, cx);
            }
        }
        let Some(store) = extension_host::ExtensionStore::get(cx) else {
            return;
        };
        if store.stage == extension_host::Stage::Ready && !self.extension_errors_shown {
            self.extension_errors_shown = true;
            self.check_extension_updates(window, cx);
            self.refresh_all_diff_bases(cx);
            // Extension の読み込みより先に開いていたファイルも、言語サーバーにつなぐ。
            let open: Vec<_> = self.editors.values().filter_map(WeakEntity::upgrade).collect();
            for e in open {
                self.lsp_attach(e, window, cx);
            }
            let Some(store) = extension_host::ExtensionStore::get(cx) else {
                return;
            };
            let failed: Vec<String> = store
                .failed
                .iter()
                .map(|f| format!("{}: {}", f.location, f.errors.join(" / ")))
                .collect();
            for f in failed {
                let msg = cx.t_args("core.notify.extension-failed", &[("error", f.into())]);
                window.push_notification(Notification::error(msg), cx);
            }
        }
        cx.notify();
    }

    fn new_terminal(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let cwd = self.project.read(cx).root();
        self.new_terminal_in(cwd, window, cx);
    }

    /// 名前を変えた・消したファイル（フォルダならその中も）を開いていたら、エディタを付け替える・閉じる。
    /// 未保存の変更があるものはそのまま残す（保存すると元の名前で書かれる）。
    fn file_moved(&mut self, old: &Path, new: Option<&Path>, window: &mut Window, cx: &mut Context<Self>) {
        let affected: Vec<(PathBuf, Entity<EditorPanel>)> = self
            .editors
            .iter()
            .filter(|(p, _)| p.starts_with(old))
            .filter_map(|(p, e)| Some((p.clone(), e.upgrade()?)))
            .collect();
        for (path, editor) in affected {
            if editor.read(cx).is_dirty() {
                continue;
            }
            self.remove_editor(editor, window, cx);
            if let (Some(new), Ok(rest)) = (new, path.strip_prefix(old)) {
                self.open_file(new.join(rest), window, cx);
            }
        }
    }

    fn new_terminal_in(&mut self, cwd: Option<PathBuf>, window: &mut Window, cx: &mut Context<Self>) {
        let view = cx.new(|cx| terminal::TerminalView::new(cwd, cx));
        self.terminals.retain(|t| t.upgrade().is_some());
        self.terminals.push(view.downgrade());
        self.dock_area.update(cx, |area, cx| {
            area.add_panel_view(panel_handle(view.clone()), DockPlacement::Bottom, None, window, cx);
            if !area.is_dock_open(DockPlacement::Bottom) {
                area.toggle_dock(DockPlacement::Bottom, window, cx);
            }
        });
        let focus = view.read(cx).focus_handle(cx);
        focus.focus(window, cx);
    }

    /// 下ドックを開いてターミナルにフォーカスする。開いていてターミナルにフォーカスがあれば閉じる。
    fn toggle_terminal(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.terminals.retain(|t| t.upgrade().is_some());
        let Some(view) = self.terminals.first().and_then(WeakEntity::upgrade) else {
            self.new_terminal(window, cx);
            return;
        };
        let focus = view.read(cx).focus_handle(cx);
        let open = self.dock_area.read(cx).is_dock_open(DockPlacement::Bottom);
        if open && focus.contains_focused(window, cx) {
            self.toggle(DockPlacement::Bottom, window, cx);
            if let Some(e) = self.active_editor(cx) {
                let f = e.read(cx).focus_handle(cx);
                f.focus(window, cx);
            } else {
                self.focus.focus(window, cx);
            }
            return;
        }
        let id = PanelId::from(view.entity_id());
        self.dock_area.update(cx, |area, cx| {
            if !area.is_dock_open(DockPlacement::Bottom) {
                area.toggle_dock(DockPlacement::Bottom, window, cx);
            }
            area.select_panel(id, window, cx);
        });
        focus.focus(window, cx);
    }

    fn toggle(&mut self, placement: DockPlacement, window: &mut Window, cx: &mut Context<Self>) {
        self.dock_area
            .update(cx, |area, cx| area.toggle_dock(placement, window, cx));
    }

    fn render_title_bar(&self, window: &Window, cx: &mut Context<Self>) -> impl IntoElement {
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
                    .child(self.render_menus(window, cx)),
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
        for (name, status) in &self.lsp.status {
            let (icon, label) = match status {
                language_support::ServerStatus::Starting => (IconName::RefreshCw, format!("{name}…")),
                language_support::ServerStatus::Ready => (IconName::Check, name.clone()),
                language_support::ServerStatus::Busy(m) => {
                    let m: String = m.chars().take(32).collect();
                    (IconName::RefreshCw, format!("{name}: {m}"))
                }
                language_support::ServerStatus::Failed => (IconName::CircleX, name.clone()),
            };
            bar = bar.right(item(Some(icon), label, cx));
        }
        // Extension の更新があれば、押すと拡張機能の画面を開く印を出す。
        let updates = cx
            .try_global::<extensions_page::ExtensionUpdates>()
            .map_or(0, |u| u.available.len());
        if updates > 0 {
            bar = bar.right(
                div()
                    .id("extension-updates")
                    .cursor_pointer()
                    .child(
                        h_flex()
                            .gap_1()
                            .px_2()
                            .text_xs()
                            .text_color(cx.token(Token::PrimaryBackground))
                            .child(Icon::new(IconName::Puzzle).xsmall())
                            .child(cx.t_args("core.status.extension-updates", &[("count", updates.into())])),
                    )
                    .on_click(cx.listener(|this, _, window, cx| this.open_extensions(window, cx))),
            );
        }
        let (errors, warnings) = self.lsp.counts();
        bar.left(item(
            Some(IconName::CircleX),
            cx.t_args("core.status.errors", &[("count", errors.into())]),
            cx,
        ))
        .left(item(
            Some(IconName::TriangleAlert),
            cx.t_args("core.status.warnings", &[("count", warnings.into())]),
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
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.sync_extension_root(cx);
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
            .on_action(cx.listener(|this, _: &ToggleTheme, window, cx| this.toggle_theme(window, cx)))
            .on_action(
                cx.listener(|this, _: &SelectTheme, window, cx| this.toggle_palette(PaletteKind::Themes, window, cx)),
            )
            .on_action(cx.listener(|this, _: &SelectLanguage, window, cx| {
                this.toggle_palette(PaletteKind::Languages, window, cx)
            }))
            .on_action(cx.listener(|this, _: &Quit, window, cx| this.quit(window, cx)))
            .on_action(cx.listener(|this, _: &OpenSettings, window, cx| this.open_settings_page(window, cx)))
            .on_action(cx.listener(|this, _: &OpenSettingsJson, window, cx| this.open_user_settings(window, cx)))
            .on_action(cx.listener(|this, _: &NewTerminal, window, cx| this.new_terminal(window, cx)))
            .on_action(cx.listener(|this, _: &ToggleTerminal, window, cx| this.toggle_terminal(window, cx)))
            .on_action(cx.listener(|this, _: &ProjectSearch, window, cx| this.open_project_search(window, cx)))
            .on_action(cx.listener(|this, _: &OpenExtensions, window, cx| this.open_extensions(window, cx)))
            .on_action(cx.listener(|this, _: &menus::NewFile, window, cx| this.new_file(window, cx)))
            .on_action(cx.listener(|this, _: &menus::CloneRepository, window, cx| this.clone_repository(window, cx)))
            .on_action(cx.listener(|this, a: &menus::OpenRecent, window, cx| this.open_recent(a, window, cx)))
            .on_action(cx.listener(|this, _: &menus::About, window, cx| this.show_about(window, cx)))
            .on_action(
                cx.listener(|this, _: &menus::ShowKeyboardShortcuts, window, cx| this.show_shortcuts(window, cx)),
            )
            .on_action(cx.listener(|this, _: &menus::OpenDocumentation, _, cx| this.open_documentation(cx)))
            .on_action(cx.listener(|this, _: &menus::ReportIssue, _, cx| this.report_issue(cx)))
            .on_action(
                cx.listener(|this, a: &extension_bridge::RunExtensionCommand, window, cx| {
                    this.run_extension_command(a, window, cx)
                }),
            )
            .on_action(
                cx.listener(|this, _: &OpenWorkspaceSettings, window, cx| this.open_workspace_settings(window, cx)),
            )
            .relative()
            .size_full()
            .bg(cx.token(Token::Background))
            .text_color(cx.token(Token::Foreground))
            .font_family(cx.global::<theme::ActiveTheme>().resolved.fonts.ui.clone())
            .child(self.render_title_bar(window, cx))
            .child(div().flex_1().min_h_0().child(self.dock_area.clone()))
            .child(self.render_status_bar(cx))
            .children(self.render_palette(cx))
    }
}
