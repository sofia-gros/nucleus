# Nucleus 実装進捗 & 完成ロードマップ

Nucleus は、Rust (edition 2024)、GPUI、Wasmtime をベースにした、超高速・軽量な次世代デスクトップコードエディタです。
本ドキュメントは、現在の実装状況と完成目標までの道のりをタスクリスト形式で管理・追跡するための進捗表です。

---

## 📊 実装進捗サマリー

- **Phase 1: Core Editor Foundation**: 🟢 **90% 完了**
- **Phase 2: Project & File System**: 🟡 **40% 完了**
- **Phase 3: LSP & Language Intelligence**: ⚪ **0% 未着手**
- **Phase 4: Terminal & Process**: 🟢 **100% 完了**
- **Phase 5: Plugin System (WASM)**: 🟢 **100% 完了**
- **Phase 6: Polish & Performance Tuning**: 🟡 **20% 完了**

---

## 🗺️ 詳細フェーズ別ロードマップ

### Phase 1: Core Editor Foundation (エディタ基盤) 【最重要】

- [x] **Rope データ構造の統合 (`ropey = "1.6"`)**
  - [x] $O(\log n)$ によるテキスト挿入・削除・スライス
  - [x] `TextBuffer` による行・文字オフセット相互変換
  - [x] 改行コード判別 (LF / CRLF)
- [x] **Undo / Redo 履歴スタック (`src/editor/history.rs`)**
  - [x] トランザクション対応 (複数操作の単一Undo集約)
  - [x] 最大履歴数の制限とメモリ効率化
- [x] **座標系 & 選択範囲 (`src/editor/buffer/point.rs`, `src/editor/selection.rs`)**
  - [x] 2次元論理座標 `Point(row, column)`
  - [x] 変更追従用アンカー `Anchor`
  - [x] マルチカーソル/選択範囲の統合・正規化 (`normalize_selections`)
- [x] **DisplayMap 変換パイプライン (`src/editor/display_map/`)**
  - [x] タブ幅展開 (Tab stop計算)
  - [x] Buffer 論理座標 ↔ Display 画面座標の双方向変換
  - [ ] ソフトラップ計算 (SoftWrapMap)
  - [ ] コード折りたたみ範囲マッピング (FoldMap)
- [x] **エディタコンポーネント統合 (`src/editor/mod.rs`)**
  - [x] `gpui-component` の `Editor` / `EditorState` UI との連携
  - [x] `SyntectHighlighter` による構文ハイライト
  - [x] コード折りたたみ (`folding(true)`)
  - [x] `TextBuffer` との双方向テキスト同期 & ファイル保存
- [x] **タブ管理 & カスタムタブバー (`src/workspace/editor_area/`)**
  - [x] カスタムファイルタブUI (ファイルアイコン、タイトル、ダーティインジケータ `●`、個別閉じるボタン `✕`)
  - [x] 複数タブの開閉・切り替え
  - [x] 右クリックコンテキストメニュー (VSCode 互換: Close, Close Others, Close to the Right, Close Saved, Close All)
  - [x] `Ctrl+S` 等によるバッファ保存

---

### Phase 2: Project & File System (プロジェクト・ファイル管理)

- [x] **バッファストア (`src/project/buffer_store.rs`)**
  - [x] 開いているバッファの一元管理・重複オープン防止
  - [x] ファイルパスとのマッピング
- [x] **ファイルツリー探索 (`src/file_system/mod.rs`, `src/workspace/left_sidebar/`)**
  - [x] ディレクトリの再帰走査とソート（フォルダ優先）
  - [x] `gpui-component` の `Tree` を用いたツリーUI表示
  - [x] VSCode 準拠 Git ステータス表示（フォルダ右端の丸バッジ `●`、ファイル右端のステータス文字 `M`, `U`, `D`）
  - [x] Source Control UI (Staged / Changes 分離、ホバー時の `+` ステージング / `↺` 破棄アクション)
- [ ] **Worktree & ファイル監視 (`notify` crate 連携)**
  - [ ] 外部変更のリアルタイム検知
  - [ ] エディタ外で変更されたファイルの自動再読み込み / 警告ダイアログ
  - [ ] `.gitignore` の自動認識とファイルツリーからの除外

---

### Phase 3: LSP & Language Intelligence (言語サーバー統合)

- [ ] **LSP クライアント基盤 (`src/lsp/`)**
  - [ ] JSON-RPC over stdio による Language Server との非同期通信
  - [ ] `LspStore` による Language Server プロセスライフサイクル管理
