//! 公式の MiniMap Extension。エディタの右側に、ファイル全体を 1 行 2px の縮図で描く。
//! 見えている範囲を重ねて描き、クリック・ドラッグでその場所へ動かす。

use std::{cell::RefCell, collections::HashMap};

use nucleus_extension_api::{
    Extension, export_extension, host,
    ui::{DrawRect, El, PointerKind, UiEvent, canvas, h_stack, rect},
};

mod lex;

/// MiniMap の幅（px）と 1 行の高さ（px）。1 文字は 1px。
const WIDTH: f32 = 96.0;
const LINE: f32 = 2.0;
const LEFT: f32 = 6.0;

struct Cache {
    version: u64,
    /// 行ごとの（開始列, 長さ, 色）。
    lines: Vec<Vec<(u16, u16, lex::Class)>>,
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
        let Some(v) = host::editor_view(id) else {
            return h_stack();
        };
        let rects = STATE.with(|s| {
            let mut s = s.borrow_mut();
            let stale = s.cache.get(&id).is_none_or(|c| c.version != v.version);
            if stale {
                let text = host::editor_text(id).unwrap_or_default();
                s.cache.insert(
                    id,
                    Cache {
                        version: v.version,
                        lines: lex::analyze(&text, WIDTH as usize),
                    },
                );
            }
            draw(&s.cache[&id], &v)
        });
        h_stack().child(canvas("map", WIDTH, rects))
    }

    fn handle_ui_event(view: String, _key: String, event: UiEvent) -> Result<(), String> {
        let (Some(id), UiEvent::Pointer(p)) = (editor_id(&view), event) else {
            return Ok(());
        };
        if p.kind == PointerKind::Up {
            return Ok(());
        }
        let Some(v) = host::editor_view(id) else { return Ok(()) };
        let rows = p.height / LINE;
        let line = offset_lines(&v, rows) + p.y / LINE;
        // 押した場所が見えている範囲の真ん中に来るようにする。
        let half = (v.last_line.saturating_sub(v.first_line) + 1) / 2;
        let top = (line.max(0.0) as u32).saturating_sub(half);
        host::scroll_editor(id, top.min(v.line_count.saturating_sub(1)));
        Ok(())
    }
}

fn draw(cache: &Cache, v: &host::EditorState) -> Vec<DrawRect> {
    let height = if v.height > 0.0 { v.height } else { 800.0 };
    let rows = height / LINE;
    let top = offset_lines(v, rows);
    let first = top.floor() as usize;
    let shift = (top - first as f32) * LINE;
    let mut out = Vec::new();
    // 見えている範囲（スクロールバーのつまみの色を薄く重ねる）。
    let y = (v.first_line as f32 - top) * LINE;
    let h = (v.last_line.saturating_sub(v.first_line) + 1) as f32 * LINE;
    out.push(rect(0.0, y, WIDTH, h.max(LINE), "scrollbar.thumb.background", 0.35));
    for (i, line) in cache.lines.iter().enumerate().skip(first).take(rows as usize + 1) {
        let y = (i - first) as f32 * LINE - shift;
        for &(col, len, class) in line {
            out.push(rect(LEFT + col as f32, y, len as f32, LINE - 0.5, class.token(), 0.7));
        }
    }
    out
}

export_extension!(Minimap);
