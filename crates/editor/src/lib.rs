//! エディタ（docs/ARCHITECTURE.md §7）。テキストの正本は gpui-kit の EditorState が持つ Rope だけ。

pub mod diff;
pub mod diff_view;
pub mod fields;
pub mod file;
mod find_panel;
mod quick_diff;

pub use find_panel::{FindNext, FindPrevious};
use find_panel::{FindPanel, FindPanelEvent};

use std::path::{Path, PathBuf};

use std::{ops::Range, rc::Rc, time::Duration};

use gpui_kit::{
    AnyWindowHandle, App, AppContext as _, Context, Entity, EntityInputHandler as _, EventEmitter, FocusHandle,
    Focusable, InteractiveElement as _, IntoElement, KeyBinding, ParentElement, Render, SharedString, Styled,
    Subscription, Task, Window, actions,
    assets::IconName,
    component::{
        Icon, Sizable as _,
        dock::{BasePanel, Panel, PanelEvent},
        h_flex,
        input::{
            Editor, EditorState, GoToDefinition, GutterStyle, InputEvent, Position, Replace, RopeExt as _, Search,
            TabSize, WrappingIndent,
        },
    },
    div, px, relative,
};
use settings::{AutoSave, CursorBlinking, Eol, MultiCursorModifier, RenderWhitespace};
use theme::{Token, TokenColors as _};

pub use file::{LineEnding, LoadError, Loaded};
use remote::RemoteExt as _;

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

actions!(editor, [Save, AddNextOccurrence, SelectAllOccurrences]);

/// 保存の前の整形（`editor.format_on_save`）。言語サーバーを持つ Workspace が入れる。
/// 返すのは置き換え（バイトの範囲と新しい文字列）。言語サーバーがなければ None。
#[derive(Clone)]
#[allow(clippy::type_complexity)]
pub struct FormatOnSave(pub Rc<dyn Fn(&Path, &mut App) -> Option<Task<anyhow::Result<Vec<(Range<usize>, String)>>>>>);

impl gpui_kit::Global for FormatOnSave {}

const CONTEXT: &str = "EditorPanel";

