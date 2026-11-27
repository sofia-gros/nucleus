use std::collections::{BTreeMap, HashMap};

use serde::{Deserialize, Serialize};

use crate::{
    Token,
    color::Color,
    ext::{ExtTokenId, ExtTokenSet},
    file::{Fonts, Mode, ThemeFile, builtin},
    value::ValueExpr,
};

/// 参照をすべて解決した、描画にそのまま使える色の表。
#[derive(Clone, Debug, PartialEq)]
pub struct ResolvedTheme {
    pub name: String,
    pub mode: Mode,
    pub fonts: Fonts,
    core: Vec<Color>,
    ext: Vec<Color>,
    ext_ids: HashMap<String, ExtTokenId>,
}

impl ResolvedTheme {
    #[inline]
    pub fn get(&self, token: Token) -> Color {
        self.core[token.index()]
    }

    #[inline]
    pub fn get_ext(&self, id: ExtTokenId) -> Color {
        self.ext.get(id.0 as usize).copied().unwrap_or(Color::TRANSPARENT)
    }

    /// 拡張 Token の色（`ExtTokenId` の順）。
    pub fn ext_colors(&self) -> &[Color] {
        &self.ext
    }

    pub fn ext_id(&self, key: &str) -> Option<ExtTokenId> {
        self.ext_ids.get(key).copied()
    }

    /// キャッシュ用の表現。
    pub fn to_snapshot(&self) -> Snapshot {
        let mut ext: BTreeMap<String, String> = BTreeMap::new();
        for (k, id) in &self.ext_ids {
            ext.insert(k.clone(), self.ext[id.0 as usize].to_hex());
        }
        Snapshot {
            version: Snapshot::VERSION,
            name: self.name.clone(),
            mode: self.mode,
            fonts: self.fonts.clone(),
            core: Token::ALL
                .iter()
                .map(|t| (t.key().to_string(), self.get(*t).to_hex()))
                .collect(),
            ext,
        }
    }

