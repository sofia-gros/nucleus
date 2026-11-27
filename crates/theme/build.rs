//! tokens.yaml から `enum Token` を生成する。

use std::{env, fmt::Write as _, fs, path::PathBuf};

#[derive(serde::Deserialize)]
struct Def {
    key: String,
    doc: String,
    #[serde(default = "yes")]
    gpui: bool,
}

fn yes() -> bool {
    true
}

fn variant(key: &str) -> String {
    key.split(['.', '_'])
        .filter(|s| !s.is_empty())
        .map(|s| {
            let mut c = s.chars();
            c.next()
                .map(|f| f.to_ascii_uppercase().to_string() + c.as_str())
                .unwrap_or_default()
        })
        .collect()
}

fn main() {
    println!("cargo:rerun-if-changed=tokens.yaml");
    let src = fs::read_to_string("tokens.yaml").expect("read tokens.yaml");
    let defs: Vec<Def> = serde_yaml_ng::from_str(&src).expect("parse tokens.yaml");

    let mut seen = std::collections::HashSet::new();
    for d in &defs {
        assert!(seen.insert(&d.key), "duplicate token key: {}", d.key);
        assert!(
            d.key
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '.' || c == '_'),
            "token keys are lowercase dotted names: {}",
            d.key
        );
    }

    let mut out = String::new();
    out.push_str("/// Core Token。`crates/theme/tokens.yaml` から生成される。\n");
    out.push_str(
        "#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]\n#[repr(u16)]\npub enum Token {\n",
    );
    for d in &defs {
        writeln!(out, "    /// `{}` — {}", d.key, d.doc).unwrap();
        writeln!(out, "    {},", variant(&d.key)).unwrap();
    }
    out.push_str("}\n\nimpl Token {\n");
    writeln!(out, "    pub const COUNT: usize = {};", defs.len()).unwrap();
    out.push_str("    pub const ALL: [Token; Self::COUNT] = [\n");
    for d in &defs {
        writeln!(out, "        Token::{},", variant(&d.key)).unwrap();
    }
    out.push_str("    ];\n\n    /// テーマファイルでのキー名。\n    pub const fn key(self) -> &'static str {\n        match self {\n");
    for d in &defs {
        writeln!(out, "            Token::{} => {:?},", variant(&d.key), d.key).unwrap();
    }
    out.push_str(
        "        }\n    }\n\n    /// 説明。\n    pub const fn doc(self) -> &'static str {\n        match self {\n",
    );
    for d in &defs {
        writeln!(out, "            Token::{} => {:?},", variant(&d.key), d.doc).unwrap();
    }
    out.push_str("        }\n    }\n\n    /// gpui-kit のテーマにも渡すかどうか。\n    pub const fn is_gpui(self) -> bool {\n        match self {\n");
    for d in &defs {
        writeln!(out, "            Token::{} => {},", variant(&d.key), d.gpui).unwrap();
    }
    out.push_str("        }\n    }\n\n    pub fn from_key(key: &str) -> Option<Token> {\n        match key {\n");
    for d in &defs {
        writeln!(out, "            {:?} => Some(Token::{}),", d.key, variant(&d.key)).unwrap();
    }
    out.push_str("            _ => None,\n        }\n    }\n\n    #[inline]\n    pub const fn index(self) -> usize {\n        self as usize\n    }\n}\n");

    let path = PathBuf::from(env::var("OUT_DIR").unwrap()).join("token.rs");
    fs::write(path, out).expect("write token.rs");
}
