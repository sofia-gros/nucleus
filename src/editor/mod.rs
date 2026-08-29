//! gpui-component の Editor をUIコンポーネントとして採用し、
//! ropey ベースの TextBuffer と連携するエディタモジュール

pub mod buffer;
pub mod display_map;
pub mod selection;
pub mod cursor;
pub mod history;
pub mod actions;
pub mod completion;
pub mod hover;
pub mod find_replace;
pub mod bracket_match;

use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use gpui::*;
use gpui_component::input::{Editor as GpuiEditor, EditorState};
use crate::editor::buffer::TextBuffer;
use crate::workspace::editor_area::highlighter::{ShowcaseHighlightStyles, SyntectHighlighter};

/// gpui-component の EditorState と ropey の TextBuffer を統合したエディタエンティティ
pub struct Editor {
    /// gpui-component のエディタ状態
    pub editor_state: Entity<EditorState>,
    /// バックエンドの Rope ベーステキストバッファ
    pub buffer: Arc<RwLock<TextBuffer>>,
    /// 言語種別（例: "rust", "javascript" 等）
    pub language: String,
    /// 関連付けられたファイルパス
    pub file_path: Option<PathBuf>,
}

impl Editor {
    /// 新しい Editor エンティティを作成
    pub fn new(
        window: &mut Window,
        content: &str,
        file_path: Option<PathBuf>,
        language: &str,
        cx: &mut Context<Self>,
    ) -> Self {
        let mut text_buffer = TextBuffer::new(content);
        text_buffer.file_path = file_path.clone();
        let arc_buffer = Arc::new(RwLock::new(text_buffer));

        let lang_str = language.to_string();
        let lang_clone = lang_str.clone();

        let editor_state = cx.new(|cx| {
            let mut state = EditorState::new(window, cx)
                .language(&lang_clone)
                .folding(true)
                .default_value(content);

            // 構文ハイライターの設定
            state.set_highlighter_factory(
                std::rc::Rc::new(|lang: &str| {
                    if let Some(h) = SyntectHighlighter::new(lang) {
                        Some(Box::new(h) as Box<dyn gpui_component::input::InputHighlighter>)
                    } else {
                        None
                    }
                }),
                cx,
            );

            // ハイライトスタイルの設定
            let mut editor_style = gpui_base::input::InputEditorStyle::default();
            editor_style.highlight_styles = Arc::new(ShowcaseHighlightStyles::default());
            state.set_editor_style(editor_style);

            state
        });

        Self {
            editor_state,
            buffer: arc_buffer,
            language: lang_str,
            file_path,
        }
    }

    /// 現在のエディタのテキストを取得し、TextBuffer と同期する
    pub fn sync_to_buffer(&self, cx: &App) {
        let current_text = self.editor_state.read(cx).value();
        let mut buf = self.buffer.write().unwrap();
        if buf.to_string() != current_text {
            *buf = TextBuffer::new(&current_text);
            buf.file_path = self.file_path.clone();
            buf.is_dirty = true;
        }
    }

    /// バッファの内容をファイルに保存する
    pub fn save(&mut self, cx: &mut App) -> std::io::Result<()> {
        self.sync_to_buffer(cx);
        let mut buf = self.buffer.write().unwrap();
        if let Some(path) = &self.file_path {
            buf.save_to_file(path)?;
        }
        Ok(())
    }

    /// エディタのテキストを外部更新して同期
    pub fn set_text(&mut self, text: &str, window: &mut Window, cx: &mut Context<Self>) {
        self.editor_state.update(cx, |state, cx| {
            state.set_value(text.to_string(), window, cx);
        });
        let mut buf = self.buffer.write().unwrap();
        *buf = TextBuffer::new(text);
        buf.file_path = self.file_path.clone();
        buf.is_dirty = false;
        cx.notify();
    }

    /// バッファが変更されているか確認
    pub fn is_dirty(&self, cx: &App) -> bool {
        let current_text = self.editor_state.read(cx).value();
        let buf = self.buffer.read().unwrap();
        buf.is_dirty || buf.to_string() != current_text
    }
}

impl Render for Editor {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        GpuiEditor::new(&self.editor_state)
            .size_full()
    }
}
