# Nucleus 多言語プラグイン開発実践ガイド

Nucleus プラグインは WebAssembly (Wasmtime) をターゲットとするため、**Rust**, **Go (TinyGo)**, **TypeScript / JavaScript**, **C / C++** など、WASM にコンパイルできるあらゆる言語で開発可能です。

---

## 1. Rust によるプラグイン開発

Rust は公式 SDK (`nucleus-plugin-sdk`) が提供されているため、最も簡単かつ型安全に開発できます。

### 1.1 プロジェクト作成
```powershell
cargo new --lib plugins/my_rust_plugin
```

### 1.2 `plugins/my_rust_plugin/Cargo.toml`
```toml
[package]
name = "my_rust_plugin"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib"]

[dependencies]
nucleus_plugin_sdk = { path = "../../crates/nucleus-plugin-sdk" }
serde_json = "1.0"
```

### 1.3 `plugins/my_rust_plugin/plugin.toml`
```toml
[plugin]
id = "my_rust_plugin"
name = "Rust Sample Plugin"
version = "0.1.0"
description = "Rust で作成されたサンプルプラグイン"
author = "Nucleus Developer"

[runtime]
wasm = "my_rust_plugin.wasm"

[permissions]
ui = ["register_status_bar", "register_activity_bar"]
process = ["exec"]
```

### 1.4 `plugins/my_rust_plugin/src/lib.rs`
```rust
use nucleus_plugin_sdk::{export_plugin, log, invoke};

fn init() {
    log("Rust プラグインを初期化中...");

    // ステータスバー項目の登録
    let status_args = serde_json::json!({
        "id": "rust_status_item",
        "text": "🦀 Rust Plugin Active",
        "align": "right",
        "command": "rust_plugin.say_hello"
    });
    invoke("ui.register_status_bar_item", &serde_json::to_string(&status_args).unwrap());
}

fn on_event(event: serde_json::Value) {
    if event["event"] == "command_executed" && event["command"] == "rust_plugin.say_hello" {
        let notif = serde_json::json!({
            "message": "Hello from Rust WASM Plugin!"
        });
        invoke("workspace.show_notification", &serde_json::to_string(&notif).unwrap());
    }
}

export_plugin!(init, on_event);
```

### 1.5 ビルドコマンド
```powershell
cargo build --target wasm32-wasip1 --release
# または wasm32-unknown-unknown
cp target/wasm32-wasip1/release/my_rust_plugin.wasm plugins/my_rust_plugin/
```

---

## 2. Go (TinyGo) によるプラグイン開発

Go 言語では、軽量 WASM 出力に最適化された **TinyGo** を使用してプラグインを開発します。

### 2.1 プロジェクト構成
```
plugins/my_go_plugin/
├── plugin.toml
├── main.go
└── go.mod
```

### 2.2 `plugins/my_go_plugin/plugin.toml`
```toml
[plugin]
id = "my_go_plugin"
name = "Go Sample Plugin"
version = "0.1.0"
description = "TinyGo で作成されたサンプルプラグイン"

[runtime]
wasm = "my_go_plugin.wasm"

[permissions]
ui = ["register_status_bar"]
```

### 2.3 `plugins/my_go_plugin/main.go`
ホスト側の `host_invoke` 関数を `//go:wasmimport` で直接インポートして通信します。

