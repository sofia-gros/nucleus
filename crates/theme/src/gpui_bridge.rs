//! 解決済みのテーマを gpui / gpui-kit に渡す。色の直書きが許されるのはこのモジュールだけ。
#![allow(clippy::disallowed_types, clippy::disallowed_methods)]

use std::{rc::Rc, sync::Arc};

use gpui_kit::{App, Global, Hsla, Rgba, component::Theme as KitTheme};
use serde_json::{Map, Value, json};

use crate::{Color, ExtTokenId, Mode, ResolvedTheme, Token};

impl From<Color> for Hsla {
    fn from(c: Color) -> Hsla {
        Rgba {
            r: c.r as f32 / 255.0,
            g: c.g as f32 / 255.0,
            b: c.b as f32 / 255.0,
            a: c.a as f32 / 255.0,
        }
        .into()
    }
}

/// 有効なテーマ（gpui の Global）。
pub struct ActiveTheme {
    pub resolved: Arc<ResolvedTheme>,
    core: Vec<Hsla>,
    ext: Vec<Hsla>,
}

impl Global for ActiveTheme {}

impl ActiveTheme {
    fn new(resolved: Arc<ResolvedTheme>) -> Self {
        let core = Token::ALL.iter().map(|t| resolved.get(*t).into()).collect();
        let ext = resolved.ext_colors().iter().map(|c| (*c).into()).collect();
        ActiveTheme { resolved, core, ext }
    }
}

/// テーマの色を取り出す。Core も Extension も色はここからしか取らない。
pub trait TokenColors {
    fn token(&self, token: Token) -> Hsla;
    fn ext_token(&self, id: ExtTokenId) -> Hsla;
}

impl TokenColors for App {
    #[inline]
    fn token(&self, token: Token) -> Hsla {
        self.global::<ActiveTheme>().core[token.index()]
    }

    #[inline]
    fn ext_token(&self, id: ExtTokenId) -> Hsla {
        self.global::<ActiveTheme>()
            .ext
            .get(id.0 as usize)
            .copied()
            .unwrap_or_default()
    }
}

/// gpui-kit の ThemeConfig（JSON）を組み立てる。キーは Core Token と同じ名前。
pub fn kit_config_json(t: &ResolvedTheme) -> Value {
    let mut colors = Map::new();
    let mut highlight = Map::new();
    let mut syntax = Map::new();
    for token in Token::ALL {
        if !token.is_gpui() {
            continue;
        }
        let hex = Value::String(t.get(token).to_hex());
        match token.key().strip_prefix("highlight.") {
            Some(rest) => match rest.strip_prefix("syntax.") {
                Some(name) => {
                    syntax.insert(kit_syntax_key(name).to_string(), json!({ "color": hex }));
                }
                None => {
                    highlight.insert(rest.to_string(), hex);
                }
            },
            None => {
                colors.insert(token.key().to_string(), hex);
            }
        }
    }
    highlight.insert("syntax".into(), Value::Object(syntax));
    json!({
        "name": t.name,
        "mode": match t.mode { Mode::Dark => "dark", Mode::Light => "light" },
        "font.family": t.fonts.ui,
        "mono_font.family": t.fonts.mono,
        "mono_font.size": t.fonts.mono_size,
        "radius": 6,
        "radius.lg": 8,
        "colors": colors,
        "highlight": highlight,
    })
}

/// gpui-kit のシンタックス名（ほとんどは同じ名前。`comment.doc` だけ `comment_doc`）。
fn kit_syntax_key(name: &str) -> &str {
    match name {
        "comment.doc" => "comment_doc",
        other => other,
    }
}

/// テーマを有効にする。すべてのウィンドウが次のフレームで新しい色になる。
pub fn install(resolved: Arc<ResolvedTheme>, cx: &mut App) {
    let config: gpui_kit::component::ThemeConfig = match serde_json::from_value(kit_config_json(&resolved)) {
        Ok(c) => c,
        Err(e) => {
            log::error!("gpui-kit のテーマ設定を作れなかった: {e}");
            return;
        }
    };
    cx.set_global(ActiveTheme::new(resolved));
    let config = Rc::new(config);
    KitTheme::update(cx, |theme| theme.apply_config(&config));
}

/// ターミナルの 256 色。0〜15 はテーマの Token、16〜231 は 6×6×6 の色立方体、232〜255 は灰色の段階。
pub fn ansi_color(index: u8, cx: &App) -> Hsla {
    const ANSI: [Token; 16] = [
        Token::TerminalAnsiBlack,
        Token::TerminalAnsiRed,
        Token::TerminalAnsiGreen,
        Token::TerminalAnsiYellow,
        Token::TerminalAnsiBlue,
        Token::TerminalAnsiMagenta,
        Token::TerminalAnsiCyan,
        Token::TerminalAnsiWhite,
        Token::TerminalAnsiBrightBlack,
        Token::TerminalAnsiBrightRed,
        Token::TerminalAnsiBrightGreen,
        Token::TerminalAnsiBrightYellow,
        Token::TerminalAnsiBrightBlue,
        Token::TerminalAnsiBrightMagenta,
        Token::TerminalAnsiBrightCyan,
        Token::TerminalAnsiBrightWhite,
    ];
    match index {
        0..=15 => cx.token(ANSI[index as usize]),
        16..=231 => {
            let i = index - 16;
            let level = |v: u8| if v == 0 { 0 } else { 55 + v * 40 };
            program_color(level(i / 36), level((i / 6) % 6), level(i % 6))
        }
        _ => {
            let v = 8 + (index - 232) * 10;
            program_color(v, v, v)
        }
    }
}

/// 外部プログラム（ターミナルに出力されたエスケープシーケンスなど）が指定した色。
/// テーマで決められない唯一の例外なので、用途をここに限る。
pub fn program_color(r: u8, g: u8, b: u8) -> Hsla {
    Color { r, g, b, a: 255 }.into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ExtTokenSet, builtin, resolve};

    /// gpui-kit が読み捨てたキーがないこと（名前の打ち間違いを検出する）。
    #[test]
    fn every_gpui_token_reaches_gpui_kit() {
        for mode in [Mode::Dark, Mode::Light] {
            let (t, _) = resolve(&builtin(mode), &ExtTokenSet::default()).unwrap();
            let json = kit_config_json(&t);
            let config: gpui_kit::component::ThemeConfig = serde_json::from_value(json.clone()).unwrap();
            let back = serde_json::to_value(&config).unwrap();
            for token in Token::ALL.iter().filter(|t| t.is_gpui()) {
                let key = token.key();
                let found = match key.strip_prefix("highlight.") {
                    Some(rest) => match rest.strip_prefix("syntax.") {
                        Some(name) => back["highlight"]["syntax"][kit_syntax_key(name)]["color"].is_string(),
                        None => back["highlight"][rest].is_string(),
                    },
                    None => back["colors"][key].is_string(),
                };
                assert!(found, "{mode:?}: gpui-kit dropped `{key}`");
            }
        }
    }
}
