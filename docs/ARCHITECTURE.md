# Nucleus アーキテクチャ

> Core は小さく速く。機能はすべて Extension で足す。色と文言は必ず差し替えられる。

デザインの正: [`docs/design/HANDOFF.md`](design/HANDOFF.md)（gpui-kit のトークンと寸法、Atom One の配色、画面一覧）

---

## 1. 原則

1. **UI スレッドを止めない。** UI スレッドでは描画と入力処理だけを行う。I/O、WASM、LSP、git などは必ずバックグラウンドで実行し、結果はメッセージとして受け取る。
2. **テキストの正本は1か所。** バッファの Rope は1つだけ持ち、コピーで同期しない。保存などにはスナップショット（O(1)）を使う。
3. **色は Token からしか取らない。** Core も Extension も、色はテーマの Token 経由でだけ指定する。
4. **文言は i18n キーからしか取らない。** UI に表示する文字列は、Core も Extension もすべて翻訳できる。
5. **公式機能も Extension の API で作る。** 速さが必要な部分（エディタ、ファイルツリー、ターミナル、検索）だけを Core に置く。
6. **型で守る。** Token、権限、API の版は型とスキーマで検証し、ずれはコンパイル時か読み込み時に検出する。

---

## 2. 全体構成

```
┌──────────────────────── UI スレッド（gpui-kit） ──────────────────────────┐
│ Workspace: TitleBar / DockArea(左・右・下・中央) / StatusBar / Overlay     │
│   ├─ Core のビュー : Editor, ProjectTree, Terminal, Search, Settings       │
│   └─ Extension のビュー : UI ツリー → ui_bridge が gpui-kit 部品で描画       │
│ Theme(解決済みの色の表)  ・  I18n(解決済みの文言の表)                        │
└────────────▲ パッチ / イベント（非同期メッセージだけ） ▼──────────────────────┘
┌──────────── Extension Host（スレッドプール。Extension ごとに Store を持つ） ──────────┐
│ wasmtime Component Model ・ WIT: nucleus:core@1 / nucleus:ui@N                │
│ 権限チェック ・ epoch 割り込み（時間切れ） ・ trap の隔離                           │
└────────────▲ 起動・監視 ▼───────────────────────────────────────────────────┘
       外部プロセス: 言語サーバー / git / シェル（Core が管理）
```

### Cargo workspace（予定）
| crate | 役割 |
|---|---|
| `nucleus` (bin) | 起動、ウィンドウ、アクションの登録 |
| `theme` | Core Token の定義（スキーマから生成）、拡張 Token、テーマの解決とキャッシュ |
| `i18n` | Fluent による文言の解決、言語の切り替え、フォールバック |
| `workspace` | レイアウト（Dock）、タブ、パネル、StatusBar、TitleBar、コマンドパレット、拡張用スロット |
| `editor` | gpui-kit の code editor を包むもの（Document、保存、検索バー、装飾） |
| `project` | Worktree（`ignore` によるバックグラウンド走査と `notify` による差分更新）、ファイル検索、全文検索 |
| `terminal` | `alacritty_terminal`（VT の解釈と PTY）、グリッドの描画 |
| `lsp` | LSP クライアント（言語ごとにサーバー1つ、非同期） |
| `settings` | 階層化した設定（既定 / ユーザー / ワークスペース）、ホットリロード |
| `extension_host` | マニフェスト、権限、起動の流れ、wasmtime |
| `extension_api` | WIT 定義 + Rust SDK |
| `ui_bridge` | Extension の UI ツリーを gpui-kit の部品に変換して描画（差分の適用） |

依存の向き: `nucleus → workspace → (editor, project, terminal, lsp, extension_host, ui_bridge) → (settings, theme, i18n)`。`theme` と `i18n` は他の crate に依存しない。

---

## 3. Theme と Token

