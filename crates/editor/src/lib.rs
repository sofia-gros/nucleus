//! エディタ（docs/ARCHITECTURE.md §7）。テキストの正本は gpui-kit の EditorState が持つ Rope だけ。

pub mod file;
mod find_panel;

use find_panel::{FindNext, FindPanel, FindPanelEvent, FindPrevious};

use std::path::{Path, PathBuf};

use gpui_kit::{
    App, AppContext as _, Context, Entity, EventEmitter, FocusHandle, Focusable, InteractiveElement as _, IntoElement,
    KeyBinding, ParentElement, Render, SharedString, Styled, Subscription, Task, Window, actions,
    assets::IconName,
    component::{
        Icon, Sizable as _,
        dock::{BasePanel, Panel, PanelEvent},
        h_flex,
        input::{Editor, EditorState, GoToDefinition, InputEvent, Position, Replace, Search, TabSize},
    },
    div, px,
};
use theme::{Token, TokenColors as _};

pub use file::{LineEnding, LoadError, Loaded};

/// Extension（`language` 権限）が足した、ファイルと言語の対応（gpui の Global）。
#[derive(Clone, Debug, Default)]
pub struct LanguageOverrides {
    pub by_extension: std::collections::HashMap<String, String>,
    pub by_file_name: std::collections::HashMap<String, String>,
}

impl gpui_kit::Global for LanguageOverrides {}

/// ファイルの言語名。Extension の対応付けを優先し、なければ組み込みの対応を使う。
pub fn language_for(path: &Path, cx: &App) -> String {
    if let Some(o) = cx.try_global::<LanguageOverrides>() {
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if let Some(l) = o.by_file_name.get(name) {
            return l.clone();
        }
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
        if let Some(l) = o.by_extension.get(&ext) {
            return l.clone();
        }
    }
    file::language_for(path)
}

actions!(editor, [Save]);

const CONTEXT: &str = "EditorPanel";

pub fn init(cx: &mut App) {
    find_panel::init(cx);
    cx.bind_keys([
        KeyBinding::new("ctrl-s", Save, Some(CONTEXT)),
        KeyBinding::new("f12", GoToDefinition, Some(CONTEXT)),
    ]);
}

/// ワークスペースに伝えるイベント。
#[derive(Clone, Debug)]
pub enum EditorEvent {
    /// フォーカスを得た（アクティブなエディタになった）。
    Activated,
    /// 未保存かどうか、またはカーソルが変わった。
    Changed,
    /// テキストが編集された。
    Edited,
    Saved,
    SaveFailed(String),
}

pub struct EditorPanel {
    path: PathBuf,
    language: SharedString,
    line_ending: LineEnding,
    state: Entity<EditorState>,
    focus: FocusHandle,
    version: u64,
    saved_version: u64,
    saving: bool,
    find_panel: Option<Entity<FindPanel>>,
    /// テキストが変わるたびに増える番号（言語サーバーとの同期に使う。共有して読む）。
    edit_version: std::rc::Rc<std::cell::Cell<u64>>,
    /// 読み込み後に移動する位置（定義へのジャンプで別ファイルを開いたときなど）。
    _subscriptions: Vec<Subscription>,
}

