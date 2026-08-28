<div align="center">

# ⚛️ Nucleus

**Next-Generation, Blazing Fast Native Code Editor built with Rust, GPUI & WebAssembly**

[![Rust Edition 2024](https://img.shields.io/badge/Rust-2024_Edition-orange.svg?style=flat-square&logo=rust)](https://www.rust-lang.org/)
[![UI GPUI](https://img.shields.io/badge/UI-GPUI_%2B_gpui--component-blue.svg?style=flat-square)](https://github.com/zed-industries/zed)
[![Plugin Wasmtime](https://img.shields.io/badge/Plugin-Wasmtime_WASM-purple.svg?style=flat-square)](https://wasmtime.dev/)
[![Platform](https://img.shields.io/badge/Platform-macOS%20%7C%20Linux%20%7C%20Windows-lightgrey.svg?style=flat-square)](#)
[![License](https://img.shields.io/badge/License-MIT%20%2F%20Apache--2.0-green.svg?style=flat-square)](#)

<br/>

<p align="center">
  <img src="image1.png" alt="Nucleus Editor UI" width="48%" style="border-radius: 8px; box-shadow: 0 4px 12px rgba(0,0,0,0.15);" />
  <img src="image2.png" alt="Nucleus Workspace & Plugins" width="48%" style="border-radius: 8px; box-shadow: 0 4px 12px rgba(0,0,0,0.15);" />
</p>

</div>

---

## 🌟 主な特徴 (Features)

- ⚡ **Zed 級の圧倒的パフォーマンス**:
  - GPU アクセラレーションによる 60fps 描画（GPUI & `gpui-component`）。
  - 大規模ファイルでも快適に動作する $O(\log n)$ の `ropey` テキストデータ構造。
  - Zero UI Thread Blocking: 重いファイル I/O、プロセス実行、WASM 処理はすべて非同期バックグラウンド実行。
- 🧩 **WASM プラグインサンドボックス**:
  - Wasmtime を採用した安全・堅牢なプラグイン実行環境。
  - プラグインの trap / パニックがエディタ本体に影響を与えないプロセス隔離。
  - 宣言的 UI AST により、Activity Bar、Sidebar、Status Bar、Panel を安全に拡張可能。
- 🎨 **モダンで美しい UI/UX**:
  - `gpui-component` の Shadcn 風デザインシステム。
  - OS の外観（Dark / Light モード）への自動追従とスムーズなアニメーション。
  - Git ステータスバッジ（Modified, Untracked, Deleted）付きファイルツリー。
- 📂 **堅牢なワークスペース管理**:
  - 複数タブ管理と未保存変更（ダーティインジケータ `●`）のリアルタイム追跡。
  - パネルのリサイズ・開閉状態の自動保存（`.nucleus/state.json`）。

---

## 🏗️ アーキテクチャ概要

Nucleus はドメイン駆動の垂直スライス（Domain-Driven Vertical Slice）アーキテクチャを採用しています。

```
Nucleus (Host)
├── GPUI + gpui-component (GPU レンダリング & UI)
├── Workspace
│   ├── ActivityBar / LeftSidebar (Explorer, Plugin Views)
│   ├── EditorArea (Tabs, GpuiEditor + Syntect Syntax Highlighter)
│   ├── RightSidebar / BottomPanel (Terminal Logs, Problems)
│   └── StatusBar / TitleBar
├── Core Editor Engine
│   ├── TextBuffer (ropey::Rope, O(log n) 編集, LineEnding)
│   ├── History (Undo / Redo トランザクション)
│   ├── Point & Selection (マルチカーソル座標系)
│   └── DisplayMap (タブ展開・座標変換)
├── Project & FileSystem (Worktree, BufferStore)
└── PluginManager
    └── Wasmtime Runtime (WASM Sandbox)
        ├── FileSystem API / Process API / Settings API
        └── UI AST Registry (Sidebar, ActivityBar, StatusBar)
```

---

## 🚀 クイックスタート (Getting Started)

### 前提条件 (Prerequisites)

- [Rust](https://www.rust-lang.org/tools/install) (最新の stable または nightly)
- C++ ビルドツール（Windows の場合は Visual Studio C++ Build Tools、Linux は `build-essential`, `libx11-dev` 等）

### 1. リポジトリのクローン

```bash
git clone https://github.com/sofia-gros/nucleus.git
cd nucleus
```

### 2. ローカルで起動 (Run)

```bash
# 通常の起動
cargo run

# 特定のフォルダをルートとして開いて起動
cargo run -- --root=.

# リリースビルドでの高速起動
cargo run --release
```

### 3. テストの実行

```bash
cargo test
```

---

## 🔌 プラグインの開発・ビルド

付属のサンプルプラグイン (`plugins/dummy`) をビルドして動作確認ができます。

```bash
# wasm32 ターゲットの追加
rustup target add wasm32-unknown-unknown

# ダミープラグインのビルド
cd plugins/dummy
cargo build --target wasm32-unknown-unknown
cd ../..

# プラグインを含めてエディタを起動
cargo run
```

---

## ⌨️ 標準ショートカットキー

| ショートカット | 機能 |
| :--- | :--- |
| <kbd>Ctrl</kbd> + <kbd>B</kbd> | 左サイドバー（エクスプローラー）の開閉 |
| <kbd>Ctrl</kbd> + <kbd>J</kbd> | ボトムパネル（ログ・ターミナル）の開閉 |
| <kbd>Ctrl</kbd> + <kbd>R</kbd> | 右サイドバー（AI / プラグイン）の開閉 |
| <kbd>Ctrl</kbd> + <kbd>S</kbd> | アクティブバッファのファイル保存 |
| <kbd>Ctrl</kbd> + <kbd>Z</kbd> | 元に戻す (Undo) |
| <kbd>Ctrl</kbd> + <kbd>Y</kbd> / <kbd>Ctrl</kbd>+<kbd>Shift</kbd>+<kbd>Z</kbd> | やり直す (Redo) |
| <kbd>Ctrl</kbd> + <kbd>A</kbd> | 全選択 |

---

## 🗺️ 実装進捗とロードマップ

完成品までの詳細な実装計画とタスク進捗は [docs/ROADMAP.md](docs/ROADMAP.md) をご覧ください。

---

## 📄 ライセンス

本プロジェクトは [MIT License](LICENSE) の下で公開されています。
