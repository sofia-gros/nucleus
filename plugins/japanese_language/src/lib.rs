/// 日本語言語パック プラグイン実装

use nucleus_plugin_sdk::{export_plugin, log, invoke};

fn init() {
    log("Initializing Japanese Language Pack plugin...");

    let translations = serde_json::json!({
        "dict": {
            "File": "ファイル",
            "Edit": "編集",
            "Selection": "選択",
            "View": "表示",
            "Go": "移動",
            "Run": "実行",
            "Terminal": "ターミナル",
            "Help": "ヘルプ",
            "Settings": "設定",
            "Keybindings": "キーボード ショートカット",
            "All Settings": "すべての設定",
            "Appearance": "外観",
            "Editor": "エディタ",
            "Files": "ファイル",
            "Languages & LSP": "言語とLSP",
            "Debug": "デバッグ",
            "Git": "ソース管理 (Git)",
            "Plugins": "拡張機能 (プラグイン)",
            "User": "ユーザー",
            "Workspace": "ワークスペース",
            "Search settings...": "設定の検索...",
            "No folder opened": "フォルダーが開かれていません",
            "Loading...": "読み込み中...",
            "Enabled": "有効",
            "Disabled": "無効",
            "Save": "保存"
        }
    });

    let args = serde_json::to_string(&translations).unwrap_or_default();
    invoke("ui.register_translations", &args);

    log("Japanese Language Pack dictionary registered successfully.");
}

export_plugin!(init);