impl EditorPanel {
    pub fn new(path: PathBuf, loaded: Loaded, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let language: SharedString = language_for(&path, cx).into();
        let state = cx.new(|cx| {
            EditorState::new(window, cx)
                .language(language.clone())
                .line_number(true)
                .indent_guides(true)
                // 組み込みの検索バーは使わない（Ctrl+F は EditorPanel の Atom 式パネルが受け取る）。
                .searchable(false)
                .tab_size(TabSize {
                    tab_size: 4,
                    hard_tabs: false,
                })
                .default_value(loaded.text)
        });
        let subs = vec![
            cx.subscribe(&state, |this, _, event: &InputEvent, cx| match event {
                InputEvent::Change => {
                    let was_dirty = this.is_dirty();
                    this.version += 1;
                    this.edit_version.set(this.edit_version.get() + 1);
                    cx.emit(EditorEvent::Edited);
                    if !was_dirty {
                        cx.notify();
                    }
                    cx.emit(EditorEvent::Changed);
                }
                InputEvent::Focus => cx.emit(EditorEvent::Activated),
                _ => {}
            }),
            // カーソル移動も含め、エディタの状態が変わったらステータスバーへ伝える。
            cx.observe(&state, |_, _, cx| cx.emit(EditorEvent::Changed)),
            // 設定が変わったら、開いているエディタにもすぐ反映する。
            cx.observe_global_in::<settings::ActiveSettings>(window, |this, window, cx| {
                this.apply_settings(window, cx)
            }),
        ];
        EditorPanel {
            path,
            language,
            line_ending: loaded.line_ending,
            state,
            focus: cx.focus_handle(),
            version: 0,
            saved_version: 0,
            saving: false,
            find_panel: None,
            edit_version: Default::default(),
            _subscriptions: subs,
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn file_name(&self) -> SharedString {
        self.path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default()
            .into()
    }

    pub fn is_dirty(&self) -> bool {
        self.version != self.saved_version
    }

    pub fn language(&self) -> &SharedString {
        &self.language
    }

    pub fn line_ending(&self) -> LineEnding {
        self.line_ending
    }

    pub fn cursor(&self, cx: &App) -> Position {
        self.state.read(cx).cursor_position()
    }

    /// 保存する。Rope のスナップショット（O(1)）をバックグラウンドで書き出す。
    /// 返り値のタスクは保存が終わると完了する（未保存の確認ダイアログから待つため）。
    pub fn save(&mut self, cx: &mut Context<Self>) -> Task<Result<(), String>> {
        let rope = self.state.read(cx).text().clone();
        let version = self.version;
        let path = self.path.clone();
        let line_ending = self.line_ending;
        self.saving = true;
        let write = cx.background_spawn(async move { file::save(&path, rope.chunks(), line_ending) });
        cx.spawn(async move |this, cx| {
            let result = write.await.map_err(|e| e.to_string());
            this.update(cx, |this, cx| {
                this.saving = false;
                match &result {
                    Ok(()) => {
                        this.saved_version = this.saved_version.max(version);
                        cx.emit(EditorEvent::Saved);
                    }
                    Err(e) => {
                        log::error!("{} を保存できなかった: {e}", this.path.display());
                        cx.emit(EditorEvent::SaveFailed(e.clone()));
                    }
                }
                cx.notify();
            })
            .ok();
            result
        })
    }

    fn apply_settings(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let s = settings::ActiveSettings::get(cx).editor.clone();
        self.state.update(cx, |state, cx| {
            state.set_tab_size(
                TabSize {
                    tab_size: s.tab_size,
                    hard_tabs: s.hard_tabs,
                },
                cx,
            );
            state.set_soft_wrap(s.soft_wrap, window, cx);
            state.set_line_number(s.line_numbers, window, cx);
            state.set_indent_guides(s.indent_guides, window, cx);
        });
    }

    pub fn edit_version(&self) -> std::rc::Rc<std::cell::Cell<u64>> {
        self.edit_version.clone()
    }

    pub fn state(&self) -> &Entity<EditorState> {
        &self.state
    }

    /// 言語サーバーの診断を表示する（波線と、ホバーでの内容）。
    pub fn set_diagnostics(&mut self, diagnostics: &[lsp_types::Diagnostic], cx: &mut Context<Self>) {
        self.state.update(cx, |s, cx| {
            if let Some(set) = s.diagnostics_mut() {
                set.clear();
                for d in diagnostics {
                    set.push(d.clone());
                }
            }
            cx.notify();
        });
    }

    /// 指定した位置（行・文字。0 始まり）へ移動してフォーカスする。
    pub fn go_to(&mut self, position: lsp_types::Position, window: &mut Window, cx: &mut Context<Self>) {
        self.state.update(cx, |s, cx| {
            s.set_cursor_position(position, window, cx);
            s.focus(window, cx);
        });
    }

    pub fn is_saving(&self) -> bool {
        self.saving
    }

    /// ファイル内検索（Atom 式の下部パネル）を開く。
    pub fn open_search(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.open_find(false, window, cx);
    }

    fn open_find(&mut self, replace: bool, window: &mut Window, cx: &mut Context<Self>) {
        let panel = match &self.find_panel {
            Some(p) => p.clone(),
            None => {
                let state = self.state.clone();
                let p = cx.new(|cx| FindPanel::new(state, window, cx));
                self._subscriptions
                    .push(cx.subscribe(&p, |this, _, e: &FindPanelEvent, cx| match e {
                        FindPanelEvent::Closed => {
                            this.find_panel = None;
                            cx.notify();
                        }
                    }));
                self.find_panel = Some(p.clone());
                p
            }
        };
        panel.update(cx, |p, cx| p.activate(replace, window, cx));
        cx.notify();
    }
}

impl EventEmitter<PanelEvent> for EditorPanel {}
impl EventEmitter<EditorEvent> for EditorPanel {}

impl Focusable for EditorPanel {
    fn focus_handle(&self, cx: &App) -> FocusHandle {
        self.state.read(cx).focus_handle(cx)
    }
}

impl BasePanel for EditorPanel {
    fn panel_name(&self) -> &'static str {
        "nucleus.editor"
    }

    /// Atom と同じく、未保存のタブには閉じるボタンの代わりに ● を出す。
    /// 未保存のまま閉じるときは Workspace の「タブを閉じる」で確認する。
    fn closable(&self, _: &App) -> bool {
        !self.is_dirty()
    }
}

impl Panel for EditorPanel {
    // tab_name を返さないと、タブバーは title（アイコンと未保存マーク）を描く。

    fn title(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        h_flex()
            .gap_2()
            .child(
                Icon::new(IconName::FileCode)
                    .small()
                    .text_color(cx.token(Token::ListHighlightForeground)),
            )
            .child(self.file_name())
            .when(self.is_dirty(), |d| {
                // Atom 式: 未保存は modified 色の丸で示す。
                d.child(div().size(px(8.)).rounded_full().bg(cx.token(Token::HighlightModified)))
            })
    }

    fn inner_padding(&self, _: &App) -> bool {
        false
    }
}

use gpui_kit::prelude::FluentBuilder as _;

impl Render for EditorPanel {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let _ = &self.focus;
        div()
            .key_context(CONTEXT)
            .on_action(cx.listener(|this, _: &Save, _, cx| this.save(cx).detach()))
            .on_action(cx.listener(|this, _: &Search, window, cx| this.open_find(false, window, cx)))
            .on_action(cx.listener(|this, _: &Replace, window, cx| this.open_find(true, window, cx)))
            .on_action(cx.listener(|this, _: &FindNext, window, cx| match &this.find_panel {
                Some(p) => p.update(cx, |p, cx| p.find_next(cx)),
                None => this.open_find(false, window, cx),
            }))
            .on_action(
                cx.listener(|this, _: &FindPrevious, window, cx| match &this.find_panel {
                    Some(p) => p.update(cx, |p, cx| p.find_previous(cx)),
                    None => this.open_find(false, window, cx),
                }),
            )
            .size_full()
            .flex()
            .flex_col()
            .bg(cx.token(Token::HighlightEditorBackground))
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .child(Editor::new(&self.state).bordered(false).h_full()),
            )
            .children(self.find_panel.clone())
    }
}
