//! エディタ（docs/ARCHITECTURE.md §7）。テキストの正本は gpui-kit の EditorState が持つ Rope だけ。

pub mod file;

use std::path::{Path, PathBuf};

use gpui_kit::{
    App, AppContext as _, Context, Entity, EventEmitter, FocusHandle, Focusable, InteractiveElement as _, IntoElement,
    KeyBinding, ParentElement, Render, SharedString, Styled, Subscription, Task, Window, actions,
    assets::IconName,
    component::{
        Icon, Sizable as _,
        dock::{BasePanel, Panel, PanelEvent},
        h_flex,
        input::{Editor, EditorState, InputEvent, Position, TabSize},
    },
    div, px,
};
use theme::{Token, TokenColors as _};

pub use file::{LineEnding, LoadError, Loaded};

actions!(editor, [Save]);

const CONTEXT: &str = "EditorPanel";

pub fn init(cx: &mut App) {
    cx.bind_keys([KeyBinding::new("ctrl-s", Save, Some(CONTEXT))]);
}

/// ワークスペースに伝えるイベント。
#[derive(Clone, Debug)]
pub enum EditorEvent {
    /// フォーカスを得た（アクティブなエディタになった）。
    Activated,
    /// 未保存かどうか、またはカーソルが変わった。
    Changed,
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
    _subscriptions: Vec<Subscription>,
}

impl EditorPanel {
    pub fn new(path: PathBuf, loaded: Loaded, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let language: SharedString = file::language_for(&path).into();
        let state = cx.new(|cx| {
            EditorState::new(window, cx)
                .language(language.clone())
                .line_number(true)
                .indent_guides(true)
                .searchable(true)
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

    pub fn is_saving(&self) -> bool {
        self.saving
    }

    /// ファイル内検索を開く。
    pub fn open_search(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let focus = self.state.read(cx).focus_handle(cx);
        focus.focus(window, cx);
        self.state.update(cx, |s, cx| s.open_search(false, cx));
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
            .size_full()
            .bg(cx.token(Token::HighlightEditorBackground))
            .child(Editor::new(&self.state).bordered(false).h_full())
    }
}