### 3.1 2層の Token
| 層 | 定義する場所 | 名前 | 型 |
|---|---|---|---|
| Core Token | Nucleus 本体の `theme/tokens.yaml`（すべてのテーマが必須で持つ） | 接頭辞なし（例: `tab.active.background`） | Rust の `enum Token`（生成）。存在しない名前はコンパイル時にエラー |
| 拡張 Token | Extension の `extension.yaml`（`theme.tokens` 権限） | `<publisher>.<ext>.<key>`（例: `anthropic.claude.brand`） | 読み込み時に整数 ID に変換（`ExtTokenId`） |

- Core Token のキー名は gpui-kit の `ThemeColor` / highlight のキーと同じにする（HANDOFF.md の表）。
- `tokens.yaml` から、次の3つを生成する。
  1. Rust の `enum Token`
  2. WIT の `enum core-token`
  3. テーマ JSON の検証用スキーマ

### 3.2 値として書けるもの
- `#rrggbb` / `#rrggbbaa`
- `$token`（参照）
- `$token/NN%`（参照に透明度を付けたもの）

拡張 Token は dark / light それぞれの既定値を持つ。参照が循環している場合や、存在しない Token を参照している場合は、読み込み時にエラーにする。

### 3.3 色の決まり方
1. 有効なテーマが上書きした値
2. 拡張 Token の既定値（テーマのモード dark / light で選ぶ）
3. 参照をまとめて解決し、`Vec<Hsla>` に詰める

色の取得は、配列を1回引くだけで済む。

### 3.4 強制
- **Core 側:** `theme` crate 以外での `rgb()` / `rgba()` / `hsla()` / `Hsla { .. }` の直書きを、clippy の `disallowed_methods` / `disallowed_types` で禁止する。
- **Extension 側:** UI ツリーの色の指定は `TokenRef = core(Token) | ext(id)` だけを受け付ける。使えるのは、自分が宣言した拡張 Token か、依存として宣言した Extension の Token だけ。それ以外は Host が拒否する。
- **コントラストの確認:** テーマの読み込み時に、前景と背景のコントラスト比を確認して警告を出す。

### 3.5 テーマファイル
```json
{ "name": "Nucleus One Dark", "mode": "dark",
  "fonts": { "ui": "Inter", "mono": "JetBrains Mono", "mono_size": 13 },
  "colors": { "background": "#282c34", "tab.active.background": "#282c34" },
  "tokens": { "anthropic.claude.brand": "#e08a6a" } }
```
テーマで変えられるのは色とフォントだけ。寸法は gpui-kit の値で固定する（HANDOFF.md）。

---

## 4. Extension モデル

配布の単位は **Extension の1種類だけ**。何ができるかは `extension.yaml` の `permissions` で宣言する。

### 4.1 マニフェスト
```yaml
id: anthropic.claude
name: "%name%"                 # i18n キー（後述）。直接文字列も可
version: 1.2.0
publisher: anthropic
api: { core: 1, ui: 3 }        # ui 権限を使う場合だけ ui の版が必要
depends: [nucleus.git]         # 他の Extension の Token やコマンドを使う場合

permissions:
  ui: [panel.right, tab, statusbar]
  editor: [read, edit]
  workspace: [fs.read]
  network: [api.anthropic.com]
  theme: [tokens]
  locale: [messages]

tokens:
  anthropic.claude.brand: { dark: "#d97757", light: "#c96442", kind: brand }
  anthropic.claude.message.background: { dark: "$muted.background", light: "$muted.background" }

contributes:
  commands:
    - { id: claude.ask, title: "%cmd.ask%", key: "ctrl+alt+c" }

l10n: l10n/                    # <lang>.ftl を置くディレクトリ
main: claude.wasm              # コードを持たない Extension（テーマ、文法、言語パックなど）では省略
```

