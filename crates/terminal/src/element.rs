//! 端末の画面を描く要素。大きさに合わせて行数・列数を決め、見えている行だけを描く。

use alacritty_terminal::{
    term::{TermMode, cell::Flags},
    vte::ansi::{Color as AnsiColor, CursorShape, NamedColor},
};
use gpui_kit::{
    App, Bounds, Element, ElementId, Entity, FontStyle, FontWeight, GlobalElementId, Hsla, InspectorElementId,
    LayoutId, Pixels, ShapedLine, SharedString, Size, Style, TextAlign, TextRun, Window, fill, font, outline, point,
    px, relative, size,
};
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
fn color(c: AnsiColor, overrides: &alacritty_terminal::term::color::Colors, cx: &App) -> Hsla {
    let named = |n: NamedColor| -> Hsla {
        match n {
            NamedColor::Foreground | NamedColor::BrightForeground => cx.token(Token::TerminalForeground),
            NamedColor::Background => cx.token(Token::TerminalBackground),
            NamedColor::Cursor => cx.token(Token::TerminalCursor),
            NamedColor::DimForeground => {
                let mut c = cx.token(Token::TerminalForeground);
                c.a *= 0.7;
                c
            }
            n if (n as usize) < 16 => theme::ansi_color(n as u8, cx),
            dim => {
                // Dim* は 0〜7 の色を薄くしたもの。
                let base = (dim as usize).saturating_sub(NamedColor::DimBlack as usize) as u8;
                let mut c = theme::ansi_color(base.min(7), cx);
                c.a *= 0.7;
                c
            }
        }
    };
    match c {
        AnsiColor::Named(n) => match overrides[n] {
            // OSC でプログラムが色を変えた場合はそれに従う。
            Some(rgb) if (n as usize) < 16 => theme::program_color(rgb.r, rgb.g, rgb.b),
            _ => named(n),
        },
        AnsiColor::Indexed(i) => match overrides[i as usize] {
            Some(rgb) => theme::program_color(rgb.r, rgb.g, rgb.b),
            None => theme::ansi_color(i, cx),
        },
        AnsiColor::Spec(rgb) => theme::program_color(rgb.r, rgb.g, rgb.b),
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

        let term_handle = self.terminal.read(cx).term.clone();
        let term = term_handle.lock();
        let content = term.renderable_content();
        let display_offset = content.display_offset as i32;
        let colors = content.colors;
        let mut rows: Vec<Vec<Segment>> = (0..lines).map(|_| Vec::new()).collect();
        let mut backgrounds = Vec::new();
        let mut cursor_char = None;
        let cursor_point = content.cursor.point;
        let cursor_shape = content.cursor.shape;

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

        for indexed in content.display_iter {
            let row = indexed.point.line.0 + display_offset;
            if row < 0 || row as usize >= rows.len() {
                continue;
            }
            let (row, col) = (row as usize, indexed.point.column.0);
            let cell = &indexed.cell;
            if cell
                .flags
                .intersects(Flags::WIDE_CHAR_SPACER | Flags::LEADING_WIDE_CHAR_SPACER)
            {
                continue;
            }
            let mut fg = color(cell.fg, colors, cx);
            let mut bg = color(cell.bg, colors, cx);
            if cell.flags.contains(Flags::INVERSE) {
                std::mem::swap(&mut fg, &mut bg);
            }
            if cell.flags.contains(Flags::DIM) {
                fg.a *= 0.7;
            }
            let wide = cell.flags.contains(Flags::WIDE_CHAR);
            let width = if wide { 2 } else { 1 };
            if bg != default_bg {
                backgrounds.push((
                    Bounds::new(at(col, row), size(cell_width * width as f32, line_height)),
                    bg,
                ));
            }
            let mut text = if cell.flags.contains(Flags::HIDDEN) {
                ' '.to_string()
            } else {
                cell.c.to_string()
            };
            if let Some(zw) = cell.zerowidth() {
                text.extend(zw.iter());
            }
            if indexed.point == cursor_point {
                cursor_char = Some((text.clone(), bg, wide));
            }
            let mut f = base_font.clone();
            if cell.flags.contains(Flags::BOLD) {
                f.weight = FontWeight::BOLD;
            }
            if cell.flags.contains(Flags::ITALIC) {
                f.style = FontStyle::Italic;
            }
            let underline = cell
                .flags
                .intersects(Flags::ALL_UNDERLINES)
                .then(|| gpui_kit::UnderlineStyle {
                    thickness: px(1.),
                    color: Some(fg),
                    wavy: cell.flags.contains(Flags::UNDERCURL),
                });
            let strike = cell
                .flags
                .contains(Flags::STRIKEOUT)
                .then(|| gpui_kit::StrikethroughStyle {
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
        flush(&mut current, &mut rows, window);

        let cursor_row = cursor_point.line.0 + display_offset;
        let cursor = (cursor_shape != CursorShape::Hidden
            && content.mode.contains(TermMode::SHOW_CURSOR)
            && cursor_row >= 0
            && (cursor_row as usize) < lines as usize)
            .then(|| {
                let row = cursor_row as usize;
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
                    bounds: Bounds::new(at(cursor_point.column.0, row), size(w, line_height)),
                    shape: cursor_shape,
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
