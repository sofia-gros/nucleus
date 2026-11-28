//! 設定（docs/ARCHITECTURE.md §7）。既定値 → ユーザー → ワークスペースの順に重ねる。gpui に依存しない。

mod jsonc;
mod write;

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::Value;

pub use jsonc::strip_comments;
pub use write::{set_in_text, set_value};

/// 有効な設定（gpui の Global）。変更を受け取りたい側は `cx.observe_global::<ActiveSettings>` を使う。
#[derive(Clone, Debug, Default)]
pub struct ActiveSettings(pub std::sync::Arc<Settings>);

impl gpui_kit::Global for ActiveSettings {}

impl ActiveSettings {
    pub fn get(cx: &gpui_kit::App) -> std::sync::Arc<Settings> {
        cx.try_global::<ActiveSettings>()
            .map(|s| s.0.clone())
            .unwrap_or_default()
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Settings {
    /// テーマの名前（例: "Nucleus One Dark" / "Nucleus One Light"）。
    pub theme: String,
    /// 表示言語（例: "en" / "ja"）。null なら OS の設定に従う。
    pub language: Option<String>,
    /// 起動時に前回のフォルダと開いていたファイルを復元する。
    pub restore_session: bool,
    pub editor: EditorSettings,
    /// 追加のキーバインド（変更は再起動後に反映）。
    pub keymap: Vec<KeymapEntry>,
    /// 無効にする Extension の ID。
    pub disabled_extensions: Vec<String>,
    /// Extension の一覧（index.json）の場所。null なら公式（sofia-gros/nucleus-extensions）。
    pub extensions_registry: Option<String>,
    /// 起動時に Extension の更新を確かめる。
    pub check_extension_updates: bool,
    /// Extension ごとの設定（拡張機能の画面で変える）。`{ "<id>": { "<key>": 値 } }`
    pub extension_settings: std::collections::BTreeMap<String, serde_json::Map<String, serde_json::Value>>,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            theme: "Nucleus One Dark".into(),
            language: None,
            restore_session: true,
            editor: EditorSettings::default(),
            keymap: Vec::new(),
            disabled_extensions: Vec::new(),
            extensions_registry: None,
            check_extension_updates: true,
            extension_settings: Default::default(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct EditorSettings {
    pub font_family: String,
    pub font_size: f32,
    pub tab_size: usize,
    pub hard_tabs: bool,
    pub soft_wrap: bool,
    pub line_numbers: bool,
    pub indent_guides: bool,
}

impl Default for EditorSettings {
    fn default() -> Self {
        EditorSettings {
            font_family: "JetBrains Mono".into(),
            font_size: 13.0,
            tab_size: 4,
            hard_tabs: false,
            soft_wrap: false,
            line_numbers: true,
            indent_guides: true,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KeymapEntry {
    /// 例: "ctrl-shift-k"
    pub key: String,
    /// アクション名。例: "workspace::CommandPalette"
    pub command: String,
    /// 例: "Workspace"。省略するとどこでも有効。
    #[serde(default)]
    pub context: Option<String>,
}

/// ユーザー設定ファイルの場所。
pub fn user_settings_path() -> Option<PathBuf> {
    dirs::config_dir().map(|d| d.join("nucleus").join("settings.json"))
}

/// ワークスペース設定ファイルの場所。
pub fn workspace_settings_path(root: &Path) -> PathBuf {
    root.join(".nucleus").join("settings.json")
}

/// 新しく作るユーザー設定ファイルの中身。
pub const USER_TEMPLATE: &str = r#"// Nucleus の設定。ここに書いた値が既定値を上書きします（保存すると即座に反映されます）。
// プロジェクトごとの設定は <フォルダ>/.nucleus/settings.json に書きます。
{
  // "theme": "Nucleus One Light",
  // "language": "ja",
  // "editor": { "font_size": 14, "tab_size": 2, "soft_wrap": true },
  // "keymap": [{ "key": "ctrl-shift-e", "command": "workspace::ToggleLeftDock" }]
}
"#;

/// 1つの層の読み込み結果。
#[derive(Debug, Default)]
pub struct Loaded {
    pub settings: Settings,
    /// 読めなかった層と理由（その層は無視される）。
    pub errors: Vec<String>,
}

/// 層を重ねて設定を作る。存在しないファイルは空の層として扱う。バックグラウンドで呼ぶこと。
pub fn load(user: Option<&Path>, workspace: Option<&Path>) -> Loaded {
    let mut errors = Vec::new();
    let mut merged = serde_json::to_value(Settings::default()).expect("defaults serialize");
    for path in [user, workspace].into_iter().flatten() {
        match read_layer(path) {
            Ok(Some(layer)) => {
                let mut candidate = merged.clone();
                merge(&mut candidate, layer);
                // 層ごとに検証し、壊れた層だけを捨てる。
                match serde_json::from_value::<Settings>(candidate.clone()) {
                    Ok(_) => merged = candidate,
                    Err(e) => errors.push(format!("{}: {e}", path.display())),
                }
            }
            Ok(None) => {}
            Err(e) => errors.push(format!("{}: {e}", path.display())),
        }
    }
    let settings = serde_json::from_value(merged).unwrap_or_default();
    Loaded { settings, errors }
}

fn read_layer(path: &Path) -> anyhow::Result<Option<Value>> {
    let text = match std::fs::read_to_string(path) {
        Ok(t) => t,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(e.into()),
    };
    let stripped = strip_comments(&text);
    if stripped.trim().is_empty() {
        return Ok(None);
    }
    let value: Value = serde_json::from_str(&stripped)?;
    if !value.is_object() {
        anyhow::bail!("設定は JSON のオブジェクト（{{ … }}）で書く");
    }
    Ok(Some(value))
}

/// オブジェクトは再帰的に重ね、それ以外（配列を含む）は置き換える。
fn merge(base: &mut Value, layer: Value) {
    match (base, layer) {
        (Value::Object(b), Value::Object(l)) => {
            for (k, v) in l {
                match b.get_mut(&k) {
                    Some(existing) => merge(existing, v),
                    None => {
                        b.insert(k, v);
                    }
                }
            }
        }
        (b, l) => *b = l,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn layers_override_in_order() {
        let d = tempfile::tempdir().unwrap();
        let user = d.path().join("user.json");
        let ws = d.path().join("ws.json");
        fs::write(
            &user,
            r#"// comment
            { "theme": "Nucleus One Light", "editor": { "font_size": 15, "tab_size": 2 } }"#,
        )
        .unwrap();
        fs::write(&ws, r#"{ "editor": { "tab_size": 8 } /* project */ }"#).unwrap();
        let l = load(Some(&user), Some(&ws));
        assert!(l.errors.is_empty(), "{:?}", l.errors);
        assert_eq!(l.settings.theme, "Nucleus One Light");
        assert_eq!(l.settings.editor.font_size, 15.0);
        assert_eq!(l.settings.editor.tab_size, 8);
        assert!(l.settings.editor.line_numbers, "untouched keys keep defaults");
    }

    #[test]
    fn missing_files_mean_defaults() {
        let l = load(Some(Path::new("/nonexistent/a.json")), None);
        assert!(l.errors.is_empty());
        assert_eq!(l.settings, Settings::default());
    }

    #[test]
    fn a_broken_layer_is_skipped_and_reported() {
        let d = tempfile::tempdir().unwrap();
        let user = d.path().join("user.json");
        let ws = d.path().join("ws.json");
        fs::write(&user, r#"{ "editor": { "font_size": 16 } }"#).unwrap();
        fs::write(&ws, r#"{ "editor": { "tab_size": "wide" } }"#).unwrap();
        let l = load(Some(&user), Some(&ws));
        assert_eq!(l.errors.len(), 1);
        assert_eq!(l.settings.editor.font_size, 16.0);
        assert_eq!(l.settings.editor.tab_size, 4);

        fs::write(&ws, r#"{ "edtor": {} }"#).unwrap();
        assert!(load(None, Some(&ws)).errors[0].contains("edtor"), "typos are reported");
        fs::write(&ws, "{ not json").unwrap();
        assert_eq!(load(None, Some(&ws)).errors.len(), 1);
    }

    #[test]
    fn template_parses() {
        let v: Value = serde_json::from_str(&strip_comments(USER_TEMPLATE)).unwrap();
        assert!(v.is_object());
    }

    #[test]
    fn keymap_entries_parse() {
        let d = tempfile::tempdir().unwrap();
        let user = d.path().join("user.json");
        fs::write(
            &user,
            r#"{ "keymap": [{ "key": "ctrl-k", "command": "workspace::FindFile", "context": "Workspace" }] }"#,
        )
        .unwrap();
        let l = load(Some(&user), None);
        assert_eq!(l.settings.keymap[0].command, "workspace::FindFile");
    }
}
