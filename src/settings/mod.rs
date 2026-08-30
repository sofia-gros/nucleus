/// グローバル設定（User）およびワークスペース設定（Workspace）の階層化管理モジュール

use serde_json::{Value, json};
use std::sync::{Arc, RwLock};
use std::path::PathBuf;
use gpui::*;

/// 設定のスコープ対象
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SettingsTarget {
    User,
    Workspace,
}

/// 設定グループ（カテゴリ）
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SettingGroup {
    All,
    Appearance,
    Editor,
    Files,
    Terminal,
    LanguagesAndLsp,
    Debug,
    Git,
    Plugins,
}

impl SettingGroup {
    pub fn label(&self) -> &'static str {
        match self {
            Self::All => "All Settings",
            Self::Appearance => "Appearance",
            Self::Editor => "Editor",
            Self::Files => "Files",
            Self::Terminal => "Terminal",
            Self::LanguagesAndLsp => "Languages & LSP",
            Self::Debug => "Debug",
            Self::Git => "Git",
            Self::Plugins => "Plugins",
        }
    }
}

/// 設定項目の入力型
#[derive(Clone, Debug, PartialEq)]
pub enum SettingType {
    Bool,
    Number { min: f64, max: f64, step: f64 },
    String,
    Select(Vec<&'static str>),
}

/// 設定項目のスキーマ定義
#[derive(Clone, Debug)]
pub struct SettingDefinition {
    pub key: &'static str,
    pub group: SettingGroup,
    pub label: &'static str,
    pub description: &'static str,
    pub setting_type: SettingType,
    pub default_value: Value,
}

/// 階層化設定ストア
pub struct SettingsStore {
    pub global_settings: Value,
    pub workspace_settings: Value,
    pub workspace_root: Option<PathBuf>,
}

impl SettingsStore {
    /// 新規作成と設定の読み込み
    pub fn new() -> Self {
        let default_global = Self::default_catalog_json();

        let mut store = Self {
            global_settings: default_global,
            workspace_settings: Value::Object(serde_json::Map::new()),
            workspace_root: None,
        };
        store.load_global();
        store.load_workspace();
        store
    }