### 4.2 権限
| 権限 | 内容 | 強い権限 |
|---|---|---|
| `theme.colors` / `theme.tokens` / `theme.icons` | 色のセット / 独自 Token / アイコンセット | |
| `locale.messages` / `locale.pack` | 自分の文言の翻訳 / 言語パック（他の Extension や Core の翻訳） | |
| `language.grammars` / `language.servers` / `language.formatters` | ファイルと言語の対応・文法 / 言語サーバーの起動 / フォーマッタ | `servers` |
| `editor.read` / `editor.edit` / `editor.decorate` | バッファの読み取り / 編集 / 診断・ヒント・ガター | `edit` |
| `workspace.fs.read` / `workspace.fs.write` | ワークスペース内のファイル | `write` |
| `process: [cmd…]` | 外部コマンドの実行（コマンド名を列挙） | ✓ |
| `network: [host…]` | 外部への通信（ホストを列挙） | ✓ |
| `ui.panel.{left,right,bottom}` / `ui.tab` / `ui.overlay` / `ui.statusbar` / `ui.titlebar` / `ui.menu` | UI の追加 | |
| `ui.replace.<view-id>` | 組み込みビューの置き換え | ✓ |

- マーケットとインストール画面では、権限を人が読める文で表示する。強い権限は目立たせる。
- 更新で権限が増えた場合は、再度ユーザーに確認する。
- すべての API 呼び出しで、Host が権限を確認する。

### 4.3 API の版（2系統）
WIT の world は次の2つ。
- `world extension { import nucleus:core/*@1; export lifecycle; }`
- `world extension-ui { include extension; import nucleus:ui/*@N; }`

| API | 方針 |
|---|---|
| `core@1` | 長く維持する。追加だけ行い、壊す変更はメジャー版でだけ |
| `ui@N` | Editor の UI 更新に合わせて上げる。Host は N と N-1 をサポートする |

- `ui` 権限を使わない Extension は、Editor が更新されても動き続ける。
- `ui` の版がサポート外になった場合は、`nucleus:ui` に「何もしない」代わりの実装をつないで読み込む。UI 部分だけが止まり、コマンドや言語機能はそのまま動く。ユーザーには通知する。
- `nucleus:ui` は gpui-kit の型を見せない。Nucleus 独自の部品の語彙だけで構成するので、gpui-kit の更新は `ui_bridge` が吸収する。

### 4.4 UI ツリー（`nucleus:ui`）
- Extension は `render(view-id, state) -> element` を実装する。
- 要素の語彙: `stack(h/v)`、`text`、`icon`、`button`、`input`、`textarea`、`list`、`tree`、`tabs`、`badge`、`kbd`、`markdown`、`code`、`skeleton`、`scroll`、`canvas`
  - 各要素は gpui-kit の部品と1対1に対応する。
  - 寸法は Size（xs / sm / md / lg）と、gap・padding の段階値だけ。px の自由指定はさせない。
  - 色は `TokenRef` だけ。文言は i18n キーか、その Extension が生成した値。
- 状態が変わったときだけ、Host スレッドで `render` を実行する。前回のツリーとの差分パッチだけを UI スレッドに送る。
- 入力イベントは `(view-id, element-id, event)` の形で、非同期に Extension へ返す。
- `canvas` は描画命令の一覧を受け取る（グラフなどの自由な描画用）。色はここでも `TokenRef` だけ。

### 4.5 実行時の安全
- Extension ごとに wasmtime の `Store` を1つ持つ。wasmtime の Engine は共有する。
- 時間切れ（epoch 割り込み）とメモリ上限を設ける。
- trap した Extension だけを無効にし、通知する。

---

## 5. 起動の流れ

すべての Extension を起動直後から非同期で立ち上げ、**権限ごとに段階的に反映する**。

```
0ms    ウィンドウを表示（前回のテーマ・文言・レイアウトのキャッシュで最初のフレームを描く）
       ↓ ここから先はすべて非同期。UI は操作可能なまま
段階1  データ系の権限（WASM 不要）: theme.* / locale.* / theme.icons / keybindings / language の文法
段階2  UI の登録: ui.*（スロットに枠を差し込み、中身は Skeleton で表示）
段階3  機能の起動: main の WASM → editor.* / language の言語サーバー / commands
段階4  外部との接続: network / process を使う初期化
```

