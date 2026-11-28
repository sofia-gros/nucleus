//! 公式の MiniMap Extension。エディタの右側にファイル全体の縮図を描く。
//! 見えている範囲を重ねて描き、クリック・ドラッグでその場所へ動かす。
//!
//! 表示は設定 `mode` で切り替える（拡張機能の画面の設定、1.1.0）:
//! - `blocks`: 1 行 2px、1 文字 1px の四角形（軽い）。
//! - `text`: 実際の文字を小さく描く（1 行 4px、canvas-text を使う）。

use std::{cell::RefCell, collections::HashMap};

use nucleus_extension_api::{
    Extension, export_extension, host, setting_str,
    ui::{DrawRect, El, PointerKind, TextLine, TextSpan, UiEvent, canvas, canvas_text, h_stack, rect},
};

mod lex;

/// 左の余白（px）。
const LEFT: f32 = 6.0;
/// 1 行に描く文字数の上限。
const MAX_COLUMNS: usize = 120;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    Blocks,
    Text,
}

impl Mode {
    fn current() -> Mode {
        match setting_str("mode").as_deref() {
            Some("text") => Mode::Text,
            _ => Mode::Blocks,
        }
    }

    /// 縮図の幅と 1 行の高さ（px）。
    fn metrics(self) -> (f32, f32) {
        match self {
            Mode::Blocks => (96.0, 2.0),
            Mode::Text => (120.0, 4.0),
        }
    }
}

/// 文字の大きさ（text のとき）。等幅で 1 文字はおよそ 0.6 倍の幅になる。
const TEXT_SIZE: f32 = 3.2;

struct Cache {
    version: u64,
    /// 行ごとの（開始列, 長さ, 色）。
    lines: Vec<Vec<(u16, u16, lex::Class)>>,
    /// 行の文字（タブは空白に広げ、長い行は切ったもの）。
    texts: Vec<Vec<char>>,
}

#[derive(Default)]
struct State {
    hidden: bool,
    cache: HashMap<u32, Cache>,
}

thread_local! {
    static STATE: RefCell<State> = RefCell::new(State::default());
}

fn editor_id(view: &str) -> Option<u32> {
    view.strip_prefix("minimap@")?.parse().ok()
}

/// MiniMap の上端に来るエディタの行（ファイルが長いとき、見えている範囲に合わせて縮図も動かす）。
fn offset_lines(v: &host::EditorState, rows: f32) -> f32 {
    let total = v.line_count as f32;
    if total <= rows {
        return 0.0;
    }
    let visible = (v.last_line.saturating_sub(v.first_line) + 1) as f32;
    let scrollable = (total - visible).max(1.0);
    let fraction = (v.first_line as f32 / scrollable).clamp(0.0, 1.0);
    fraction * (total - rows)
}

struct Minimap;

impl Extension for Minimap {
    fn run_command(id: String) -> Result<(), String> {
        match id.as_str() {
            "minimap.toggle" => {
                STATE.with(|s| {
                    let mut s = s.borrow_mut();
                    s.hidden = !s.hidden;
                });
                Ok(())
            }
            other => Err(format!("unknown command `{other}`")),
        }
    }

    fn render(view: String) -> El {
        let Some(id) = editor_id(&view) else { return h_stack() };
        if STATE.with(|s| s.borrow().hidden) {
            return h_stack();
        }
        let Some(v) = host::editor_view(id) else { return h_stack() };
        let mode = Mode::current();
        let (width, _) = mode.metrics();
        let rects = STATE.with(|s| {
            let mut s = s.borrow_mut();
            let stale = s.cache.get(&id).is_none_or(|c| c.version != v.version);
            if stale {
                let text = host::editor_text(id).unwrap_or_default();
                s.cache.insert(
                    id,
                    Cache {
                        version: v.version,
                        lines: lex::analyze(&text, MAX_COLUMNS),
                        texts: text
                            .lines()
                            .map(|l| expand_tabs(l).into_iter().take(MAX_COLUMNS).collect())
                            .collect(),
                    },
                );
            }
            draw(&s.cache[&id], &v, mode)
        });
        h_stack().child(canvas("map", width, rects))
    }

