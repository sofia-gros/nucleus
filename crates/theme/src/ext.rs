use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// 拡張 Token の ID（読み込み時に割り当てる番号）。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ExtTokenId(pub u32);

/// Token の分類（テーマ作者向けの表示用）。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TokenKind {
    #[default]
    Ui,
    Brand,
    Syntax,
    Icon,
}

/// Extension のマニフェストに書かれる拡張 Token の宣言。
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ExtTokenDecl {
    pub dark: String,
    pub light: String,
    #[serde(default)]
    pub kind: TokenKind,
    #[serde(default)]
    pub description: Option<String>,
}

/// 宣言をまとめたもの。キーは `<publisher>.<ext>.<name>`。
#[derive(Clone, Debug, Default)]
pub struct ExtTokenSet {
    decls: BTreeMap<String, (String, ExtTokenDecl)>,
}

impl ExtTokenSet {
    /// `owner` は宣言した Extension の ID。キーは必ず `owner.` で始まる。
    pub fn declare(&mut self, owner: &str, key: &str, decl: ExtTokenDecl) -> Result<(), String> {
        if !key.starts_with(&format!("{owner}.")) {
            return Err(format!("拡張 Token `{key}` は `{owner}.` で始める"));
        }
        if crate::Token::from_key(key).is_some() {
            return Err(format!("`{key}` は Core Token と同じ名前"));
        }
        if let Some((other, _)) = self.decls.get(key) {
            return Err(format!("拡張 Token `{key}` は `{other}` がすでに宣言している"));
        }
        self.decls.insert(key.to_string(), (owner.to_string(), decl));
        Ok(())
    }

    pub fn iter(&self) -> impl Iterator<Item = (&str, &str, &ExtTokenDecl)> {
        self.decls.iter().map(|(k, (o, d))| (k.as_str(), o.as_str(), d))
    }

    pub fn len(&self) -> usize {
        self.decls.len()
    }

    pub fn is_empty(&self) -> bool {
        self.decls.is_empty()
    }
}