    /// 全設定項目のスキーマカタログ
    pub fn get_schema_catalog() -> Vec<SettingDefinition> {
        vec![
            // Appearance
            SettingDefinition { key: "workbench.colorTheme", group: SettingGroup::Appearance, label: "Color Theme", description: "エディタ全体のカラーテーマを設定します。", setting_type: SettingType::Select(vec!["dark", "light", "nord", "monokai", "high-contrast"]), default_value: json!("dark") },
            SettingDefinition { key: "workbench.iconTheme", group: SettingGroup::Appearance, label: "Icon Theme", description: "ファイルツリーのアイコンテーマを設定します。", setting_type: SettingType::Select(vec!["default", "minimal", "none"]), default_value: json!("default") },
            SettingDefinition { key: "workbench.tree.indent", group: SettingGroup::Appearance, label: "Tree Indent", description: "エクスプローラーツリーのインデント幅（px）を設定します。", setting_type: SettingType::Number { min: 8.0, max: 40.0, step: 2.0 }, default_value: json!(16) },
            SettingDefinition { key: "workbench.activityBar.visible", group: SettingGroup::Appearance, label: "Activity Bar Visible", description: "アクティビティバーを表示するかどうかを設定します。", setting_type: SettingType::Bool, default_value: json!(true) },
            SettingDefinition { key: "workbench.statusBar.visible", group: SettingGroup::Appearance, label: "Status Bar Visible", description: "ステータスバーを表示するかどうかを設定します。", setting_type: SettingType::Bool, default_value: json!(true) },
            SettingDefinition { key: "window.zoomLevel", group: SettingGroup::Appearance, label: "Zoom Level", description: "ウィンドウ全体のUI拡大率を設定します。", setting_type: SettingType::Select(vec!["1.0", "1.25", "1.5", "1.75"]), default_value: json!("1.0") },

            // Editor
            SettingDefinition { key: "editor.fontSize", group: SettingGroup::Editor, label: "Font Size", description: "エディタのフォントサイズ（px）を設定します。", setting_type: SettingType::Number { min: 8.0, max: 48.0, step: 1.0 }, default_value: json!(14) },
            SettingDefinition { key: "editor.fontFamily", group: SettingGroup::Editor, label: "Font Family", description: "エディタで使用するフォントファミリーを設定します。", setting_type: SettingType::String, default_value: json!("Consolas, 'Courier New', monospace") },
            SettingDefinition { key: "editor.lineHeight", group: SettingGroup::Editor, label: "Line Height", description: "エディタの行高倍率を設定します。", setting_type: SettingType::Number { min: 1.0, max: 3.0, step: 0.1 }, default_value: json!(1.5) },
            SettingDefinition { key: "editor.tabSize", group: SettingGroup::Editor, label: "Tab Size", description: "タブ文字が相当するスペース数を設定します。", setting_type: SettingType::Number { min: 1.0, max: 8.0, step: 1.0 }, default_value: json!(4) },
            SettingDefinition { key: "editor.insertSpaces", group: SettingGroup::Editor, label: "Insert Spaces", description: "Tabキー入力時にスペースを挿入するかどうかを設定します。", setting_type: SettingType::Bool, default_value: json!(true) },
            SettingDefinition { key: "editor.wordWrap", group: SettingGroup::Editor, label: "Word Wrap", description: "エディタ行の折り返し方法を設定します。", setting_type: SettingType::Select(vec!["off", "on", "wordWrapColumn"]), default_value: json!("off") },
            SettingDefinition { key: "editor.lineNumbers", group: SettingGroup::Editor, label: "Line Numbers", description: "行番号の表示形式を設定します。", setting_type: SettingType::Select(vec!["on", "off", "relative"]), default_value: json!("on") },
            SettingDefinition { key: "editor.renderWhitespace", group: SettingGroup::Editor, label: "Render Whitespace", description: "空白文字の可視化方法を設定します。", setting_type: SettingType::Select(vec!["none", "boundary", "selection", "all"]), default_value: json!("none") },
            SettingDefinition { key: "editor.cursorBlinking", group: SettingGroup::Editor, label: "Cursor Blinking", description: "カーソルの点滅アニメーションを設定します。", setting_type: SettingType::Select(vec!["blink", "smooth", "solid"]), default_value: json!("blink") },
            SettingDefinition { key: "editor.cursorStyle", group: SettingGroup::Editor, label: "Cursor Style", description: "カーソルの形状を設定します。", setting_type: SettingType::Select(vec!["line", "block", "underline"]), default_value: json!("line") },
            SettingDefinition { key: "editor.bracketPairColorization.enabled", group: SettingGroup::Editor, label: "Bracket Pair Colorization", description: "対応する括弧ペアを階層別に色分け表示します。", setting_type: SettingType::Bool, default_value: json!(true) },
            SettingDefinition { key: "editor.guides.indentation", group: SettingGroup::Editor, label: "Indentation Guides", description: "インデントガイド縦線を表示します。", setting_type: SettingType::Bool, default_value: json!(true) },
            SettingDefinition { key: "editor.minimap.enabled", group: SettingGroup::Editor, label: "Minimap Enabled", description: "コードの全体ミニマップを表示します。", setting_type: SettingType::Bool, default_value: json!(true) },
            SettingDefinition { key: "editor.scrollBeyondLastLine", group: SettingGroup::Editor, label: "Scroll Beyond Last Line", description: "最終行を超えて下部にスクロール可能にします。", setting_type: SettingType::Bool, default_value: json!(true) },
            SettingDefinition { key: "editor.formatOnSave", group: SettingGroup::Editor, label: "Format On Save", description: "ファイル保存時に自動でフォーマッタを実行します。", setting_type: SettingType::Bool, default_value: json!(false) },
            SettingDefinition { key: "editor.formatOnPaste", group: SettingGroup::Editor, label: "Format On Paste", description: "貼り付け時に自動でフォーマットを適用します。", setting_type: SettingType::Bool, default_value: json!(false) },
            SettingDefinition { key: "editor.autoClosingBrackets", group: SettingGroup::Editor, label: "Auto Closing Brackets", description: "括弧入力時の自動閉じ動作を設定します。", setting_type: SettingType::Select(vec!["always", "languageDefined", "never"]), default_value: json!("always") },
            SettingDefinition { key: "editor.autoClosingQuotes", group: SettingGroup::Editor, label: "Auto Closing Quotes", description: "引用符入力時の自動閉じ動作を設定します。", setting_type: SettingType::Select(vec!["always", "languageDefined", "never"]), default_value: json!("always") },

            // Files
            SettingDefinition { key: "files.autoSave", group: SettingGroup::Files, label: "Auto Save", description: "ファイルの自動保存モードを設定します。", setting_type: SettingType::Select(vec!["off", "afterDelay", "onFocusChange"]), default_value: json!("afterDelay") },
            SettingDefinition { key: "files.autoSaveDelay", group: SettingGroup::Files, label: "Auto Save Delay", description: "自動保存の遅延時間（秒）を設定します。", setting_type: SettingType::Number { min: 1.0, max: 300.0, step: 1.0 }, default_value: json!(30) },
            SettingDefinition { key: "files.encoding", group: SettingGroup::Files, label: "Files Encoding", description: "既定の文字コードを設定します。", setting_type: SettingType::Select(vec!["utf-8", "shift_jis", "euc-jp"]), default_value: json!("utf-8") },
            SettingDefinition { key: "files.eol", group: SettingGroup::Files, label: "End of Line", description: "既定の改行コードを設定します。", setting_type: SettingType::Select(vec!["\n", "\r\n", "auto"]), default_value: json!("\n") },
            SettingDefinition { key: "files.trimTrailingWhitespace", group: SettingGroup::Files, label: "Trim Trailing Whitespace", description: "ファイル保存時に行末の不要な空白を自動削除します。", setting_type: SettingType::Bool, default_value: json!(false) },
            SettingDefinition { key: "files.insertFinalNewline", group: SettingGroup::Files, label: "Insert Final Newline", description: "ファイル保存時に末尾に改行を自動挿入します。", setting_type: SettingType::Bool, default_value: json!(true) },

            // Terminal
            SettingDefinition { key: "terminal.integrated.fontSize", group: SettingGroup::Terminal, label: "Terminal Font Size", description: "ターミナルのフォントサイズを設定します。", setting_type: SettingType::Number { min: 8.0, max: 32.0, step: 1.0 }, default_value: json!(13) },
            SettingDefinition { key: "terminal.integrated.fontFamily", group: SettingGroup::Terminal, label: "Terminal Font Family", description: "ターミナルのフォントファミリーを設定します。", setting_type: SettingType::String, default_value: json!("Consolas, monospace") },
            SettingDefinition { key: "terminal.integrated.shell.windows", group: SettingGroup::Terminal, label: "Default Shell (Windows)", description: "Windows環境で起動する既定のシェルを設定します。", setting_type: SettingType::Select(vec!["pwsh.exe", "powershell.exe", "cmd.exe", "bash.exe"]), default_value: json!("pwsh.exe") },
            SettingDefinition { key: "terminal.integrated.cursorBlinking", group: SettingGroup::Terminal, label: "Terminal Cursor Blinking", description: "ターミナルカーソルの点滅を有効にします。", setting_type: SettingType::Bool, default_value: json!(true) },
            SettingDefinition { key: "terminal.integrated.scrollback", group: SettingGroup::Terminal, label: "Scrollback", description: "ターミナルが保持する最大スクロールバック行数を設定します。", setting_type: SettingType::Number { min: 100.0, max: 10000.0, step: 100.0 }, default_value: json!(1000) },

            // Languages & LSP
            SettingDefinition { key: "lsp.inlayHints.enabled", group: SettingGroup::LanguagesAndLsp, label: "Inlay Hints", description: "インレイヒント（型推論や引数名）の表示を有効にします。", setting_type: SettingType::Bool, default_value: json!(true) },
            SettingDefinition { key: "lsp.inlayHints.parameterNames", group: SettingGroup::LanguagesAndLsp, label: "Parameter Names Inlay Hints", description: "関数呼び出し時の引数名インレイヒントを表示します。", setting_type: SettingType::Bool, default_value: json!(true) },
            SettingDefinition { key: "lsp.inlayHints.typeHints", group: SettingGroup::LanguagesAndLsp, label: "Type Inlay Hints", description: "変数の型推論インレイヒントを表示します。", setting_type: SettingType::Bool, default_value: json!(true) },
            SettingDefinition { key: "lsp.signatureHelp.enabled", group: SettingGroup::LanguagesAndLsp, label: "Signature Help", description: "引数入力時のシグネチャヘルプツールチップを表示します。", setting_type: SettingType::Bool, default_value: json!(true) },
            SettingDefinition { key: "lsp.diagnostics.showInline", group: SettingGroup::LanguagesAndLsp, label: "Inline Diagnostics", description: "エディタ行内にエラーや警告のテキストを表示します。", setting_type: SettingType::Bool, default_value: json!(true) },
            SettingDefinition { key: "lsp.servers.rust.command", group: SettingGroup::LanguagesAndLsp, label: "Rust LSP Server Command", description: "Rust 言語サーバーの起動コマンドを設定します。", setting_type: SettingType::String, default_value: json!("rust-analyzer") },
            SettingDefinition { key: "lsp.servers.go.command", group: SettingGroup::LanguagesAndLsp, label: "Go LSP Server Command", description: "Go 言語サーバーの起動コマンドを設定します。", setting_type: SettingType::String, default_value: json!("gopls") },
            SettingDefinition { key: "lsp.servers.python.command", group: SettingGroup::LanguagesAndLsp, label: "Python LSP Server Command", description: "Python 言語サーバーの起動コマンドを設定します。", setting_type: SettingType::String, default_value: json!("pyright-langserver") },

            // Debug
            SettingDefinition { key: "debug.console.fontSize", group: SettingGroup::Debug, label: "Debug Console Font Size", description: "デバッグコンソールのフォントサイズを設定します。", setting_type: SettingType::Number { min: 8.0, max: 24.0, step: 1.0 }, default_value: json!(13) },
            SettingDefinition { key: "debug.openDebugOnBreak", group: SettingGroup::Debug, label: "Open Debug On Break", description: "ブレークポイント停止時に自動でデバッグパネルを開きます。", setting_type: SettingType::Bool, default_value: json!(true) },
            SettingDefinition { key: "debug.adapters.lldb.command", group: SettingGroup::Debug, label: "LLDB DAP Command", description: "LLDB デバッグアダプタの起動コマンドを設定します。", setting_type: SettingType::String, default_value: json!("lldb-dap") },

            // Git
            SettingDefinition { key: "git.enabled", group: SettingGroup::Git, label: "Git Enabled", description: "Git ソース管理機能を有効にします。", setting_type: SettingType::Bool, default_value: json!(true) },
            SettingDefinition { key: "git.autoFetch", group: SettingGroup::Git, label: "Auto Fetch", description: "リモートリポジトリからの自動フェッチを有効にします。", setting_type: SettingType::Bool, default_value: json!(false) },
            SettingDefinition { key: "git.confirmSync", group: SettingGroup::Git, label: "Confirm Sync", description: "Git同期実行時に確認ダイアログを表示します。", setting_type: SettingType::Bool, default_value: json!(true) },
            SettingDefinition { key: "git.enableSmartCommit", group: SettingGroup::Git, label: "Smart Commit", description: "ステージングされた変更がない場合に全変更を自動コミットします。", setting_type: SettingType::Bool, default_value: json!(false) },
            SettingDefinition { key: "git.path", group: SettingGroup::Git, label: "Git Executable Path", description: "Git 実行可能ファイルのパスを設定します。", setting_type: SettingType::String, default_value: json!("git") },

            // Plugins
            SettingDefinition { key: "plugins.autoUpdate", group: SettingGroup::Plugins, label: "Plugins Auto Update", description: "WASMプラグインの自動更新を有効にします。", setting_type: SettingType::Bool, default_value: json!(true) },
            SettingDefinition { key: "plugins.allowUnsigned", group: SettingGroup::Plugins, label: "Allow Unsigned Plugins", description: "未署名のローカルプラグインの実行を許可します。", setting_type: SettingType::Bool, default_value: json!(false) },
            SettingDefinition { key: "plugins.customPath", group: SettingGroup::Plugins, label: "Custom Plugins Directory", description: "追加のプラグインディレクトリパスを設定します。", setting_type: SettingType::String, default_value: json!("") },
        ]
    }

