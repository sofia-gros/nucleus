# Nucleus Plugin SDK ガイド & 仕様書

Nucleus は **「Core層はシンプル・爆速、機能は WASM プラグインで拡張可能」** を基本思想とした次世代 IDE です。
すべてのプラグインは軽量・セキュアな WebAssembly (Wasmtime) サンドボックス内で実行され、ホストの UI スレッドをブロックすることなく非同期に動作します。

---

## 1. アーキテクチャと基本思想

```
┌─────────────────────────────────────────────────────────────┐
│                      Nucleus Host IDE                       │
│  (Rust + GPUI: 爆速レンダリング & Zero UI Thread Blocking)   │
└──────────────┬───────────────────────────────▲──────────────┘
               │ (Non-blocking JSON-RPC)       │ (UI AST / Events)
┌──────────────▼───────────────────────────────┴──────────────┐
│                  WASM Plugin Sandbox Runtime                │
│    ┌──────────────────┐               ┌──────────────────┐  │
│    │  Rust Plugin     │               │   Go Plugin      │  │
│    │  (Official SDK)  │               │   (TinyGo WASM)  │  │
│    └──────────────────┘               └──────────────────┘  │
│    ┌──────────────────┐               ┌──────────────────┐  │
│    │   TS / JS        │               │   C / C++        │  │
│    │   (Javy WASM)    │               │   (WASI / Clang) │  │
│    └──────────────────┘               └──────────────────┘  │
└─────────────────────────────────────────────────────────────┘
```

### 3大コア原則
1. **Host/Plugin Boundary Strictness**: ホスト内部の Rust/GPUI 型は一切プラグイン ABI に露出せず、標準化された JSON-RPC メッセージでのみ通信します。
2. **Zero UI Thread Blocking**: 重いファイル I/O や外部プロセス実行はすべてバックグラウンドで非同期処理されます。
3. **Conflict-Free Merging**: アイコンテーマやステータスバッジ、多言語化辞書などは、ホスト側で合成（Merge）されて競合なく描画されます。

---

## 2. プラグインマニフェスト (`plugin.toml`)

プラグインのルートディレクトリに必ず `plugin.toml` を配置します。

```toml
[plugin]
id = "my-awesome-plugin"
name = "My Awesome Plugin"
version = "0.1.0"
description = "Nucleus を拡張するプラグイン"
author = "Your Name <you@example.com>"
license = "MIT"

[runtime]
# 生成された WASM ファイル名
wasm = "my_plugin.wasm"

[permissions]
# 必要な権限を宣言（サンドボックス制御）
fs = ["read", "write"]
process = ["exec", "spawn"]
ui = [
    "register_activity_bar",
    "register_status_bar",
    "register_sidebar",
    "register_panel",
    "register_icon_rules",
    "register_translations"
]
```

### パーミッション一覧
| 権限名 | 説明 |
|---|---|
| `fs` | ワークスペース内外のファイル読み書き（`read`, `write`） |
| `process` | 外部プロセスの同期/非同期実行（`exec`, `spawn`） |
| `ui` | アクティビティバー、ステータスバー、サイドバー、アイコン、翻訳辞書の登録 |

---

## 3. Rust 公式 SDK (`nucleus-plugin-sdk`)

Rust でプラグインを開発する場合、公式 SDK クレートを使用することで安全かつ簡潔に API を呼び出せます。

### 3.1 `Cargo.toml` の設定
```toml
[package]
name = "my_plugin"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib"]

[dependencies]
nucleus_plugin_sdk = { path = "../../crates/nucleus-plugin-sdk" }
serde_json = "1.0"
```

### 3.2 ライフサイクルマクロ (`export_plugin!`)
```rust
use nucleus_plugin_sdk::{export_plugin, log, invoke};

fn init() {
    log("プラグインがロードされました！");
}

fn on_event(event: serde_json::Value) {
    if let Some(event_type) = event["event"].as_str() {
        match event_type {
            "file_opened" => {
                let path = event["path"].as_str().unwrap_or("");
                log(&format!("ファイルが開かれました: {}", path));
            }
            "command_executed" => {
                let cmd = event["command"].as_str().unwrap_or("");
                log(&format!("コマンドが実行されました: {}", cmd));
            }
            _ => {}
        }
    }
}

export_plugin!(init, on_event);
```

---

## 4. ホスト API リファレンス

### 4.1 UI 拡張 (`ui.*`)

#### `ui.register_status_bar_item`
ステータスバーに項目を登録・更新します。
```json
{
  "api": "ui.register_status_bar_item",
  "args": {
    "id": "my_status",
    "text": "⚡ Ready",
    "align": "left",
    "command": "my_plugin.action"
  }
}
```

#### `ui.register_activity_bar_item`
左端アクティビティバーにアイコンボタンを追加します。
```json
{
  "api": "ui.register_activity_bar_item",
  "args": {
    "id": "my_sidebar_btn",
    "icon": "box",
    "tooltip": "My Custom Sidebar",
    "command": "my_plugin.open_sidebar"
  }
}
```

#### `ui.register_icon_rules`
ファイル拡張子・ファイル名ごとのアイコンとカラーを登録します（Material Icons 連携）。
```json
{
  "api": "ui.register_icon_rules",
  "args": {
    "rules": {
      "rs": { "icon": "🦀", "color": "#f97316" },
      "ts": { "icon": "🔷", "color": "#3b82f6" },
      "Cargo.toml": { "icon": "📦", "color": "#ea580c" }
    }
  }
}
```

#### `ui.register_translations`
UI の文言を多言語化するための辞書を登録します（日本語パック連携）。
```json
{
  "api": "ui.register_translations",
  "args": {
    "dict": {
      "File": "ファイル",
      "Edit": "編集",
      "Settings": "設定"
    }
  }
}
```

---

### 4.2 ワークスペース & エディタ (`workspace.*`, `editor.*`)

#### `editor.open_tab`
新しいエディタタブを開きます。
```json
{
  "api": "editor.open_tab",
  "args": {
    "title": "Preview.md",
    "path": "/path/to/Preview.md",
    "content": "# Hello from Plugin"
  }
}
```

#### `workspace.show_notification`
画面右下に通知バナーを表示します。
```json
{
  "api": "workspace.show_notification",
  "args": {
    "message": "ビルドが正常に完了しました。"
  }
}
```

---

### 4.3 プロセス実行 (`process.*`)

#### `process.spawn` (非同期実行・推奨)
バックグラウンドで外部プロセスを起動し、完了時にイベントを受け取ります。
```json
{
  "api": "process.spawn",
  "args": {
    "id": "git_fetch",
    "command": "git",
    "args": ["fetch", "--all"],
    "cwd": "A:/Project/nucleus"
  }
}
```

---

### 4.4 設定アクセス (`settings.*`)

#### `settings.get`
ユーザーまたはワークスペースの設定値を取得します。
```json
{
  "api": "settings.get",
  "args": {
    "key": "editor.fontSize"
  }
}
```
