//! 設定（docs/ARCHITECTURE.md §7）。既定値 → ユーザー → ワークスペースの順に重ねる。gpui に依存しない。

mod jsonc;
mod model;
mod write;

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::Value;

pub use jsonc::strip_comments;
pub use model::*;
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
    /// 知らない項目（無視して、ほかの項目は反映する。古い版の項目や書き間違い）。
    pub warnings: Vec<String>,
}

/// 層を重ねて設定を作る。存在しないファイルは空の層として扱う。バックグラウンドで呼ぶこと。
pub fn load(user: Option<&Path>, workspace: Option<&Path>) -> Loaded {
    let mut errors = Vec::new();
    let mut warnings = Vec::new();
    let defaults = serde_json::to_value(Settings::default()).expect("defaults serialize");
    let mut merged = defaults.clone();
    for path in [user, workspace].into_iter().flatten() {
        match read_layer(path) {
            Ok(Some(mut layer)) => {
                for key in tidy(&mut layer, &defaults) {
                    warnings.push(format!("{}: unknown setting `{key}` (ignored)", path.display()));
                }
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
    Loaded {
        settings,
        errors,
        warnings,
    }
}

/// 知らない項目を取り除き、その名前を返す。
/// 古い版の書き方（`editor` の外の `font_size` など）は、`editor` の中へ移す（`editor` 側にあればそちらを優先）。
fn tidy(layer: &mut Value, defaults: &Value) -> Vec<String> {
    let (Value::Object(l), Value::Object(d)) = (layer, defaults) else {
        return Vec::new();
    };
    let editor_keys: Vec<String> = d
        .get("editor")
        .and_then(Value::as_object)
        .map(|e| e.keys().cloned().collect())
        .unwrap_or_default();
    let mut unknown = Vec::new();
    for key in l.keys().cloned().collect::<Vec<_>>() {
        if d.contains_key(&key) {
            continue;
        }
        let value = l.remove(&key).expect("key exists");
        if editor_keys.contains(&key) {
            let editor = l.entry("editor").or_insert_with(|| Value::Object(Default::default()));
            if let Value::Object(e) = editor {
                e.entry(key).or_insert(value);
            }
        } else {
            unknown.push(key);
        }
    }
    // 項目の決まっているまとまり（editor、files など）の中も確かめる。
    for group in GROUPS {
        if let (Some(Value::Object(lg)), Some(Value::Object(dg))) = (l.get_mut(*group), d.get(*group)) {
            lg.retain(|k, _| {
                let known = dg.contains_key(k);
                if !known {
                    unknown.push(format!("{group}.{k}"));
                }
                known
            });
        }
    }
    unknown
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

        fs::write(&ws, r#"{ "edtor": {}, "theme": "X" }"#).unwrap();
        let l = load(None, Some(&ws));
        assert!(l.errors.is_empty(), "unknown keys do not reject the layer");
        assert!(l.warnings[0].contains("edtor"), "typos are reported");
        assert_eq!(l.settings.theme, "X");
        fs::write(&ws, "{ not json").unwrap();
        assert_eq!(load(None, Some(&ws)).errors.len(), 1);
    }

    #[test]
    fn old_top_level_editor_keys_move_into_editor() {
        let d = tempfile::tempdir().unwrap();
        let user = d.path().join("user.json");
        fs::write(&user, r#"{ "font_size": 18, "theme": "Nucleus One Light" }"#).unwrap();
        let l = load(Some(&user), None);
        assert!(
            l.errors.is_empty() && l.warnings.is_empty(),
            "{:?} {:?}",
            l.errors,
            l.warnings
        );
        assert_eq!(l.settings.editor.font_size, 18.0);
        assert_eq!(l.settings.theme, "Nucleus One Light");
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
