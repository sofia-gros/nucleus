//! `extension.yaml`（docs/ARCHITECTURE.md §4）。Extension は1種類だけで、できることは権限で宣言する。

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use theme::ExtTokenDecl;

/// Host が今サポートしている API の版。
pub const CORE_API: u32 = 1;
pub const UI_API: u32 = 1;

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    /// `<publisher>.<name>`（小文字・数字・`-`）。
    pub id: String,
    /// 表示名。`%key%` と書くと i18n キー。
    pub name: String,
    pub version: String,
    pub publisher: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub api: Api,
    #[serde(default)]
    pub depends: Vec<String>,
    #[serde(default)]
    pub permissions: BTreeMap<String, Vec<String>>,
    /// 独自の Token（`theme.tokens` 権限）。
    #[serde(default)]
    pub tokens: BTreeMap<String, ExtTokenDecl>,
    /// テーマファイル（`theme.colors` 権限）。マニフェストからの相対パス。
    #[serde(default)]
    pub themes: Vec<String>,
    /// 文言（`<lang>.ftl` を置くディレクトリ。`locale.messages` / `locale.pack` 権限）。
    #[serde(default)]
    pub l10n: Option<String>,
    /// 言語パック（`locale.pack` 権限）。
    #[serde(default)]
    pub locale: Option<LocalePack>,
    #[serde(default)]
    pub contributes: Contributes,
    /// WASM（P5）。データだけの Extension では省略する。
    #[serde(default)]
    pub main: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Api {
    #[serde(default)]
    pub core: Option<u32>,
    #[serde(default)]
    pub ui: Option<u32>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LocalePack {
    /// BCP 47 の言語タグ（例: `ja`, `es`）。
    pub language: String,
    /// 言語の表示名（その言語で書く。例: `日本語`）。
    pub label: String,
    /// 翻訳対象（`core` や Extension の ID）。
    #[serde(default)]
    pub translates: Vec<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Contributes {
    #[serde(default)]
    pub commands: Vec<CommandContribution>,
    /// ファイルと言語の対応（`language` 権限）。
    #[serde(default)]
    pub languages: Vec<LanguageContribution>,
    /// 言語サーバーの起動方法（`language` 権限の `servers`）。
    #[serde(default)]
    pub language_servers: Vec<LanguageServerContribution>,
    /// ドックに置くビュー（ui@1、`ui: [panel.<場所>]` の権限）。
    #[serde(default)]
    pub views: Vec<ViewContribution>,
    /// 利用者が拡張機能の画面で変えられる設定（権限不要。値は `get-setting` で読む）。
    #[serde(default)]
    pub settings: Vec<SettingContribution>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SettingKind {
    /// `options` の中から1つ選ぶ。
    Enum,
    Boolean,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SettingOption {
    pub value: String,
    pub label: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SettingContribution {
    pub key: String,
    pub title: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(rename = "type")]
    pub kind: SettingKind,
    #[serde(default)]
    pub options: Vec<SettingOption>,
    pub default: serde_json::Value,
}

impl SettingContribution {
    /// この設定の値として正しいか。
    pub fn accepts(&self, v: &serde_json::Value) -> bool {
        match self.kind {
            SettingKind::Boolean => v.is_boolean(),
            SettingKind::Enum => v.as_str().is_some_and(|s| self.options.iter().any(|o| o.value == s)),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ViewLocation {
    Left,
    Right,
    Bottom,
    /// それぞれのエディタの右側（MiniMap など）。
    Editor,
}

impl ViewLocation {
    pub fn permission(self) -> &'static str {
        match self {
            ViewLocation::Left => "panel.left",
            ViewLocation::Right => "panel.right",
            ViewLocation::Bottom => "panel.bottom",
            ViewLocation::Editor => "editor.side",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ViewContribution {
    pub id: String,
    pub title: String,
    /// Lucide のアイコン名（kebab-case）。
    #[serde(default)]
    pub icon: Option<String>,
    pub location: ViewLocation,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CommandContribution {
    pub id: String,
    pub title: String,
    #[serde(default)]
    pub key: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LanguageContribution {
    /// tree-sitter の文法名（gpui-kit の言語名）。例: `rust`
    pub name: String,
    /// 拡張子（`.` なし）。
    #[serde(default)]
    pub extensions: Vec<String>,
    /// ファイル名そのもの（例: `Dockerfile`）。
    #[serde(default)]
    pub file_names: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LanguageServerContribution {
    /// 対象の言語名（`languages` の `name`）。
    pub language: String,
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub initialization_options: Option<serde_json::Value>,
}

/// 権限の一覧（docs/ARCHITECTURE.md §4.2）。`(カテゴリ, 値, 強い権限か)`。
/// `network` と `process` の値はホスト名・コマンド名なので自由。
const KNOWN: &[(&str, &str, bool)] = &[
    ("theme", "colors", false),
    ("theme", "tokens", false),
    ("theme", "icons", false),
    ("locale", "messages", false),
    ("locale", "pack", false),
    ("language", "grammars", false),
    ("language", "servers", true),
    ("language", "formatters", false),
    ("editor", "read", false),
    ("editor", "edit", true),
    ("editor", "decorate", false),
    ("workspace", "fs.read", false),
    ("workspace", "fs.write", true),
    ("ui", "panel.left", false),
    ("ui", "panel.right", false),
    ("ui", "panel.bottom", false),
    ("ui", "editor.side", false),
    ("ui", "tab", false),
    ("ui", "overlay", false),
    ("ui", "statusbar", false),
    ("ui", "titlebar", false),
    ("ui", "menu", false),
];

/// 1つの権限（表示用）。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Permission {
    pub category: String,
    pub value: String,
    pub strong: bool,
}

impl Manifest {
    pub fn from_yaml(src: &str) -> Result<Manifest, String> {
        serde_yaml_ng::from_str(src).map_err(|e| e.to_string())
    }

    pub fn has(&self, category: &str, value: &str) -> bool {
        self.permissions
            .get(category)
            .is_some_and(|v| v.iter().any(|x| x == value))
    }

    /// 宣言された権限（強い権限の印つき）。
    pub fn permission_list(&self) -> Vec<Permission> {
        let mut out = Vec::new();
        for (cat, values) in &self.permissions {
            for v in values {
                let strong = match cat.as_str() {
                    "network" | "process" => true,
                    "ui" if v.starts_with("replace.") => true,
                    _ => KNOWN.iter().any(|(c, k, s)| c == cat && k == v && *s),
                };
                out.push(Permission {
                    category: cat.clone(),
                    value: v.clone(),
                    strong,
                });
            }
        }
        out
    }

    pub fn uses_ui(&self) -> bool {
        self.permissions.contains_key("ui")
    }

    /// 検証する。エラーがあれば読み込まない。
    pub fn validate(&self) -> Vec<String> {
        let mut errors = Vec::new();
        let id_ok = |s: &str| {
            let parts: Vec<&str> = s.split('.').collect();
            parts.len() >= 2
                && parts.iter().all(|p| {
                    !p.is_empty()
                        && p.chars()
                            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
                })
        };
        if !id_ok(&self.id) {
            errors.push(format!(
                "id `{}` は `<publisher>.<name>`（小文字・数字・-）で書く",
                self.id
            ));
        }
        if !self.id.starts_with(&format!("{}.", self.publisher)) {
            errors.push(format!("id `{}` は publisher `{}` で始める", self.id, self.publisher));
        }
        for (cat, values) in &self.permissions {
            match cat.as_str() {
                "network" | "process" => {
                    if values.iter().any(|v| v.is_empty() || v.contains('*')) {
                        errors.push(format!("`{cat}` には具体的な名前を書く（ワイルドカード不可）"));
                    }
                }
                "ui" => {
                    for v in values {
                        let known = v.starts_with("replace.") || KNOWN.iter().any(|(c, k, _)| *c == "ui" && k == v);
                        if !known {
                            errors.push(format!("未知の権限 `ui.{v}`"));
                        }
                    }
                }
                _ => {
                    if !KNOWN.iter().any(|(c, _, _)| c == cat) {
                        errors.push(format!("未知の権限カテゴリ `{cat}`"));
                        continue;
                    }
                    for v in values {
                        if !KNOWN.iter().any(|(c, k, _)| c == cat && k == v) {
                            errors.push(format!("未知の権限 `{cat}.{v}`"));
                        }
                    }
                }
            }
        }
        // API の版
        match self.api.core {
            Some(v) if v > CORE_API => {
                errors.push(format!("api.core {v} はこの Nucleus では使えない（{CORE_API} まで）"))
            }
            _ => {}
        }
        if self.main.is_some() && self.api.core.is_none() {
            errors.push("main（WASM）を使うときは api.core を書く".into());
        }
        if !self.contributes.commands.is_empty() && self.main.is_none() {
            errors.push("contributes.commands を実行するには main（WASM）が必要".into());
        }
        for c in &self.contributes.commands {
            if !c
                .id
                .starts_with(&format!("{}.", self.id.split_once('.').map_or("", |(_, n)| n)))
            {
                errors.push(format!(
                    "コマンド `{}` は `{}.` で始める",
                    c.id,
                    self.id.split_once('.').map_or("", |(_, n)| n)
                ));
            }
        }
        let mut keys = std::collections::HashSet::new();
        for s in &self.contributes.settings {
            if !keys.insert(&s.key) {
                errors.push(format!("設定 `{}` が2回ある", s.key));
            }
            if s.kind == SettingKind::Enum && s.options.is_empty() {
                errors.push(format!("設定 `{}` には options が必要", s.key));
            }
            if !s.accepts(&s.default) {
                errors.push(format!("設定 `{}` の default が正しくない", s.key));
            }
        }
        for v in &self.contributes.views {
            if !self.has("ui", v.location.permission()) {
                errors.push(format!(
                    "ビュー `{}` を置くには `ui: [{}]` の権限が必要",
                    v.id,
                    v.location.permission()
                ));
            }
            if self.main.is_none() {
                errors.push(format!("ビュー `{}` を描くには main（WASM）が必要", v.id));
            }
        }
        if self.uses_ui() && self.api.ui.is_none() {
            errors.push("ui 権限を使うときは api.ui を書く".into());
        }
        // 宣言と権限の対応
        let need = |cond: bool, cat: &str, v: &str, what: &str, errors: &mut Vec<String>| {
            if cond && !self.has(cat, v) {
                errors.push(format!("{what} には `{cat}: [{v}]` の権限が必要"));
            }
        };
        need(!self.tokens.is_empty(), "theme", "tokens", "tokens", &mut errors);
        need(!self.themes.is_empty(), "theme", "colors", "themes", &mut errors);
        need(self.locale.is_some(), "locale", "pack", "locale", &mut errors);
        if self.l10n.is_some() && !self.has("locale", "messages") && !self.has("locale", "pack") {
            errors.push("l10n には `locale: [messages]` か `locale: [pack]` の権限が必要".into());
        }
        need(
            !self.contributes.languages.is_empty(),
            "language",
            "grammars",
            "contributes.languages",
            &mut errors,
        );
        need(
            !self.contributes.language_servers.is_empty(),
            "language",
            "servers",
            "contributes.language_servers",
            &mut errors,
        );
        for key in self.tokens.keys() {
            if !key.starts_with(&format!("{}.", self.id)) {
                errors.push(format!("Token `{key}` は `{}.` で始める", self.id));
            }
        }
        if self.has("language", "servers") {
            for s in &self.contributes.language_servers {
                if !self.has("process", &s.command) {
                    errors.push(format!(
                        "言語サーバー `{}` を起動するには `process: [{}]` の権限が必要",
                        s.command, s.command
                    ));
                }
            }
        }
        errors
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CLAUDE: &str = r##"
id: anthropic.claude
name: Claude
version: 1.2.0
publisher: anthropic
api: { core: 1, ui: 1 }
permissions:
  ui: [panel.right, tab, statusbar]
  editor: [read, edit]
  network: [api.anthropic.com]
  theme: [tokens]
tokens:
  anthropic.claude.brand: { dark: "#d97757", light: "#c96442", kind: brand }
contributes:
  commands:
    - { id: claude.ask, title: "%cmd.ask%", key: "ctrl-alt-c" }
main: claude.wasm
"##;

    #[test]
    fn parses_and_validates_the_architecture_example() {
        let m = Manifest::from_yaml(CLAUDE).unwrap();
        assert!(m.validate().is_empty(), "{:?}", m.validate());
        let strong: Vec<_> = m
            .permission_list()
            .into_iter()
            .filter(|p| p.strong)
            .map(|p| p.value)
            .collect();
        assert_eq!(strong, vec!["edit", "api.anthropic.com"]);
        assert!(m.uses_ui());
    }

    #[test]
    fn rejects_undeclared_features_and_unknown_permissions() {
        let m = Manifest::from_yaml(
            r##"
id: someone.thing
name: T
version: 0.1.0
publisher: someone
permissions: { theme: [colorz], magic: [x], network: ["*"] }
tokens: { other.thing.c: { dark: "#000000", light: "#ffffff" } }
themes: [a.json]
"##,
        )
        .unwrap();
        let e = m.validate().join("\n");
        assert!(e.contains("theme.colorz"));
        assert!(e.contains("magic"));
        assert!(e.contains("ワイルドカード"));
        assert!(e.contains("themes には `theme: [colors]`"));
        assert!(e.contains("Token `other.thing.c`"));
    }

    #[test]
    fn ui_needs_an_api_version_and_servers_need_process_permission() {
        let m = Manifest::from_yaml(
            r#"
id: x.y
name: Y
version: 1.0.0
publisher: x
permissions: { ui: [tab], language: [servers] }
contributes:
  language_servers: [{ language: rust, command: rust-analyzer }]
"#,
        )
        .unwrap();
        let e = m.validate().join("\n");
        assert!(e.contains("api.ui"));
        assert!(e.contains("process: [rust-analyzer]"));
    }

    #[test]
    fn unknown_fields_are_errors() {
        assert!(Manifest::from_yaml("id: a.b\nname: n\nversion: 1\npublisher: a\nmian: x.wasm\n").is_err());
    }
}