pub fn init(cx: &mut App) {
    find_panel::init(cx);
    cx.bind_keys([
        KeyBinding::new("ctrl-s", Save, Some(CONTEXT)),
        KeyBinding::new("f12", GoToDefinition, Some(CONTEXT)),
        KeyBinding::new("ctrl-d", AddNextOccurrence, Some(CONTEXT)),
        KeyBinding::new("ctrl-shift-l", SelectAllOccurrences, Some(CONTEXT)),
        KeyBinding::new("cmd-d", AddNextOccurrence, Some(CONTEXT)),
        KeyBinding::new("cmd-shift-l", SelectAllOccurrences, Some(CONTEXT)),
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
    /// エディタの右側に置くビュー（Extension の MiniMap など）。
    side: Vec<gpui_kit::AnyView>,
    /// 行番号の横の変更の印。
    quick_diff: quick_diff::QuickDiff,
    /// 中身から推し量ったインデント（`editor.detect_indentation`）。設定より優先する。
    detected_indent: Option<TabSize>,
    /// 自動保存の待ち。
    auto_save: Option<Task<()>>,
    window: AnyWindowHandle,
    /// カーソルのある行（1 始まり。行番号の相対表示が読む）。
    cursor_line: Rc<std::cell::Cell<usize>>,
    _subscriptions: Vec<Subscription>,
}

impl EditorPanel {
    pub fn new(path: PathBuf, loaded: Loaded, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let language: SharedString = language_for(&path, cx).into();
        let detected_indent =
            file::detect_indentation(&loaded.text).map(|(tab_size, hard_tabs)| TabSize { tab_size, hard_tabs });
        // 改行のないファイル（新しいファイルなど）は、設定の改行コードにする。
        let line_ending = if loaded.text.contains('\n') {
            loaded.line_ending
        } else {
            match settings::ActiveSettings::get(cx).files_for(&language).eol {
                Eol::Lf => LineEnding::Lf,
                Eol::Crlf => LineEnding::Crlf,
                Eol::Auto if cfg!(windows) => LineEnding::Crlf,
                Eol::Auto => LineEnding::Lf,
            }
        };
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
                    this.quick_diff_changed(cx);
                    cx.emit(EditorEvent::Edited);
                    if !was_dirty {
                        cx.notify();
                    }
                    cx.emit(EditorEvent::Changed);
                    this.schedule_auto_save(cx);
                }
                InputEvent::Focus => cx.emit(EditorEvent::Activated),
                InputEvent::Blur if this.files_settings(cx).auto_save == AutoSave::OnFocusChange => {
                    this.save_if_dirty(cx);
                }
                _ => {}
            }),
            // カーソル移動も含め、エディタの状態が変わったらステータスバーへ伝える。
            cx.observe(&state, |this: &mut Self, state, cx| {
                this.cursor_line.set(state.read(cx).cursor_position().line as usize + 1);
                cx.emit(EditorEvent::Changed)
            }),
            // 設定が変わったら、開いているエディタにもすぐ反映する。
            cx.observe_global_in::<settings::ActiveSettings>(window, |this, window, cx| {
                this.apply_settings(window, cx)
            }),
        ];
        let mut this = EditorPanel {
            path,
            language,
            line_ending,
            state,
            focus: cx.focus_handle(),
            version: 0,
            saved_version: 0,
            saving: false,
            find_panel: None,
            edit_version: Default::default(),
            side: Vec::new(),
            quick_diff: Default::default(),
            detected_indent,
            auto_save: None,
            window: window.window_handle(),
            cursor_line: Rc::new(std::cell::Cell::new(1)),
            _subscriptions: subs,
        };
        this.apply_settings(window, cx);
        this
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

    /// 保存する。設定に従って整形・手直ししてから、Rope のスナップショット（O(1)）を書き出す。
    /// 返り値のタスクは保存が終わると完了する（未保存の確認ダイアログから待つため）。
    pub fn save(&mut self, cx: &mut Context<Self>) -> Task<Result<(), String>> {
        let (editor_s, files_s) = (self.editor_settings(cx), self.files_settings(cx));
        let format = editor_s
            .format_on_save
            .then(|| cx.try_global::<FormatOnSave>().cloned())
            .flatten()
            .and_then(|f| (f.0)(&self.path, cx));
        let fixes = file::SaveFixes {
            trim_trailing_whitespace: files_s.trim_trailing_whitespace,
            insert_final_newline: files_s.insert_final_newline,
            trim_final_newlines: files_s.trim_final_newlines,
        };
        self.saving = true;
        self.auto_save = None;
        let window = self.window;
        cx.spawn(async move |this, cx| {
            if let Some(format) = format {
                match format.await {
                    Ok(edits) if !edits.is_empty() => {
                        cx.update_window(window, |_, window, cx| {
                            this.update(cx, |this, cx| this.apply_edits(edits, window, cx)).ok();
                        })
                        .ok();
                    }
                    Ok(_) => {}
                    // 整形できなくても保存はする。
                    Err(e) => log::warn!("保存前の整形ができなかった: {e}"),
                }
            }
            cx.update_window(window, |_, window, cx| {
                this.update(cx, |this, cx| {
                    let text = this.state.read(cx).value().to_string();
                    let edits = file::save_fixes(&text, fixes);
                    if !edits.is_empty() {
                        this.apply_edits(edits, window, cx);
                    }
                })
                .ok();
            })
            .ok();
            let Ok((rope, version, path, line_ending, client)) = this.update(cx, |this, cx| {
                (
                    this.state.read(cx).text().clone(),
                    this.version,
                    this.path.clone(),
                    this.line_ending,
                    cx.remote(),
                )
            }) else {
                return Err("closed".into());
            };
            let bytes = cx
                .background_spawn(async move { file::encode(rope.chunks(), line_ending) })
                .await;
            let result = client.write_file(&path, bytes).await.map_err(|e| e.to_string());
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

    /// 未保存なら保存する（自動保存）。
    pub fn save_if_dirty(&mut self, cx: &mut Context<Self>) {
        if self.is_dirty() && !self.saving {
            self.save(cx).detach();
        }
    }

    /// `files.auto_save` が `after_delay` なら、最後の編集から少し待って保存する。
    fn schedule_auto_save(&mut self, cx: &mut Context<Self>) {
        let files = self.files_settings(cx);
        if files.auto_save != AutoSave::AfterDelay {
            return;
        }
        let delay = Duration::from_millis(files.auto_save_delay.max(100));
        self.auto_save = Some(cx.spawn(async move |this, cx| {
            cx.background_executor().timer(delay).await;
            this.update(cx, |this, cx| {
                this.auto_save = None;
                this.save_if_dirty(cx);
            })
            .ok();
        }));
    }

    /// テキストを置き換える（範囲はバイト。後ろのものから順に並べて渡す）。カーソルの行と列は保つ。
    pub fn apply_edits(&mut self, edits: Vec<(Range<usize>, String)>, window: &mut Window, cx: &mut Context<Self>) {
        self.state.update(cx, |state, cx| {
            let cursor = state.cursor_position();
            for (range, text) in edits {
                let rope = state.text();
                let start = rope.offset_to_offset_utf16(range.start.min(rope.len()));
                let end = rope.offset_to_offset_utf16(range.end.min(rope.len()));
                state.replace_text_in_range(Some(start..end), &text, window, cx);
            }
            state.set_cursor_position(cursor, window, cx);
        });
    }

    /// この言語で使うエディタの設定（`"[言語]"` の上書きを重ねたもの）。
    fn editor_settings(&self, cx: &App) -> settings::EditorSettings {
        settings::ActiveSettings::get(cx).editor_for(&self.language)
    }

    /// この言語で使うファイルの設定（`"[言語]"` の上書きを重ねたもの）。
    fn files_settings(&self, cx: &App) -> settings::FilesSettings {
        settings::ActiveSettings::get(cx).files_for(&self.language)
    }

    /// Ctrl+D: 選んでいる文字（なければカーソルの単語）の、次に出てくる所も選ぶ。
    fn add_next_occurrence(&mut self, cx: &mut Context<Self>) {
        self.state.update(cx, |state, cx| {
            let range = state.selected_range();
            let text = state.text().clone();
            if range.is_empty() {
                if let Some(word) = text.word_range(range.start) {
                    state.set_selected_range(word, cx);
                }
                return;
            }
            let needle = text.slice(range.clone()).to_string();
            let hay = text.to_string();
            let found = hay[range.end..]
                .find(&needle)
                .map(|i| range.end + i)
                .or_else(|| hay[..range.start].find(&needle));
            if let Some(start) = found {
                state.add_selection(start..start + needle.len(), cx);
            }
        });
    }

    /// Ctrl+Shift+L: 選んでいる文字（なければカーソルの単語）が出てくる所を、すべて選ぶ。
    fn select_all_occurrences(&mut self, cx: &mut Context<Self>) {
        self.state.update(cx, |state, cx| {
            let text = state.text().clone();
            let mut range = state.selected_range();
            if range.is_empty() {
                match text.word_range(range.start) {
                    Some(word) => range = word,
                    None => return,
                }
            }
            let needle = text.slice(range.clone()).to_string();
            let hay = text.to_string();
            state.set_selected_range(range.clone(), cx);
            for (start, _) in hay.match_indices(&needle) {
                if start != range.start {
                    state.add_selection(start..start + needle.len(), cx);
                }
            }
        });
    }

    fn apply_settings(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let s = self.editor_settings(cx);
        let tab = match self.detected_indent {
            Some(t) if s.detect_indentation => t,
            _ => TabSize {
                tab_size: s.tab_size,
                hard_tabs: s.hard_tabs,
            },
        };
        self.state.update(cx, |state, cx| {
            state.set_tab_size(tab, cx);
            state.set_soft_wrap(s.soft_wrap, window, cx);
            state.set_wrapping_indent(
                match s.wrapping_indent {
                    settings::WrappingIndent::None => WrappingIndent::None,
                    settings::WrappingIndent::Same => WrappingIndent::Same,
                },
                window,
                cx,
            );
            state.set_line_number(s.line_numbers, window, cx);
            state.set_indent_guides(s.indent_guides, window, cx);
            state.set_show_whitespaces(s.render_whitespace == RenderWhitespace::All, window, cx);
            state.set_folding(s.folding, window, cx);
            state.set_auto_close(s.auto_closing_brackets, window, cx);
            state.set_smart_indent(s.auto_indent, window, cx);
            state.set_scroll_beyond_last_line((!s.scroll_beyond_last_line).then_some(0), window, cx);
            state.set_cursor_surrounding_lines(
                (s.cursor_surrounding_lines > 0).then_some(s.cursor_surrounding_lines),
                window,
                cx,
            );
            state.set_active_line_only_when_focused(s.highlight_line_only_when_focused, window, cx);
            state.set_rulers(s.rulers.clone(), cx);
            state.set_multi_cursor_with_ctrl(s.multi_cursor_modifier == MultiCursorModifier::CtrlCmd);
        });
        let blink = match s.cursor_blinking {
            CursorBlinking::Blink => gpui_kit::component::input::CursorBlink::default(),
            CursorBlinking::Solid => gpui_kit::component::input::CursorBlink::steady(),
        };
        gpui_kit::component::Theme::global_mut(cx).cursor_blink = blink;
        cx.notify();
    }

    pub fn edit_version(&self) -> std::rc::Rc<std::cell::Cell<u64>> {
        self.edit_version.clone()
    }

    /// エディタの右側にビューを足す（Extension の `location: editor`）。
    pub fn add_side_view(&mut self, view: gpui_kit::AnyView, cx: &mut Context<Self>) {
        self.side.push(view);
        cx.notify();
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
        let s = self.editor_settings(cx);
        let line_height = s.line_height.clamp(1.0, 3.0);
        // editor.relative_line_numbers: 行番号はカーソルの行からの距離（カーソルの行は本来の番号）。
        let gutter = if s.relative_line_numbers {
            GutterStyle::default().with_relative_line_numbers(self.cursor_line.clone())
        } else {
            GutterStyle::default()
        };
        div()
            .key_context(CONTEXT)
            .on_action(cx.listener(|this, _: &Save, _, cx| this.save(cx).detach()))
            .on_action(cx.listener(|this, _: &AddNextOccurrence, _, cx| this.add_next_occurrence(cx)))
            .on_action(cx.listener(|this, _: &SelectAllOccurrences, _, cx| this.select_all_occurrences(cx)))
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
                    .flex()
                    .flex_row()
                    .child(
                        div().flex_1().min_w_0().h_full().child(
                            Editor::new(&self.state)
                                .gutter_style(gutter)
                                .bordered(false)
                                .h_full()
                                .line_height(relative(line_height)),
                        ),
                    )
                    .children(self.side.iter().cloned()),
            )
            .children(self.find_panel.clone())
    }
}