    fn handle_ui_event(view: String, _key: String, event: UiEvent) -> Result<(), String> {
        let (Some(id), UiEvent::Pointer(p)) = (editor_id(&view), event) else {
            return Ok(());
        };
        if p.kind == PointerKind::Up {
            return Ok(());
        }
        let Some(v) = host::editor_view(id) else { return Ok(()) };
        let (_, line_h) = Mode::current().metrics();
        let rows = p.height / line_h;
        let line = offset_lines(&v, rows) + p.y / line_h;
        // 押した場所が見えている範囲の真ん中に来るようにする。
        let half = (v.last_line.saturating_sub(v.first_line) + 1) / 2;
        let top = (line.max(0.0) as u32).saturating_sub(half);
        host::scroll_editor(id, top.min(v.line_count.saturating_sub(1)));
        Ok(())
    }
}

fn expand_tabs(line: &str) -> Vec<char> {
    let mut out = Vec::new();
    for c in line.chars() {
        if c == '\t' {
            out.extend([' '; 4]);
        } else {
            out.push(c);
        }
    }
    out
}

fn draw(cache: &Cache, v: &host::EditorState, mode: Mode) -> Vec<DrawRect> {
    let (width, line_h) = mode.metrics();
    let height = if v.height > 0.0 { v.height } else { 800.0 };
    let rows = height / line_h;
    let top = offset_lines(v, rows);
    let first = top.floor() as usize;
    let shift = (top - first as f32) * line_h;
    let mut out = Vec::new();
    // 見えている範囲（スクロールバーのつまみの色を薄く重ねる）。
    let y = (v.first_line as f32 - top) * line_h;
    let h = (v.last_line.saturating_sub(v.first_line) + 1) as f32 * line_h;
    out.push(rect(0.0, y, width, h.max(line_h), "scrollbar.thumb.background", 0.35));
    let visible = cache.lines.iter().enumerate().skip(first).take(rows as usize + 1);
    match mode {
        Mode::Blocks => {
            for (i, line) in visible {
                let y = (i - first) as f32 * line_h - shift;
                for &(col, len, class) in line {
                    out.push(rect(LEFT + col as f32, y, len as f32, line_h - 0.5, class.token(), 0.7));
                }
            }
        }
        Mode::Text => {
            let lines: Vec<TextLine> = visible
                .filter_map(|(i, tokens)| {
                    let chars = cache.texts.get(i)?;
                    let spans = spans(chars, tokens);
                    (!spans.is_empty()).then(|| TextLine {
                        x: LEFT,
                        y: (i - first) as f32 * line_h - shift,
                        size: TEXT_SIZE,
                        alpha: 0.9,
                        spans,
                    })
                })
                .collect();
            canvas_text("map", &lines);
        }
    }
    out
}

/// 字句の色分けを、文字の並び（隙間は空白）に直す。
fn spans(chars: &[char], tokens: &[(u16, u16, lex::Class)]) -> Vec<TextSpan> {
    let mut out = Vec::new();
    let mut col = 0usize;
    for &(start, len, class) in tokens {
        let (start, end) = (start as usize, (start + len) as usize);
        if start > chars.len() {
            break;
        }
        if start > col {
            out.push(TextSpan {
                text: " ".repeat(start - col),
                color: lex::Class::Text.token().into(),
            });
        }
        let end = end.min(chars.len());
        out.push(TextSpan {
            text: chars[start..end].iter().collect(),
            color: class.token().into(),
        });
        col = end;
    }
    out
}

export_extension!(Minimap);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spans_keep_columns() {
        let chars = expand_tabs("\tlet x = 1;");
        let tokens = lex::analyze("\tlet x = 1;", MAX_COLUMNS).remove(0);
        let text: String = spans(&chars, &tokens).iter().map(|s| s.text.as_str()).collect();
        assert_eq!(text, "    let x = 1;");
    }
}
