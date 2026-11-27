use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    Dark,
    Light,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Fonts {
    pub ui: String,
    pub mono: String,
    pub mono_size: f32,
    /// 言語ごとの代替フォント（例: `ja` → `Noto Sans JP`）。
    #[serde(default)]
    pub fallbacks: BTreeMap<String, Vec<String>>,
}

impl Default for Fonts {
    fn default() -> Self {
        Fonts {
            ui: "Inter".into(),
            mono: "JetBrains Mono".into(),
            mono_size: 13.0,
            fallbacks: BTreeMap::new(),
        }
    }
}

/// テーマファイル（JSON）。
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ThemeFile {
    pub name: String,
    pub mode: Mode,
    #[serde(default)]
    pub fonts: Option<Fonts>,
    /// Core Token の値（キーは tokens.yaml の名前）。
    #[serde(default)]
    pub colors: BTreeMap<String, String>,
    /// 拡張 Token の上書き。
    #[serde(default)]
    pub tokens: BTreeMap<String, String>,
}

impl ThemeFile {
    pub fn from_json(s: &str) -> anyhow::Result<ThemeFile> {
        Ok(serde_json::from_str(s)?)
    }
}

pub const NUCLEUS_ONE_DARK: &str = include_str!("../themes/nucleus-one-dark.json");
pub const NUCLEUS_ONE_LIGHT: &str = include_str!("../themes/nucleus-one-light.json");

/// 同梱テーマ。Core Token をすべて定義している（テストで確認）。
pub fn builtin(mode: Mode) -> ThemeFile {
    let src = match mode {
        Mode::Dark => NUCLEUS_ONE_DARK,
        Mode::Light => NUCLEUS_ONE_LIGHT,
    };
    ThemeFile::from_json(src).expect("bundled theme is valid JSON")
}
