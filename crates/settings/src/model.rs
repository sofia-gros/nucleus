//! 設定の中身。VSCode の設定を手本にしている（名前は `editor.font_size` のように小文字と `_`）。
//!
//! どの項目も省略できる（省略すると既定値）。知らない項目は無視する（lib.rs の `tidy`）。

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::KeymapEntry;

/// 項目の決まっているまとまり（この中の知らない項目は警告して無視する）。
pub const GROUPS: &[&str] = &[
    "editor",
    "files",
    "workbench",
    "window",
    "terminal",
    "search",
    "explorer",
];

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// テーマの名前（例: "Nucleus One Dark" / "Nucleus One Light"）。
    pub theme: String,
    /// 表示言語（例: "en" / "ja"）。null なら OS の設定に従う。
    pub language: Option<String>,
    /// 起動時に前回のフォルダと開いていたファイルを復元する。
    pub restore_session: bool,
    pub editor: EditorSettings,
    pub files: FilesSettings,
    pub workbench: WorkbenchSettings,
    pub window: WindowSettings,
    pub terminal: TerminalSettings,
    pub search: SearchSettings,
    pub explorer: ExplorerSettings,
    /// 追加のキーバインド（変更は再起動後に反映）。
    pub keymap: Vec<KeymapEntry>,
    /// 無効にする Extension の ID。
    pub disabled_extensions: Vec<String>,
    /// Extension の一覧（index.json）の場所。null なら公式（sofia-gros/nucleus-extensions）。
    pub extensions_registry: Option<String>,
    /// 起動時に Extension の更新を確かめる。
    pub check_extension_updates: bool,
    /// Extension ごとの設定（拡張機能の画面で変える）。`{ "<id>": { "<key>": 値 } }`
    pub extension_settings: BTreeMap<String, serde_json::Map<String, serde_json::Value>>,
    /// 言語ごとの上書き（VSCode と同じく `"[rust]": { "editor.tab_size": 2 }` と書く）。
    /// 言語名 → `{ "editor": {...}, "files": {...} }`。読み込むときに lib.rs が作る。
    #[serde(skip)]
    pub language_overrides: BTreeMap<String, serde_json::Value>,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            theme: "Nucleus One Dark".into(),
            language: None,
            restore_session: true,
            editor: EditorSettings::default(),
            files: FilesSettings::default(),
            workbench: WorkbenchSettings::default(),
            window: WindowSettings::default(),
            terminal: TerminalSettings::default(),
            search: SearchSettings::default(),
            explorer: ExplorerSettings::default(),
            keymap: Vec::new(),
            disabled_extensions: Vec::new(),
            extensions_registry: None,
            check_extension_updates: true,
            extension_settings: Default::default(),
            language_overrides: Default::default(),
        }
    }
}

/// 言語ごとの上書きで変えられるまとまり。
pub const OVERRIDABLE: &[&str] = &["editor", "files"];

impl Settings {
    /// その言語で使うエディタの設定（`"[言語]"` の上書きを重ねたもの）。
    pub fn editor_for(&self, language: &str) -> EditorSettings {
        self.overridden("editor", language, &self.editor)
    }

    /// その言語で使うファイルの設定（`"[言語]"` の上書きを重ねたもの）。
    pub fn files_for(&self, language: &str) -> FilesSettings {
        self.overridden("files", language, &self.files)
    }

    fn overridden<T: Serialize + serde::de::DeserializeOwned + Clone>(
        &self,
        group: &str,
        language: &str,
        base: &T,
    ) -> T {
        let Some(over) = self.language_overrides.get(language).and_then(|o| o.get(group)) else {
            return base.clone();
        };
        let mut v = serde_json::to_value(base).expect("settings serialize");
        if let (Some(b), Some(o)) = (v.as_object_mut(), over.as_object()) {
            for (k, val) in o {
                b.insert(k.clone(), val.clone());
            }
        }
        serde_json::from_value(v).unwrap_or_else(|_| base.clone())
    }
}