- [ ] **コア LSP 機能**
  - [ ] コード補完 (Completion / Suggestions ポップアップ)
  - [ ] ホバー情報 (Hover tooltip)
  - [ ] 定義へジャンプ (Go to Definition)
  - [ ] 参照箇所の検索 (Find References)
  - [ ] シンボルのリネーム (Rename)
  - [ ] コードアクション (Quick Fix)
  - [ ] ドキュメントフォーマット (Formatting)
- [ ] **Diagnostics & Problems パネル**
  - [ ] エラー/警告の下線表示
  - [ ] `BottomPanel` の Problems タブへの一覧集約とクリックジャンプ

---

### Phase 4: Terminal & Process (端末・外部プロセス)

- [x] **外部プロセスの実行管理 (`src/process/`, `src/plugin_manager/`)**
  - [x] バックグラウンドでのコマンド実行
  - [x] stdout / stderr の非同期キャプチャ
- [x] **ログ出力パネル (`src/workspace/bottom_panel/`)**
  - [x] プラグイン・プロセスログのターミナル風表示
  - [x] ログクリア機能
- [x] **本物の PTY 端末統合 (`portable-pty` 採用)**
  - [x] ConPTY / PTY ネイティブセッション管理 (`src/terminal/mod.rs`)
  - [x] BottomPanel 内でのインタラクティブ PTY セッション & 双方向入出力
  - [x] 複数端末タブの追加・切り替え・終了

---

### Phase 5: Plugin System (WASM プラグイン)

- [x] **Wasmtime サンドボックス実行基盤 (`src/plugin_manager/runtime.rs`)**
  - [x] `plugin.toml` マニフェスト解析
  - [x] WASM モジュールの動的ロード・初期化 (`init`)・シャットダウン
  - [x] プラグインクラッシュ（trap）時のホスト隔離
- [x] **Plugin ABI & Host API ルーター (`src/plugin_manager/api_router.rs`)**
  - [x] `host_invoke` (ポインタ渡し + JSONシリアライズ)
  - [x] イベントディスパッチ (`on_event`)
  - [x] プロセス実行 API (`process.exec`, `process.spawn`)
  - [x] ワークスペース API (`workspace.get_root_path`, `workspace.open_tab`, `workspace.show_notification`)
  - [x] ファイル読み書き API (`fs.read_file`, `fs.write_file`)
  - [x] 設定取得・更新 API (`settings.get`, `settings.set`)
  - [x] コマンド登録 API (`command.register`)
- [x] **UI Extension Points (`src/plugin_manager/ui.rs`)**
  - [x] Activity Bar アイテム登録
  - [x] Status Bar アイテム登録 (ブランチ名表示等)
  - [x] Sidebar / Panel アイテム登録 & 動的更新 (`ui.update_sidebar`)
  - [x] 宣言的 UI AST レンダリング (`source_control`, `tree`, `text`)
- [x] **公式組み込み Git プラグイン (`plugins/git/`)**
  - [x] `git status` / `git branch` の同期・非同期取得
  - [x] Source Control サイドバー (変更ファイル一覧, コミット入力フォーム)
  - [x] ファイルツリーへの Git ステータスバッジ反映 (`M`, `??`, `A`, `D`)
  - [x] 変更ファイルクリック時のエディタタブ表示 & コミット実行
- [ ] **Plugin SDK & 権限システムの厳格化**
  - [ ] `nucleus-plugin-sdk` クレートの提供

---

### Phase 6: Polish & Performance Tuning (洗練と最適化)

- [x] **Workspace レイアウト & 状態永続化 (`src/workspace/`)**
  - [x] Activity Bar / Left Sidebar / Editor Area / Right Sidebar / Bottom Panel / Status Bar
  - [x] パネルのリサイズ・開閉トグル
  - [x] `.nucleus/state.json` へのウィンドウ・パネル状態の保存・復元
- [x] **キーバインディング (`src/keybindings/mod.rs`)**
  - [x] VSCode 互換の標準キーバインド
- [ ] **コマンドパレット (`src/workspace/command_palette/`)**
  - [ ] `Ctrl+Shift+P` によるコマンド検索 & 実行モーダル
  - [ ] ファイル検索 (`Ctrl+P`)
- [ ] **グローバル検索・置換 (`src/search/`)**
  - [ ] プロジェクト全体からの ripgrep 高速検索
- [ ] **パフォーマンスチューニング**
  - [ ] コールドスタート起動時間 < 200ms
  - [ ] 60fps スクロール & ゼロ UI スレッドブロッキングの検証