    /// グループごとの設定項目一覧
    pub fn get_items_by_group(group: SettingGroup) -> Vec<SettingDefinition> {
        let catalog = Self::get_schema_catalog();
        if group == SettingGroup::All {
            catalog
        } else {
            catalog.into_iter().filter(|i| i.group == group).collect()
        }
    }

    fn default_catalog_json() -> Value {
        let mut map = serde_json::Map::new();
        for item in Self::get_schema_catalog() {
            let parts: Vec<&str> = item.key.split('.').collect();
            let mut current = &mut map;
            for (i, part) in parts.iter().enumerate() {
                if i == parts.len() - 1 {
                    current.insert(part.to_string(), item.default_value.clone());
                } else {
                    if !current.contains_key(*part) || !current.get(*part).unwrap().is_object() {
                        current.insert(part.to_string(), Value::Object(serde_json::Map::new()));
                    }
                    current = current.get_mut(*part).unwrap().as_object_mut().unwrap();
                }
            }
        }
        Value::Object(map)
    }

    /// ワークスペースルートパスの設定
    pub fn set_workspace_root(&mut self, root: Option<PathBuf>) {
        self.workspace_root = root;
        self.load_workspace();
    }

    fn global_settings_path() -> Option<PathBuf> {
        dirs::config_dir().map(|mut p| {
            p.push("nucleus");
            p.push("settings.json");
            p
        })
    }