// ---- テキストエディタ ----

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct EditorSettings {
    pub font_family: String,
    pub font_size: f32,
    /// 行の高さ（文字の大きさの何倍か）。
    pub line_height: f32,
    pub tab_size: usize,
    /// インデントにタブ文字を使う（VSCode の `insertSpaces` の逆）。
    pub hard_tabs: bool,
    /// ファイルを開いたとき、中身からタブ幅とタブ文字かどうかを決める。
    pub detect_indentation: bool,
    /// 長い行を折り返す。
    pub soft_wrap: bool,
    /// 折り返した行の頭をそろえる位置。
    pub wrapping_indent: WrappingIndent,
    pub line_numbers: bool,
    /// 行番号を、カーソルのある行からの距離で出す（カーソルの行は本来の番号）。
    pub relative_line_numbers: bool,
    /// 縦の目安線を引く列（例: `[80, 120]`）。
    pub rulers: Vec<usize>,
    pub indent_guides: bool,
    /// 空白とタブを記号で見せる。
    pub render_whitespace: RenderWhitespace,
    /// コードの折りたたみ。
    pub folding: bool,
    /// 括弧や引用符を入れたら、閉じる方も入れる。
    pub auto_closing_brackets: bool,
    /// 改行したとき、言語に合わせてインデントする。
    pub auto_indent: bool,
    /// 最後の行より下までスクロールできる。
    pub scroll_beyond_last_line: bool,
    /// カーソルの上下に、最低限見せておく行数。
    pub cursor_surrounding_lines: usize,
    pub cursor_blinking: CursorBlinking,
    /// カーソルのある行の強調を、エディタを使っているときだけにする。
    pub highlight_line_only_when_focused: bool,
    /// 保存するとき、言語サーバーで整形する。
    pub format_on_save: bool,
    /// マウスを乗せたとき、言語サーバーの説明を出す。
    pub hover: bool,
    /// クリックでカーソルを足すときに押すキー（もう一方は定義へ移動）。
    pub multi_cursor_modifier: MultiCursorModifier,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MultiCursorModifier {
    Alt,
    /// Windows / Linux は Ctrl、macOS は Cmd。
    CtrlCmd,
}

impl Default for EditorSettings {
    fn default() -> Self {
        EditorSettings {
            font_family: "JetBrains Mono".into(),
            font_size: 13.0,
            line_height: 1.5,
            tab_size: 4,
            hard_tabs: false,
            detect_indentation: true,
            soft_wrap: false,
            wrapping_indent: WrappingIndent::Same,
            line_numbers: true,
            relative_line_numbers: false,
            rulers: Vec::new(),
            indent_guides: true,
            render_whitespace: RenderWhitespace::None,
            folding: true,
            auto_closing_brackets: true,
            auto_indent: true,
            scroll_beyond_last_line: true,
            cursor_surrounding_lines: 0,
            cursor_blinking: CursorBlinking::Blink,
            highlight_line_only_when_focused: false,
            format_on_save: false,
            hover: true,
            multi_cursor_modifier: MultiCursorModifier::Alt,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WrappingIndent {
    /// 左端から。
    None,
    /// 元の行と同じインデント。
    Same,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RenderWhitespace {
    None,
    All,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CursorBlinking {
    Blink,
    Solid,
}

// ---- ファイル ----

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct FilesSettings {
    pub auto_save: AutoSave,
    /// `after_delay` のときの待ち時間（ミリ秒）。
    pub auto_save_delay: u64,
    /// 保存するとき、行末の空白を消す。
    pub trim_trailing_whitespace: bool,
    /// 保存するとき、最後に改行がなければ足す。
    pub insert_final_newline: bool,
    /// 保存するとき、最後の余分な改行を 1 つにする。
    pub trim_final_newlines: bool,
    /// 新しいファイル（改行のないファイル）の改行コード。
    pub eol: Eol,
    /// ファイルツリーと Ctrl+P に出さないもの（glob。例: `**/node_modules`）。
    pub exclude: Vec<String>,
}

impl Default for FilesSettings {
    fn default() -> Self {
        FilesSettings {
            auto_save: AutoSave::Off,
            auto_save_delay: 1000,
            trim_trailing_whitespace: false,
            insert_final_newline: false,
            trim_final_newlines: false,
            eol: Eol::Auto,
            exclude: ["**/.git", "**/.svn", "**/.hg", "**/CVS", "**/.DS_Store", "**/Thumbs.db"]
                .map(String::from)
                .to_vec(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AutoSave {
    Off,
    /// 変更してから `auto_save_delay` ミリ秒たったら。
    AfterDelay,
    /// 別のタブやウィンドウに移ったら。
    OnFocusChange,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Eol {
    /// OS に合わせる（Windows は CRLF、ほかは LF）。
    Auto,
    Lf,
    Crlf,
}

// ---- ワークベンチ（画面全体） ----

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct WorkbenchSettings {
    pub status_bar: bool,
    /// 主サイドバー（ファイルツリーと検索）の場所。副サイドバー（アウトラインや Git）は反対側。
    pub side_bar_location: SideBarLocation,
    /// ファイルツリーの 1 段の字下げ（px）。
    pub tree_indent: f32,
    /// ファイルツリーの字下げの線。
    pub tree_indent_guides: bool,
    /// 起動したとき、フォルダを開いていなければようこそ画面を出す。
    pub startup_welcome: bool,
}

impl Default for WorkbenchSettings {
    fn default() -> Self {
        WorkbenchSettings {
            status_bar: true,
            side_bar_location: SideBarLocation::Left,
            tree_indent: 16.0,
            tree_indent_guides: true,
            startup_welcome: true,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SideBarLocation {
    Left,
    Right,
}

// ---- ウィンドウ ----

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct WindowSettings {
    /// 画面全体の拡大率。0 が標準で、1 増えるごとに 20% 大きくなる（VSCode と同じ）。
    pub zoom_level: f32,
}

impl WindowSettings {
    /// 拡大率（1.0 が標準）。
    pub fn zoom(&self) -> f32 {
        1.2f32.powf(self.zoom_level.clamp(-5.0, 9.0))
    }
}

// ---- ターミナル ----

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct TerminalSettings {
    /// 使うシェル。null なら OS の既定（Windows は PowerShell、ほかは $SHELL）。
    pub shell: Option<String>,
    pub shell_args: Vec<String>,
    /// null ならエディタと同じ。
    pub font_family: Option<String>,
    /// null ならエディタと同じ。
    pub font_size: Option<f32>,
    /// 行の高さ（文字の大きさの何倍か）。
    pub line_height: f32,
    /// さかのぼって見られる行数。
    pub scrollback: usize,
}

impl Default for TerminalSettings {
    fn default() -> Self {
        TerminalSettings {
            shell: None,
            shell_args: Vec::new(),
            font_family: None,
            font_size: None,
            line_height: 1.5,
            scrollback: 10_000,
        }
    }
}

// ---- 検索 ----

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct SearchSettings {
    /// プロジェクト全体の検索で探さないもの（glob）。`files.exclude` も探さない。
    pub exclude: Vec<String>,
    /// `.gitignore` などに書かれたものを探さない。
    pub use_ignore_files: bool,
    /// 大文字を含むときだけ、大文字小文字を区別する。
    pub smart_case: bool,
    /// 一致の数の上限。
    pub max_results: usize,
}

impl Default for SearchSettings {
    fn default() -> Self {
        SearchSettings {
            exclude: ["**/node_modules", "**/bower_components", "**/*.code-search"]
                .map(String::from)
                .to_vec(),
            use_ignore_files: true,
            smart_case: false,
            max_results: 20_000,
        }
    }
}

// ---- エクスプローラー（ファイルツリー） ----

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ExplorerSettings {
    /// ゴミ箱へ移す前に確かめる。
    pub confirm_delete: bool,
    pub sort_order: SortOrder,
    /// エディタで選んだファイルを、ツリーでも選ぶ（フォルダを開いて見せる）。
    pub auto_reveal: bool,
}

impl Default for ExplorerSettings {
    fn default() -> Self {
        ExplorerSettings {
            confirm_delete: true,
            sort_order: SortOrder::Default,
            auto_reveal: true,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SortOrder {
    /// フォルダが先、名前順。
    Default,
    /// フォルダとファイルを混ぜて名前順。
    Mixed,
    /// ファイルが先、名前順。
    FilesFirst,
    /// 拡張子の順（同じなら名前順）。フォルダが先。
    Type,
}