1. **最初のフレームは拡張を待たない。** 終了時に、解決済みの色の表・文言の表・レイアウトをキャッシュに保存しておく。起動時はそれで描くので、色がちらつかない。拡張が更新されていた場合は、段階1が終わった時点で差分を当てる。
2. **終わったものから反映する。** 全員の完了は待たない。
3. **同時に起動する数に上限を設ける**（CPU コア数程度）。起動にかかった時間は Extension ごとに記録し、設定画面で確認できるようにする。
4. **ユーザーの操作で優先順位を上げる。** ファイルを開いた、パネルを開いた、コマンドを実行した、といったときは、その Extension を待ち行列の先頭に移す。
5. **失敗は隔離する。** 起動に失敗・時間切れになった Extension だけを無効にし、通知する。他の起動は止めない。

メモリ対策として、しばらく使われていない Extension を一時停止する仕組みを後から足せるようにしておく（最初は入れない）。

---

## 6. i18n（多言語化）

### 6.1 方式
- **メッセージ形式:** [Fluent](https://projectfluent.org/)（`fluent-bundle`）。複数形、性、変数、日付と数値の書式を、言語ごとに正しく扱える。
- **キーの規則:**
  - Core: `core.<領域>.<名前>`（例: `core.menu.file`、`core.status.line-col`）
  - Extension: `<extension-id>.<名前>`
  - Fluent の識別子には `.` を使えないため、`.ftl` ファイルの中では `.` を `-` に置き換えて書く（`core.menu.file` → `core-menu-file`）。コードからは `.` 区切りのまま引く。
- **マニフェストでの参照:** `"%key%"` と書くと i18n キーとして扱う（`name`、コマンドの `title` など）。
- **UI ツリーの文言:** `text(key, args)` で i18n キーを渡す。ユーザーのデータ（ファイル名やコードなど）は翻訳しない生の文字列として渡し、種類を区別する。

### 6.2 文言の出どころ（Extension で言語を追加できる）
| 権限 | できること | 例 |
|---|---|---|
| `locale.messages` | 自分の文言を複数の言語で持つ（`l10n/en.ftl`、`l10n/ja.ftl`…） | Claude 拡張の英語・日本語 |
| `locale.pack` | 新しい言語を追加し、Core と他の Extension の文言を翻訳する | 「日本語」言語パック、「Español」言語パック |

言語パックの `extension.yaml` の例:
```yaml
id: nucleus.locale-ja
permissions:
  locale: [pack]
locale:
  language: ja               # BCP 47 の言語タグ
  label: 日本語
  translates: [core, anthropic.claude, nucleus.git]   # 翻訳対象
l10n: l10n/                  # l10n/core.ftl, l10n/anthropic.claude.ftl …
```

### 6.3 解決の順番
1. ユーザーが選んだ言語（例: `ja-JP` → `ja`）の、言語パックによる訳
2. 同じ言語の、Extension 自身が持つ訳（`locale.messages`）
3. 英語（`en`、必須）

どこにもないキーは、キー名をそのまま表示し、開発時の警告に出す。

### 6.4 実装の要点
- 解決済みの文言の表は、Theme と同じく起動時のキャッシュから読む。言語を切り替えたときは表を入れ替え、全画面を再描画する。
- フォント: UI 用フォントに CJK などの字形がない場合に備えて、言語ごとの代替フォントのリストを Theme のフォント設定に持たせる（例: `ja → Noto Sans JP`）。
- 右から左に書く言語（RTL）は、最初は対象外。将来に備えて、レイアウト方向の情報だけは持っておく。
- 翻訳の進み具合（キーがどれだけ埋まっているか）を、言語パックごとに設定画面で表示する。

---

## 7. Core のサブシステム（要点）
- **Editor:** gpui-kit の code editor（tree-sitter の差分ハイライト、補完・ホバーの土台）を使う。Nucleus 側は Document（パス、保存済みの版、改行コード、エンコーディング）だけを持つ。保存は Rope のスナップショットをバックグラウンドで書き出す。
- **Project:** フォルダは展開されたときに直下だけをバックグラウンドで読む（`ignore` で `.gitignore` を反映）。監視も読み込み済みのフォルダだけを再帰なしで行い（`notify`）、変更があったフォルダだけを読み直す。ツリーは `uniform_list` で、画面に見えている行だけを描く。描画中にファイルの状態を問い合わせる（`stat`）ことはしない。
- **Search:** `grep-searcher` + `ignore` で、結果を少しずつ UI に流す。
- **Terminal:** `alacritty_terminal` の VT 解釈と PTY（Windows は ConPTY）を使う。PTY の読み書きは alacritty の EventLoop が専用スレッドで行い、UI には更新の通知だけが届く（届いた分をまとめて1回描く）。描画は見えている行だけを、同じ見た目の文字をまとめて行う。全角文字は2マス幅に固定する。色は ANSI 16色と既定色をテーマの Token（`terminal.*`）から取り、256色の残りとプログラムが指定した 24bit 色は `theme` クレートの関数で変換する。
- **LSP:** Core のクライアントとして実装する。どのサーバーをどう起動するかは、Extension（`language` 権限）が返す。
- **Settings:** 既定 / ユーザー / ワークスペースの3層。JSON とスキーマで検証し、ホットリロードする。設定画面の項目名と説明も i18n キーで持つ。

---

## 8. 性能の目標（CI で計測する）
| 項目 | 目標 |
|---|---|
| キー入力から描画まで（10万行のファイル） | < 8ms |
| 起動から最初のフレームまで | < 300ms（Extension の数に依存しない） |
| アイドル時の CPU 使用率 | 約 0% |
| テーマ・言語の切り替え | 1フレーム |
| Extension の UI パッチの反映 | 1フレーム以内 |

---

## 9. 実装の順番
1. **P0 基盤:** Cargo workspace、`theme`（Core Token の生成、Nucleus One Dark/Light、キャッシュ）、`i18n`（Fluent、en/ja）、ウィンドウと Workspace の骨格（TitleBar・DockArea・StatusBar） — **完了**（Dock のレイアウトの保存と復元は P2 で行う）
2. **P1 エディタ:** フォルダを開く、ProjectTree、ファイルを開く・編集・保存、タブ、検索バー — **完了**
   - 暫定: tree-sitter の文法は今は gpui-kit の機能として Core に同梱している。P4 で `language` 権限の Extension に移す。
   - 未対応: 開いたファイルをツリーで表示する（reveal）。
3. **P2 操作系:** コマンドパレット、ファイル検索、設定、キーバインド — **完了**
   - コマンドパレット（`Ctrl+Shift+P`）とファイル検索（`Ctrl+P`）は gpui-kit の `Command` 部品に、自前のあいまい検索（nucleo）の結果を渡す。
   - 未保存のタブは Atom と同じく閉じるボタンの代わりに ● を出し、`Ctrl+W`・ウィンドウを閉じる・終了で「保存 / 保存しない / キャンセル」を確認する。確認ダイアログは gpui の prompt を差し替えた Token だけで描くもの。
   - 設定は 既定 → ユーザー → フォルダ の3層の JSON（コメント可）。保存すると即座に反映する（テーマ、表示言語、エディタのフォントとタブ幅など）。キーバインドは設定の `keymap` で足せる（反映は再起動後。gpui のキーバインドは個別に外せないため）。
   - セッション（開いていたフォルダ・ファイル・ドックの状態）を保存し、次回の起動で復元する。gpui-kit の `DockArea::load` は使わず、Nucleus 独自の形式で持つ。
   - 検索と置換は Atom 式の下部パネル（`Ctrl+F` / `Ctrl+H`、`F3` / `Shift+F3`）。gpui-kit エディタの検索エンジンが対応していないため、正規表現と単語単位の検索は未対応（大文字小文字の区別のみ）。
4. **P3 端末:** Terminal（下ドック） — **完了**
   - シェルを最初に表示したときに起動（アプリの起動時間に影響させない）。複数のターミナル（`Ctrl+Shift+T`）、表示の切り替え（`Ctrl+`` ` ``）、スクロールバック 10,000 行、ブラケットペースト、終了後の再起動。
   - シェルで使う `Ctrl+B` / `Ctrl+P` / `Ctrl+W` などは、ターミナル内ではワークスペースのショートカットにせずシェルへ渡す。
   - 未対応: マウスでの選択とコピー、マウス入力の転送（vim などのマウス操作）、ターミナル内での IME 変換（変換確定後の文字は入る）。
5. **P4 Extension 基盤:** マニフェストと権限、起動の流れ、データ系の権限（テーマ・言語パック・文法） — **完了**
   - `extension_host`: `extension.yaml` の読み込みと検証（未知の権限、宣言していない機能の使用、名前空間の違う Token、ワイルドカードの network / process はエラー）。組み込み（リポジトリの `extensions/` をアプリに埋め込む）とユーザーのもの（`~/.local/share/nucleus/extensions/`）を探す。同じ ID ならユーザーのものを使う。
   - 段階1（データ系の権限）を起動後にバックグラウンドで読み、UI スレッドで1回だけ反映する: `theme.colors`（テーマ一覧に追加）、`theme.tokens`（拡張 Token）、`locale.messages` / `locale.pack`（Fluent）、`language.grammars`（ファイルと言語の対応）。
   - テーマ・表示言語はパレットから選べる（`Theme: Select Theme…` / `Display Language: Select…`）。選んだ値はユーザー設定に1行だけ書き込む（コメントは残す）。
   - 設定 `disabled_extensions` で Extension を無効にできる。
   - 例: 組み込みのテーマ Extension `extensions/nucleus.theme-nord`、言語パックの例 `examples/extensions/locale-es`。
   - 未対応: `theme.icons`（アイコンテーマ）、Extension からの tree-sitter 文法の読み込み（文法は今も Core に同梱。対応付けだけ Extension で足せる）。
6. **P5 WASM:** `core@1` の API、コマンド、言語サーバーの起動設定 → LSP — **完了**
   - `lsp`: 自前の LSP クライアント（JSON-RPC、読み書きは専用スレッド、位置は utf-32 を優先して交渉）。`contributes.language_servers` と `language: [servers]` + `process: [<コマンド>]` で起動する。組み込みの `extensions/nucleus.lang-rust`（rust-analyzer）。
   - エディタ: 診断（下線と Problems パネル、ステータスバーの件数）、補完、ホバー、定義へ移動（`F12`、別ファイルにも飛ぶ）。
   - WASM: wasmtime の Component Model。WIT は `crates/extension_api/wit/nucleus.wit`（`nucleus:extension@1.0.0`）、Extension 作者向けの SDK は `crates/extension_api`（crate 名 `nucleus-extension-api`、`export_extension!`）。`wasm32-wasip2` でビルドする。
   - ホスト関数（`core@1`）: `log` / `notify` / `workspace-root` / `read-file`（`workspace: [fs.read]`）/ `write-file`（`workspace: [fs.write]`）/ `active-document`（`editor: [read]`）/ `replace-active-text`（`editor: [edit]`、元に戻せる）/ `run-process`（`process: [<コマンド>]`、名前だけ。パスは不可）。権限は呼ぶたびに確かめ、なければエラーを返す。ファイルはワークスペースの中だけ（`..`・外への絶対パス・シンボリックリンクでの脱出を拒む）。
   - 段階3: 段階1の後に、`main` を持つ Extension を1つずつ専用スレッドで起動して `activate` を呼ぶ。1回の呼び出しは 5 秒（epoch 割り込み）、メモリは 64MB まで。打ち切られた Extension はそれ以降止める。WASI は p2 で、ファイル・ネットワーク・環境変数は渡さない。
   - `contributes.commands` はコマンドパレットに並び、`key` でショートカットも登録される。コマンド ID は `<Extension 名>.` で始める。
   - 例: 組み込みの `extensions/nucleus.word-tools`（ソースは `extensions-src/word-tools`、`build.sh` で WASM を作り直す）。単語数を数える・行を並べ替える（`Ctrl+Alt+S`）。
   - 未対応: `run-process` の時間切れ（WASM の 5 秒の外で待つ）、同時起動数の上限と優先順位、`network` 権限を使う API、Extension の言語サーバー以外からの LSP 利用、utf-16 のサーバーでの診断位置の厳密な変換。
7. **P6 UI API:** `ui@1`、`ui_bridge`、スロット → 公式の Git Extension を UI 付きで作る — **完了（最初の版）**
   - WIT の `ui` interface（同じ `nucleus:extension@1.0.0` パッケージ。manifest の `api.ui: 1` で使うことを宣言する）。Extension は `render(view) -> list<node>` で要素の木を返し、`handle-ui-event(view, key, event)` でイベントを受け取る。木は平らな配列（0 番が根、子は添字）で、Nucleus が形を確かめてから描く（範囲外・循環・1万要素超はエラー）。
   - 要素: `stack`（縦横・gap・padding・grow・spread）、`scroll`、`text`、`icon`、`button`、`input`（1行 / 複数行）、`list-item`（アイコン・詳細・印・ホバーで出るボタン）、`section`（折りたためる見出し・件数・ボタン）、`divider`、`skeleton`。色は役割（tone → Token）、寸法は段階値（none / xs 4 / sm 8 / md 12 / lg 16px）だけ。
   - 入力欄の中身と見出しの開閉は Nucleus が持つ。Extension は `change` / `submit` で値を受け取り、`revision` を増やしたときだけ中身を置き換えられる（コミット後に空にする、など）。
   - 描き直し: コマンド・UI のイベント・ワークスペースのイベント（`root-changed` / `file-saved` / `window-focused`）を処理した後に、その Extension のビューを描き直す。木は毎回まるごと返す（差分パッチは今後）。
   - `contributes.views`（`location: left | right | bottom`、`ui: [panel.<場所>]` の権限）。段階2として、段階1の後にドックへ置き、中身は Extension の起動まで Skeleton。右ドックでは Extension のビューを先頭に並べる。
   - ホスト関数 `open-file(path, line)` を追加（ワークスペース内だけ）。
   - 公式の Git Extension `extensions/nucleus.git`（ソースは `extensions-src/git`、権限は `ui: [panel.right]` と `process: [git]` だけ）: 変更の一覧（未ステージ / ステージ済み、M・A・D・R・U の印）、ファイルごと・まとめてのステージと取り消し、コミット（`Ctrl+Enter`、1行目の文字数表示）、Push / Fetch、ファイルを開く、空のフォルダでの `git init`。コマンド `Git: Refresh` / `Git: Stage All Changes` / `Git: Commit Staged Changes`。
   - 未対応: ガター（行ごとの変更の印、`editor: [decorate]`）とプロジェクトツリーの変更の印、差分表示（P7）、`tabs` / `markdown` / `code` / `canvas` などの要素、ステータスバー・タイトルバーのスロット、Extension の文言の多言語化（Git の画面は英語のみ）、描き直しの差分パッチ。
8. **P7 仕上げ:** プロジェクト全体検索、Diff、拡張機能の管理画面、Light テーマの調整 — **進行中**
   - プロジェクト全体の検索と置換（左ドックの Search、`Ctrl+Shift+F`）: `.gitignore` に従って辿り、正規表現・大文字小文字・単語単位・対象のファイル（glob、`!` で除外）を切り替えられる。入力のたびに前の検索を取り消してバックグラウンドで探す（5,000 件まで、2MB を超えるファイルとバイナリは飛ばす）。結果はファイルごとにまとめ、見えている行だけを描く。行をクリックするとその位置を開く。すべて置換は確認してから行い、開いているファイルはエディタの中で（元に戻せる）、それ以外はディスク上で置き換える。正規表現では `$1` が使える。