```go
package main

import (
	"encoding/json"
	"unsafe"
)

// ホスト ABI のインポート定義
//go:wasmimport nucleus host_invoke
func host_invoke(ptr uintptr, length uint32) uint64

// ホスト API 呼び出しのヘルパー関数
func invoke(api string, args map[string]interface{}) (string, error) {
	req := map[string]interface{}{
		"api":  api,
		"args": args,
	}
	reqBytes, err := json.Marshal(req)
	if err != nil {
		return "", err
	}

	ptr := uintptr(unsafe.Pointer(&reqBytes[0]))
	length := uint32(len(reqBytes))

	packed := host_invoke(ptr, length)
	resPtr := uintptr(packed >> 32)
	resLen := uint32(packed & 0xFFFFFFFF)

	resBytes := unsafe.Slice((*byte)(unsafe.Pointer(resPtr)), resLen)
	return string(resBytes), nil
}

//export init
func initPlugin() {
	// ステータスバーの登録
	invoke("ui.register_status_bar_item", map[string]interface{}{
		"id":        "go_status_item",
		"text":      "🐹 Go Plugin Active",
		"alignment": "right",
	})
}

//export on_event
func onEvent(ptr uintptr, length uint32) {
	eventBytes := unsafe.Slice((*byte)(unsafe.Pointer(ptr)), length)
	var event map[string]interface{}
	if err := json.Unmarshal(eventBytes, &event); err == nil {
		if event["event"] == "file_opened" {
			invoke("workspace.show_notification", map[string]interface{}{
				"message": "Go プラグイン: ファイルが開かれました",
			})
		}
	}
}

func main() {}
```

### 2.4 ビルドコマンド
```powershell
tinygo build -o plugins/my_go_plugin/my_go_plugin.wasm -target=wasi plugins/my_go_plugin/main.go
```

---

## 3. TypeScript / JavaScript (Javy) によるプラグイン開発

Shopify 製の **Javy** (QuickJS ベースの WASM コンパイラ) を使用することで、モダンな TypeScript / JavaScript コードをそのまま Nucleus プラグインとして実行できます。

### 3.1 `plugins/my_js_plugin/index.js`
```javascript
// ホスト ABI バインディング
function invoke(api, args) {
  const req = JSON.stringify({ api, args });
  const res = NucleusHost.invoke(req);
  return JSON.parse(res);
}

// 初期化関数
function init() {
  invoke("ui.register_status_bar_item", {
    id: "js_status",
    text: "🟨 JS Plugin Active",
    align: "right"
  });
}

// イベントリスナー
function on_event(eventJson) {
  const event = JSON.parse(eventJson);
  if (event.event === "file_opened") {
    invoke("workspace.show_notification", {
      message: `JS: 開かれたファイル ${event.path}`
    });
  }
}
```

### 3.2 ビルドコマンド
```powershell
javy compile plugins/my_js_plugin/index.js -o plugins/my_js_plugin/my_js_plugin.wasm
```

---

## 4. C / C++ (Clang / WASI-SDK) によるプラグイン開発

### 4.1 `plugins/my_c_plugin/main.c`
```c
#include <stdint.h>
#include <stdlib.h>
#include <string.h>

// ホスト ABI のインポート
__attribute__((import_module("nucleus"), import_name("host_invoke")))
extern uint64_t host_invoke(const uint8_t* ptr, uint32_t len);

void invoke(const char* json_str) {
    uint32_t len = strlen(json_str);
    host_invoke((const uint8_t*)json_str, len);
}

__attribute__((export_name("init")))
void init() {
    const char* req = "{\"api\": \"ui.register_status_bar_item\", \"args\": {\"id\": \"c_status\", \"text\": \"⚡ C Plugin Active\", \"align\": \"right\"}}";
    invoke(req);
}

__attribute__((export_name("on_event")))
void on_event(const uint8_t* ptr, uint32_t len) {
    // イベント処理
}
```

### 4.2 ビルドコマンド
```powershell
clang --target=wasm32-wasi -O3 -nostdlib -Wl,--no-entry -Wl,--export=init -Wl,--export=on_event -o plugins/my_c_plugin/my_c_plugin.wasm plugins/my_c_plugin/main.c
```

---

## 5. プラグインのテスト & デバッグ

### 5.1 ホスト IDE での即時動作確認
Nucleus は起動時に `plugins/` ディレクトリを自動探索します。
作成したプラグインフォルダを `plugins/` 直下に配置し、エディタを起動するだけで自動ロードされます。

```powershell
# Nucleus の起動
cargo run --release
```

### 5.2 ログの確認
プラグイン内の `log("...")` メッセージは、Nucleus のデバッグコンソールまたは標準出力にリアルタイムでストリーミング表示されます。
