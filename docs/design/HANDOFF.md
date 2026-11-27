# Nucleus UI デザイン — 引き継ぎメモ

## 方針
- UI ライブラリ: **gpui-kit 0.11**（gpui + gpui-component、shadcn 由来）。デザインの値はすべて gpui-kit のソースから取り、コードにそのまま置き換えられるようにする。
- 見た目: shadcn の構造（角丸 6/8px、細い境界線、影）＋ **Atom One Dark / Light** の配色。
- レイアウト: Atom 風の Dock 構成（gpui-kit `DockArea`）。左ドック = Project / Search、右ドック = Git / Outline、下ドック = Terminal / Problems / Output。Activity Bar はなし。
- フォント: UI = Inter、コード = JetBrains Mono 13px / 行の高さ 20px。どちらも同梱して、OS による差をなくす。
- アイコン: Lucide（gpui-kit の IconName と同じ）。

## 進捗
Figma ファイル: https://www.figma.com/design/JKkYKtCWLklEPo0GYGzPPv （Starter プランの MCP 呼び出し上限、月20回に達したため中断）

完成している画面（1440×900, Dark）:
01 Welcome / 02 Editor（メイン） / 04 Command Palette / 05 File Finder / 06 Find & Replace（Atom 風の下部パネル） / 07 Project Search / 08 Git（右ドック） / 09 LSP（補完・ドキュメント・診断・Problems） / 10 Split Editor + Git Diff

残り: 11 Settings（Editor カテゴリ） / 12 Extensions（Packages） / 13 コンテキストメニュー・通知トースト・未保存ダイアログ / Light テーマ版の全画面

### 画面の記録（`screens/`、Figma からの書き出し）
| ファイル | 内容 | 注記 |
|---|---|---|
| `01-welcome.png` | Welcome（フォルダ未オープン） | |
| `02-editor-main.png` | メインのエディタ（1440×900 等倍） | 基準画面 |
| `04-command-palette.png` | コマンドパレット | 60% 縮小 |
| `05-file-finder.png` | ファイル検索 | 60% 縮小 |
| `06-find-replace.png` | 検索と置換（下部パネル） | 60% 縮小 |
| `07-project-search.png` | プロジェクト検索 | 修正前（パスが折り返す）。正は `07-project-search-left-dock-fixed.png` |
| `08-git-right-dock.png` | Git（右ドック） | 60% 縮小 |
| `09-lsp-completion-problems.png` | 補完・診断・Problems | 修正前（詳細が折り返す）。補完メニューの正は `09-lsp-completion-menu-fixed.png` |
| `10-split-diff.png` | 分割エディタ + Diff | 修正前（Diff の行背景が不透明）。右ペインの正は `10-split-diff-right-pane-fixed.png`（created/deleted.background = 14%） |
| `components/*.png` | Button / Tab / TreeItem / TitleBar / StatusBar | `tab.png` はコンポーネントセットの枠で上下が切れている |

デザインツールでの作業はここで終了する（Figma / Penpot ともにこれ以上使わない）。
以降は **既存の Figma デザインとこのメモを正として、Editor を直接実装する**。
残りの画面は、実装の中で上記トークン・寸法・既存コンポーネントに従って作る。

## トークン（Figma 変数名 = gpui-kit ThemeColor のキー。`/` を `.` に読み替える）
| key | One Dark | One Light |
|---|---|---|
| background | #282c34 | #fafafa |
| foreground | #abb2bf | #383a42 |
| border | #181a1f | #dbdbdc |
| muted.background / muted.foreground | #2c313a / #7f848e | #f0f0f1 / #8e8f96 |
| accent.background / accent.foreground | #2c313a / #d7dae0 | #e5e5e6 / #232324 |
| primary.background / .foreground | #568af2 / #ffffff | #526fff / #ffffff |
| primary.hover / primary.active | #6b99f4 / #4c7fe6 | #6b84ff / #4a63e6 |
| secondary.background / .foreground | #333842 / #d7dae0 | #e5e5e6 / #383a42 |
| secondary.hover / secondary.active | #3a404b / #2c313a | #dbdbdc / #d0d0d1 |
| input.border | #181a1f | #c8c8c9 |
| ring / caret | #568af2 / #528bff | #526fff / #526fff |
| popover.background / .foreground | #21252b / #abb2bf | #ffffff / #383a42 |
| list.background / list.hover.background | #21252b / #2c313a | #f5f5f6 / #e8e8e9 |
| list.active.background / list.active.border | #3a3f4b / #568af2 | #dbdbdc / #526fff |
| list.highlight.foreground | #61afef | #4078f2 |
| selection.background | #3e4451 | #e5e5e6 |
| sidebar.background / .foreground / .border | #21252b / #9da5b4 / #181a1f | #eaeaeb / #383a42 / #dbdbdc |
| sidebar.accent.background / .foreground | #2c313a / #d7dae0 | #dbdbdc / #232324 |
| tab_bar.background | #21252b | #eaeaeb |
| tab.foreground | #7f848e | #8e8f96 |
| tab.active.background / tab.active.foreground | #282c34 / #d7dae0 | #fafafa / #232324 |
| title_bar.background / status_bar.background | #21252b | #eaeaeb |
| danger / success / warning / info | #e06c75 / #98c379 / #e5c07b / #56b6c2 | #e45649 / #50a14f / #c18401 / #0184bc |
| scrollbar.thumb.background | #4e5666 @60% | #4f525e @30% |
| overlay | #000 @40% | #000 @15% |
| highlight: editor.background / foreground | #282c34 / #abb2bf | #fafafa / #383a42 |
| highlight: active_line.background | #2c313a | #f0f0f1 |
| highlight: line_number / active_line_number | #495162 / #abb2bf | #9d9d9f / #383a42 |
| highlight: created / modified / deleted | #98c379 / #e5c07b / #e06c75 | #50a14f / #c18401 / #e45649 |
| highlight: created/deleted.background | #98c379 @14% / #e06c75 @14% | #50a14f @14% / #e45649 @14% |
| syntax: keyword / function / string / number | #c678dd / #61afef / #98c379 / #d19a66 | #a626a4 / #4078f2 / #50a14f / #986801 |
| syntax: type / comment / variable,property | #e5c07b / #5c6370 (italic) / #e06c75 | #c18401 / #a0a1a7 / #e45649 |
| syntax: constant,attribute / operator,macro | #d19a66 / #56b6c2 | #986801 / #0184bc |

## 寸法（gpui-kit の既定値。rem = 16px）
- radius 6 / radius_lg 8。TitleBar 34、Dock TabBar 32（タブの左右 padding 12）、StatusBar 24（項目は py_1 px_2 text_xs）
- Button / Input: Medium = 高さ 32（text_sm 14）、Small = 高さ 24（text_xs 12）
- Tree / List の行 24（インデント = 8 + 深さ×16）、パレットの行 32、メニューの行 28
- 左ドック幅 280、右ドック 320、下ドック 240。ポップオーバー: popover.background + border + radius_lg + shadow/lg
- 未保存のタブ: 閉じるボタンの代わりに modified 色の 8px の丸（Atom 式）
