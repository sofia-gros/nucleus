//! 端末の画面を描く要素。大きさに合わせて行数・列数を決め、見えている行だけを描く。

use gpui_kit::{
    App, Bounds, Element, ElementId, Entity, FontStyle, FontWeight, GlobalElementId, Hsla, InspectorElementId,
    LayoutId, Pixels, ShapedLine, SharedString, Size, Style, TextAlign, TextRun, Window, fill, font, outline, point,
    px, relative, size,
};
use remote::{CursorShape, TermColor, cell_flags as F};
use theme::{Token, TokenColors as _};

use crate::model::{GridSize, Terminal};

const PADDING: f32 = 8.;

pub struct TerminalElement {
    terminal: Entity<Terminal>,
    focused: bool,
}

impl TerminalElement {
    pub fn new(terminal: Entity<Terminal>, focused: bool) -> Self {
        TerminalElement { terminal, focused }
    }
}

struct Segment {
    col: usize,
    line: ShapedLine,
}

struct Cursor {
    bounds: Bounds<Pixels>,
    shape: CursorShape,
    text: Option<ShapedLine>,
}

pub struct Prepaint {
    cell: Size<Pixels>,
    rows: Vec<Vec<Segment>>,
    backgrounds: Vec<(Bounds<Pixels>, Hsla)>,
    cursor: Option<Cursor>,
}

/// 端末の色をテーマの色に変える（0〜15 と既定色は Token、それ以外は計算）。
/// 名前の色の番号は alacritty と同じ（256 前景、257 背景、258 カーソル、259〜266 薄い 0〜7、267 明るい前景、268 薄い前景）。
fn color(c: TermColor, cx: &App) -> Hsla {
    let dim = |mut c: Hsla| {
        c.a *= 0.7;
        c
    };
    match c {
        TermColor::Named(n) if n < 16 => theme::ansi_color(n as u8, cx),
        TermColor::Named(256) | TermColor::Named(267) => cx.token(Token::TerminalForeground),
        TermColor::Named(257) => cx.token(Token::TerminalBackground),
        TermColor::Named(258) => cx.token(Token::TerminalCursor),
        TermColor::Named(268) => dim(cx.token(Token::TerminalForeground)),
        TermColor::Named(n @ 259..=266) => dim(theme::ansi_color((n - 259) as u8, cx)),
        TermColor::Named(_) => cx.token(Token::TerminalForeground),
        TermColor::Indexed(i) => theme::ansi_color(i, cx),
        TermColor::Rgb(r, g, b) => theme::program_color(r, g, b),
    }
}