    /// キャッシュから復元する。Core Token が欠けていたり形式が違えば `None`。
    pub fn from_snapshot(s: &Snapshot) -> Option<ResolvedTheme> {
        if s.version != Snapshot::VERSION {
            return None;
        }
        let mut core = Vec::with_capacity(Token::COUNT);
        for t in Token::ALL {
            core.push(Color::parse_hex(s.core.get(t.key())?)?);
        }
        let mut ext = Vec::new();
        let mut ext_ids = HashMap::new();
        for (k, v) in &s.ext {
            ext_ids.insert(k.clone(), ExtTokenId(ext.len() as u32));
            ext.push(Color::parse_hex(v)?);
        }
        Some(ResolvedTheme {
            name: s.name.clone(),
            mode: s.mode,
            fonts: s.fonts.clone(),
            core,
            ext,
            ext_ids,
        })
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Snapshot {
    pub version: u32,
    pub name: String,
    pub mode: Mode,
    pub fonts: Fonts,
    pub core: BTreeMap<String, String>,
    pub ext: BTreeMap<String, String>,
}

impl Snapshot {
    /// Token の構成が変わったら上げる（古いキャッシュを捨てるため）。
    pub const VERSION: u32 = 1;
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct Diagnostics {
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

/// テーマを解決する。
///
/// 色の決まり方（優先度の高い順）:
/// 1. `theme` が上書きした値
/// 2. Core Token は同じモードの同梱テーマの値、拡張 Token は宣言の既定値（dark / light）
/// 3. 参照（`$key`、`$key/NN%`）を最後にまとめて解決
pub fn resolve(theme: &ThemeFile, ext: &ExtTokenSet) -> Result<(ResolvedTheme, Diagnostics), Diagnostics> {
    let mut diag = Diagnostics::default();
    let base = builtin(theme.mode);

    // key -> 式
    let mut exprs: HashMap<String, ValueExpr> = HashMap::new();
    let mut put = |key: &str, raw: &str, origin: &str, diag: &mut Diagnostics| match ValueExpr::parse(raw) {
        Ok(e) => {
            exprs.insert(key.to_string(), e);
        }
        Err(e) => diag.errors.push(format!("{origin}: `{key}`: {e}")),
    };

    for (k, v) in &base.colors {
        put(k, v, "同梱テーマ", &mut diag);
    }
    for (k, v) in &theme.colors {
        if Token::from_key(k).is_none() {
            diag.warnings.push(format!(
                "テーマ `{}`: `{k}` は Core Token ではないので無視する",
                theme.name
            ));
            continue;
        }
        put(k, v, &format!("テーマ `{}`", theme.name), &mut diag);
    }
    for (k, owner, d) in ext.iter() {
        let raw = match theme.mode {
            Mode::Dark => &d.dark,
            Mode::Light => &d.light,
        };
        put(k, raw, &format!("Extension `{owner}`"), &mut diag);
    }
    for (k, v) in &theme.tokens {
        if ext.iter().all(|(key, _, _)| key != k) {
            diag.warnings.push(format!(
                "テーマ `{}`: 拡張 Token `{k}` を宣言した Extension がないので無視する",
                theme.name
            ));
            continue;
        }
        put(k, v, &format!("テーマ `{}`", theme.name), &mut diag);
    }

    for t in Token::ALL {
        if !exprs.contains_key(t.key()) {
            diag.errors.push(format!("Core Token `{}` の値がない", t.key()));
        }
    }
    if !diag.errors.is_empty() {
        return Err(diag);
    }

    // 参照の解決（循環検出つき）
    let mut done: HashMap<String, Color> = HashMap::new();
    fn eval(
        key: &str,
        exprs: &HashMap<String, ValueExpr>,
        done: &mut HashMap<String, Color>,
        stack: &mut Vec<String>,
    ) -> Result<Color, String> {
        if let Some(c) = done.get(key) {
            return Ok(*c);
        }
        if stack.iter().any(|k| k == key) {
            stack.push(key.to_string());
            return Err(format!("参照が循環している: {}", stack.join(" → ")));
        }
        let expr = exprs.get(key).ok_or_else(|| match stack.last() {
            Some(from) => format!("`{from}` が参照する `{key}` は存在しない"),
            None => format!("`{key}` は存在しない"),
        })?;
        stack.push(key.to_string());
        let c = match expr {
            ValueExpr::Hex(c) => *c,
            ValueExpr::Ref {
                key: target,
                alpha_percent,
            } => {
                let c = eval(target, exprs, done, stack)?;
                alpha_percent.map_or(c, |p| c.scale_alpha(p))
            }
        };
        stack.pop();
        done.insert(key.to_string(), c);
        Ok(c)
    }

    let mut keys: Vec<&String> = exprs.keys().collect();
    keys.sort();
    for k in keys {
        if let Err(e) = eval(k, &exprs, &mut done, &mut Vec::new())
            && !diag.errors.contains(&e)
        {
            diag.errors.push(e);
        }
    }
    if !diag.errors.is_empty() {
        return Err(diag);
    }

    let core: Vec<Color> = Token::ALL.iter().map(|t| done[t.key()]).collect();
    let mut ext_colors = Vec::new();
    let mut ext_ids = HashMap::new();
    for (k, _, _) in ext.iter() {
        ext_ids.insert(k.to_string(), ExtTokenId(ext_colors.len() as u32));
        ext_colors.push(done[k]);
    }

    let resolved = ResolvedTheme {
        name: theme.name.clone(),
        mode: theme.mode,
        fonts: theme.fonts.clone().or(base.fonts).unwrap_or_default(),
        core,
        ext: ext_colors,
        ext_ids,
    };
    check_contrast(&resolved, &mut diag);
    Ok((resolved, diag))
}

/// 読みにくい文字色と背景の組み合わせを警告する（WCAG AA の大きい文字の基準 3.0）。
fn check_contrast(t: &ResolvedTheme, diag: &mut Diagnostics) {
    const PAIRS: &[(Token, Token)] = &[
        (Token::Foreground, Token::Background),
        (Token::MutedForeground, Token::Background),
        (Token::PrimaryForeground, Token::PrimaryBackground),
        (Token::SecondaryForeground, Token::SecondaryBackground),
        (Token::PopoverForeground, Token::PopoverBackground),
        (Token::SidebarForeground, Token::SidebarBackground),
        (Token::TabActiveForeground, Token::TabActiveBackground),
        (Token::HighlightEditorForeground, Token::HighlightEditorBackground),
    ];
    for (fg, bg) in PAIRS {
        let b = t.get(*bg).over(t.get(Token::Background));
        let ratio = t.get(*fg).over(b).contrast(b);
        if ratio < 3.0 {
            diag.warnings.push(format!(
                "テーマ `{}`: `{}` と `{}` のコントラスト比が {ratio:.2}（3.0 未満）",
                t.name,
                fg.key(),
                bg.key()
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ext::{ExtTokenDecl, TokenKind};

    fn theme(mode: Mode, colors: &[(&str, &str)], tokens: &[(&str, &str)]) -> ThemeFile {
        ThemeFile {
            name: "Test".into(),
            mode,
            fonts: None,
            colors: colors.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect(),
            tokens: tokens.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect(),
        }
    }

    #[test]
    fn builtin_themes_define_every_core_token_and_resolve_cleanly() {
        for mode in [Mode::Dark, Mode::Light] {
            let file = builtin(mode);
            for t in Token::ALL {
                assert!(file.colors.contains_key(t.key()), "{mode:?} lacks {}", t.key());
            }
            for k in file.colors.keys() {
                assert!(Token::from_key(k).is_some(), "{mode:?} has unknown key {k}");
            }
            let (_, diag) = resolve(&file, &ExtTokenSet::default()).expect("resolves");
            assert!(diag.warnings.is_empty(), "{mode:?}: {:?}", diag.warnings);
        }
    }

    #[test]
    fn missing_core_tokens_fall_back_to_the_builtin_of_the_same_mode() {
        let (t, _) = resolve(
            &theme(Mode::Light, &[("background", "#ffffff")], &[]),
            &ExtTokenSet::default(),
        )
        .unwrap();
        assert_eq!(t.get(Token::Background).to_hex(), "#ffffff");
        assert_eq!(t.get(Token::Foreground).to_hex(), "#383a42");
        // 同梱テーマの参照は、上書きした値を見る。
        assert_eq!(t.get(Token::HighlightEditorBackground).to_hex(), "#ffffff");
    }

    #[test]
    fn references_and_alpha_resolve() {
        let (t, _) = resolve(&builtin(Mode::Dark), &ExtTokenSet::default()).unwrap();
        assert_eq!(t.get(Token::HighlightCreatedBackground).to_hex(), "#98c37924");
        assert_eq!(t.get(Token::DragBorder), t.get(Token::PrimaryBackground));
    }

    #[test]
    fn cycles_and_dangling_references_are_errors() {
        let cyc = theme(
            Mode::Dark,
            &[("background", "$foreground"), ("foreground", "$background")],
            &[],
        );
        let err = resolve(&cyc, &ExtTokenSet::default()).unwrap_err();
        assert!(err.errors.iter().any(|e| e.contains("循環")), "{err:?}");

        let dangling = theme(Mode::Dark, &[("background", "$nope")], &[]);
        let err = resolve(&dangling, &ExtTokenSet::default()).unwrap_err();
        assert!(err.errors.iter().any(|e| e.contains("`nope` は存在しない")), "{err:?}");
    }

    #[test]
    fn extension_tokens_use_mode_defaults_and_theme_overrides() {
        let mut ext = ExtTokenSet::default();
        let decl = |d: &str, l: &str| ExtTokenDecl {
            dark: d.into(),
            light: l.into(),
            kind: TokenKind::Brand,
            description: None,
        };
        ext.declare("anthropic.claude", "anthropic.claude.brand", decl("#d97757", "#c96442"))
            .unwrap();
        ext.declare(
            "anthropic.claude",
            "anthropic.claude.subtle",
            decl("$anthropic.claude.brand/15%", "$anthropic.claude.brand/12%"),
        )
        .unwrap();
        ext.declare(
            "anthropic.claude",
            "anthropic.claude.bubble",
            decl("$muted.background", "$muted.background"),
        )
        .unwrap();

        let (dark, _) = resolve(&builtin(Mode::Dark), &ext).unwrap();
        let brand = dark.ext_id("anthropic.claude.brand").unwrap();
        assert_eq!(dark.get_ext(brand).to_hex(), "#d97757");
        assert_eq!(
            dark.get_ext(dark.ext_id("anthropic.claude.subtle").unwrap()).to_hex(),
            "#d9775726"
        );
        assert_eq!(
            dark.get_ext(dark.ext_id("anthropic.claude.bubble").unwrap()),
            dark.get(Token::MutedBackground)
        );

        let (light, _) = resolve(&builtin(Mode::Light), &ext).unwrap();
        assert_eq!(
            light.get_ext(light.ext_id("anthropic.claude.brand").unwrap()).to_hex(),
            "#c96442"
        );

        let custom = theme(
            Mode::Dark,
            &[],
            &[("anthropic.claude.brand", "#e08a6a"), ("someone.else.x", "#000000")],
        );
        let (t, diag) = resolve(&custom, &ext).unwrap();
        assert_eq!(
            t.get_ext(t.ext_id("anthropic.claude.brand").unwrap()).to_hex(),
            "#e08a6a"
        );
        assert_eq!(
            t.get_ext(t.ext_id("anthropic.claude.subtle").unwrap()).to_hex(),
            "#e08a6a26"
        );
        assert!(diag.warnings.iter().any(|w| w.contains("someone.else.x")));
    }

    #[test]
    fn extension_tokens_must_be_namespaced_and_unique() {
        let mut ext = ExtTokenSet::default();
        let d = ExtTokenDecl {
            dark: "#000000".into(),
            light: "#ffffff".into(),
            kind: TokenKind::Ui,
            description: None,
        };
        assert!(ext.declare("a.b", "c.d.x", d.clone()).is_err());
        assert!(ext.declare("a.b", "a.b.x", d.clone()).is_ok());
        assert!(ext.declare("a.b", "a.b.x", d).is_err());
    }

    #[test]
    fn snapshot_round_trips() {
        let mut ext = ExtTokenSet::default();
        ext.declare(
            "x.y",
            "x.y.z",
            ExtTokenDecl {
                dark: "#123456".into(),
                light: "#654321".into(),
                kind: TokenKind::Ui,
                description: None,
            },
        )
        .unwrap();
        let (t, _) = resolve(&builtin(Mode::Dark), &ext).unwrap();
        let snap = t.to_snapshot();
        let json = serde_json::to_string(&snap).unwrap();
        let back = ResolvedTheme::from_snapshot(&serde_json::from_str(&json).unwrap()).unwrap();
        assert_eq!(back.get(Token::TabActiveBackground), t.get(Token::TabActiveBackground));
        assert_eq!(back.get_ext(back.ext_id("x.y.z").unwrap()).to_hex(), "#123456");
    }
}