    fn workspace_settings_path(&self) -> Option<PathBuf> {
        self.workspace_root.as_ref().map(|root| root.join(".nucleus").join("workspace.json"))
            .or_else(|| Some(PathBuf::from(".nucleus").join("workspace.json")))
    }

    /// グローバル設定の読み込み
    pub fn load_global(&mut self) {
        if let Some(path) = Self::global_settings_path() {
            if let Ok(content) = std::fs::read_to_string(path) {
                if let Ok(val) = serde_json::from_str(&content) {
                    self.global_settings = val;
                }
            }
        }
    }

    /// ワークスペース設定の読み込み
    pub fn load_workspace(&mut self) {
        if let Some(path) = self.workspace_settings_path() {
            if let Ok(content) = std::fs::read_to_string(path) {
                if let Ok(val) = serde_json::from_str(&content) {
                    self.workspace_settings = val;
                }
            }
        }
    }

    /// ワークスペース設定のファイル保存
    pub fn save_workspace(&self) {
        if let Some(path) = self.workspace_settings_path() {
            if let Some(parent) = path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            if let Ok(json_str) = serde_json::to_string_pretty(&self.workspace_settings) {
                let _ = std::fs::write(path, json_str);
            }
        }
    }

    /// グローバル設定のファイル保存
    pub fn save_global(&self) {
        if let Some(path) = Self::global_settings_path() {
            if let Some(parent) = path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            if let Ok(json_str) = serde_json::to_string_pretty(&self.global_settings) {
                let _ = std::fs::write(path, json_str);
            }
        }
    }