impl Element for TerminalElement {
    type RequestLayoutState = ();
    type PrepaintState = Prepaint;

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, ()) {
        let mut style = Style::default();
        style.size.width = relative(1.).into();
        style.size.height = relative(1.).into();
        (window.request_layout(style, [], cx), ())
    }

    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _: &mut (),
        window: &mut Window,
        cx: &mut App,
    ) -> Prepaint {
        let fonts = cx.global::<theme::ActiveTheme>().resolved.fonts.clone();
        let s = settings::ActiveSettings::get(cx);
        // terminal.font_family / font_size が空なら、エディタと同じ。
        let mono_size = match s.terminal.font_size {
            Some(size) => size.clamp(6.0, 72.0) * s.window.zoom(),
            None => fonts.mono_size,
        };
        let family = s
            .terminal
            .font_family
            .clone()
            .filter(|f| !f.trim().is_empty())
            .unwrap_or_else(|| fonts.mono.clone());
        let font_size = px(mono_size);
        let base_font = font(SharedString::from(family));
        let font_id = window.text_system().resolve_font(&base_font);
        let cell_width = window
            .text_system()
            .advance(font_id, font_size, 'm')
            .map(|s| s.width)
            .unwrap_or(font_size * 0.6);
        let line_height = px((mono_size * s.terminal.line_height.clamp(1.0, 3.0)).round());
        let cell = size(cell_width, line_height);

        let cols = ((bounds.size.width - px(PADDING * 2.)) / cell_width).floor().max(2.) as u16;
        let lines = ((bounds.size.height - px(PADDING * 2.)) / line_height).floor().max(1.) as u16;
        self.terminal.update(cx, |t, _| {
            t.resize(GridSize {
                cols,
                lines,
                cell_width: f32::from(cell_width) as u16,
                cell_height: f32::from(line_height) as u16,
            })
        });

        let default_bg = cx.token(Token::TerminalBackground);
        let origin = bounds.origin + point(px(PADDING), px(PADDING));
        let at = |col: usize, row: usize| origin + point(cell_width * col as f32, line_height * row as f32);

        let terminal = self.terminal.read(cx);
        let mut rows: Vec<Vec<Segment>> = (0..lines).map(|_| Vec::new()).collect();
        let mut backgrounds = Vec::new();
        let mut cursor_char = None;
        let cursor_at = terminal.cursor.map(|c| (c.row as usize, c.col as usize));

        // 1行ずつ、同じ見た目の文字をまとめて1つの断片として組む。
        let mut current: Option<(usize, usize, String, Vec<TextRun>, usize)> = None; // (row, col, text, runs, next_col)
        let flush = |cur: &mut Option<(usize, usize, String, Vec<TextRun>, usize)>,
                     rows: &mut Vec<Vec<Segment>>,
                     window: &mut Window| {
            if let Some((row, col, text, runs, _)) = cur.take()
                && !text.trim_end().is_empty()
            {
                let line = window
                    .text_system()
                    .shape_line(text.into(), font_size, &runs, Some(cell_width));
                rows[row].push(Segment { col, line });
            }
        };

        for (row, cells) in terminal.rows.iter().enumerate().take(lines as usize) {
            for (col, cell) in cells.iter().enumerate() {
                // 全角文字の後ろの空きマス。
                if cell.text.is_empty() {
                    continue;
                }
                let has = |f: u16| cell.flags & f != 0;
                let mut fg = color(cell.fg, cx);
                let mut bg = color(cell.bg, cx);
                if has(F::INVERSE) {
                    std::mem::swap(&mut fg, &mut bg);
                }
                if has(F::DIM) {
                    fg.a *= 0.7;
                }
                let wide = has(F::WIDE);
                let width = if wide { 2 } else { 1 };
                if bg != default_bg {
                    backgrounds.push((
                        Bounds::new(at(col, row), size(cell_width * width as f32, line_height)),
                        bg,
                    ));
                }
                let text = if has(F::HIDDEN) {
                    " ".to_string()
                } else {
                    cell.text.clone()
                };
                if cursor_at == Some((row, col)) {
                    cursor_char = Some((text.clone(), bg, wide));
                }
                let mut f = base_font.clone();
                if has(F::BOLD) {
                    f.weight = FontWeight::BOLD;
                }
                if has(F::ITALIC) {
                    f.style = FontStyle::Italic;
                }
                let underline = (has(F::UNDERLINE) || has(F::UNDERCURL)).then(|| gpui_kit::UnderlineStyle {
                    thickness: px(1.),
                    color: Some(fg),
                    wavy: has(F::UNDERCURL),
                });
                let strike = has(F::STRIKEOUT).then(|| gpui_kit::StrikethroughStyle {
                    thickness: px(1.),
                    color: Some(fg),
                });
                let run = TextRun {
                    len: text.len(),
                    font: f,
                    color: fg,
                    background_color: None,
                    underline,
                    strikethrough: strike,
                };

                // 全角文字は 2 マス幅で別の断片にする（列がずれないように）。
                if wide {
                    flush(&mut current, &mut rows, window);
                    let line = window
                        .text_system()
                        .shape_line(text.into(), font_size, &[run], Some(cell_width * 2.));
                    rows[row].push(Segment { col, line });
                    continue;
                }
                match current.as_mut() {
                    Some((r, _, buf, runs, next)) if *r == row && *next == col => {
                        buf.push_str(&text);
                        let last = runs.last_mut().expect("non-empty");
                        if last.font == run.font
                            && last.color == run.color
                            && last.underline == run.underline
                            && last.strikethrough == run.strikethrough
                        {
                            last.len += run.len;
                        } else {
                            runs.push(run);
                        }
                        *next += 1;
                    }
                    _ => {
                        flush(&mut current, &mut rows, window);
                        current = Some((row, col, text, vec![run], col + 1));
                    }
                }
            }
        }
        flush(&mut current, &mut rows, window);

        let cursor = terminal.cursor.filter(|c| (c.row as usize) < lines as usize).map(|c| {
            let row = c.row as usize;
            let wide = cursor_char.as_ref().is_some_and(|c| c.2);
            let w = if wide { cell_width * 2. } else { cell_width };
            let text = cursor_char
                .as_ref()
                .filter(|c| !c.0.trim().is_empty())
                .map(|(t, bg, _)| {
                    let run = TextRun {
                        len: t.len(),
                        font: base_font.clone(),
                        color: *bg,
                        background_color: None,
                        underline: None,
                        strikethrough: None,
                    };
                    window
                        .text_system()
                        .shape_line(t.clone().into(), font_size, &[run], Some(w))
                });
            Cursor {
                bounds: Bounds::new(at(c.col as usize, row), size(w, line_height)),
                shape: c.shape,
                text,
            }
        });

        Prepaint {
            cell,
            rows,
            backgrounds,
            cursor,
        }
    }

    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _: &mut (),
        state: &mut Prepaint,
        window: &mut Window,
        cx: &mut App,
    ) {
        window.paint_quad(fill(bounds, cx.token(Token::TerminalBackground)));
        for (b, c) in &state.backgrounds {
            window.paint_quad(fill(*b, *c));
        }
        let origin = bounds.origin + point(px(PADDING), px(PADDING));
        let line_height = state.cell.height;
        for (row, segments) in state.rows.iter().enumerate() {
            for seg in segments {
                let p = origin + point(state.cell.width * seg.col as f32, line_height * row as f32);
                let _ = seg.line.paint(p, line_height, TextAlign::Left, None, window, cx);
            }
        }
        if let Some(c) = &state.cursor {
            let color = cx.token(Token::TerminalCursor);
            match (c.shape, self.focused) {
                (CursorShape::Block, true) => {
                    window.paint_quad(fill(c.bounds, color));
                    if let Some(t) = &c.text {
                        let _ = t.paint(c.bounds.origin, line_height, TextAlign::Left, None, window, cx);
                    }
                }
                (CursorShape::Underline, _) => {
                    let b = Bounds::new(
                        c.bounds.origin + point(px(0.), line_height - px(2.)),
                        size(c.bounds.size.width, px(2.)),
                    );
                    window.paint_quad(fill(b, color));
                }
                (CursorShape::Beam, _) => {
                    window.paint_quad(fill(Bounds::new(c.bounds.origin, size(px(2.), line_height)), color));
                }
                _ => window.paint_quad(outline(c.bounds, color, Default::default())),
            }
        }
    }
}