    /// 階層マージされた設定値の取得（Workspace 優先、なければ Global）
    pub fn get(&self, key: &str) -> Option<Value> {
        let parts: Vec<&str> = key.split('.').collect();
        if let Some(val) = Self::get_from_value(&self.workspace_settings, &parts) {
            return Some(val);
        }
        Self::get_from_value(&self.global_settings, &parts)
    }

    /// User (Global) 設定の個別取得
    pub fn get_user(&self, key: &str) -> Option<Value> {
        let parts: Vec<&str> = key.split('.').collect();
        Self::get_from_value(&self.global_settings, &parts)
    }

    /// Workspace 設定の個別取得
    pub fn get_workspace(&self, key: &str) -> Option<Value> {
        let parts: Vec<&str> = key.split('.').collect();
        Self::get_from_value(&self.workspace_settings, &parts)
    }

    fn get_from_value(value: &Value, parts: &[&str]) -> Option<Value> {
        let mut current = value;
        for part in parts {
            if let Some(obj) = current.as_object() {
                if let Some(next) = obj.get(*part) {
                    current = next;
                } else {
                    return None;
                }
            } else {
                return None;
            }
        }
        Some(current.clone())
    }

    /// 設定値の保存（デフォルトは Workspace があれば Workspace、なければ Global）
    pub fn set(&mut self, key: &str, value: Value) {
        self.set_target(SettingsTarget::Workspace, key, value);
    }

    /// 対象スコープを指定して設定値を保存
    pub fn set_target(&mut self, target: SettingsTarget, key: &str, value: Value) {
        let parts: Vec<&str> = key.split('.').collect();
        match target {
            SettingsTarget::User => {
                Self::set_to_value(&mut self.global_settings, &parts, value);
                self.save_global();
            }
            SettingsTarget::Workspace => {
                Self::set_to_value(&mut self.workspace_settings, &parts, value);
                self.save_workspace();
            }
        }
    }

    fn set_to_value(root: &mut Value, parts: &[&str], value: Value) {
        if !root.is_object() {
            *root = json!({});
        }
        let mut current = root;
        for (i, part) in parts.iter().enumerate() {
            if i == parts.len() - 1 {
                if let Some(obj) = current.as_object_mut() {
                    obj.insert(part.to_string(), value);
                    return;
                }
            } else {
                let obj = current.as_object_mut().unwrap();
                if !obj.contains_key(*part) || !obj.get(*part).unwrap().is_object() {
                    obj.insert(part.to_string(), Value::Object(serde_json::Map::new()));
                }
                current = obj.get_mut(*part).unwrap();
            }
        }
    }
}

pub struct SettingsGlobal(pub Arc<RwLock<SettingsStore>>);

impl Global for SettingsGlobal {}
