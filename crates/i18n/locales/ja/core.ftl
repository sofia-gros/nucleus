## Nucleus のコア文字列（日本語）

core-app-name = Nucleus
core-window-title = { $file } — { $project }
core-window-title-empty = Nucleus

## メニュー
core-menu-file = ファイル
core-menu-edit = 編集
core-menu-view = 表示
core-menu-selection = 選択
core-menu-find = 検索
core-menu-extensions = 拡張機能
core-menu-help = ヘルプ

## ドックとパネル
core-panel-project = プロジェクト
core-panel-search = 検索
core-panel-git = Git
core-panel-outline = アウトライン
core-panel-terminal = ターミナル
core-panel-problems = 問題
core-panel-output = 出力
core-dock-toggle-left = 左ドックの表示切り替え
core-dock-toggle-bottom = 下ドックの表示切り替え
core-dock-toggle-right = 右ドックの表示切り替え

## プロジェクトパネル
core-project-empty-title = プロジェクトが開かれていません
core-project-empty-body = フォルダを開くと、ここにファイルが表示されます。
core-project-open-folder = フォルダを開く

## ようこそ
core-welcome-tab = ようこそ
core-welcome-tagline = 自由に拡張できる、とても速いエディタ — Rust と gpui-kit 製
core-welcome-start = はじめる
core-welcome-recent = 最近のプロジェクト
core-welcome-open-folder = フォルダを開く…
core-welcome-new-file = 新しいファイル
core-welcome-clone = リポジトリをクローン…
core-welcome-command-palette = コマンドパレット
core-welcome-no-recent = 最近のプロジェクトはまだありません
core-welcome-show-on-startup = 起動時にようこそページを表示する
core-welcome-tip-find-file = ファイルを探す
core-welcome-tip-search = プロジェクト内を検索
core-welcome-tip-terminal = ターミナルの表示切り替え
core-welcome-tip-settings = 設定

## ステータスバー
core-status-cursor = { $line }:{ $column }
core-status-errors = { $count }
core-status-warnings = { $count }
core-status-extensions-loading = 拡張機能を読み込み中（{ $count }件）…

## まだ実装していないパネル
core-panel-coming-soon = 「{ $panel }」はまだ使えません。

## プロジェクトツリーとファイル
core-project-loading = 読み込み中…
core-project-read-error = このフォルダを読めませんでした: { $error }
core-dialog-open-folder = フォルダを開く
core-notify-open-failed = { $file } を開けませんでした: { $error }
core-notify-binary = { $file } はバイナリファイルのようなので開きませんでした。
core-notify-too-large = { $file } は大きすぎるため開けません。
core-notify-save-failed = { $file } を保存できませんでした: { $error }
core-status-language-text = プレーンテキスト

## コマンド
core-command-find-file = ファイルへ移動
core-command-open-folder = フォルダを開く…
core-command-save = 保存
core-command-find-in-file = ファイル内を検索
core-command-close-tab = タブを閉じる
core-command-toggle-left-dock = 左ドックの表示切り替え
core-command-toggle-bottom-dock = 下ドックの表示切り替え
core-command-toggle-right-dock = 右ドックの表示切り替え
core-command-toggle-theme = テーマ: ライト / ダークの切り替え
core-command-language-en = 表示言語: 英語 (English)
core-command-language-ja = 表示言語: 日本語
core-command-command-palette = すべてのコマンドを表示
core-command-quit = 終了

## パレット
core-palette-commands-placeholder = コマンドを入力…
core-palette-files-placeholder = ファイル名で検索…
core-palette-no-results = 一致するものがありません
core-palette-no-project = フォルダを開くと、ファイルを検索できます。
core-palette-indexing = ファイルの一覧を作成中…

## 未保存の変更
core-prompt-unsaved-one = { $file } への変更を保存しますか？
core-prompt-unsaved-many = { $count } 個のファイルに未保存の変更があります。閉じる前に保存しますか？
core-prompt-unsaved-detail = 保存しないと、変更は失われます。
core-prompt-save = 保存
core-prompt-save-all = すべて保存
core-prompt-dont-save = 保存しない
core-prompt-cancel = キャンセル

## 設定
core-command-open-settings = 設定: 設定ファイル（JSON）を開く
core-command-open-workspace-settings = 設定: このフォルダの設定ファイル（JSON）を開く
core-notify-settings-error = 設定を反映できませんでした: { $error }
core-notify-keymap-restart = キーバインドの変更は Nucleus の再起動後に反映されます。

## 検索と置換（エディタ）
core-find-title = 現在のファイル内を検索
core-find-hint = 入力すると検索します。Enter: 次へ、Shift+Enter: 前へ、Esc: 閉じる
core-find-results = 「{ $query }」 { $count } 件中 { $current } 件目
core-find-no-results = 「{ $query }」は見つかりません
core-find-find-placeholder = 現在のファイル内を検索
core-find-replace-placeholder = 置換後の文字列
core-find-find = 検索
core-find-replace = 置換
core-find-replace-all = すべて置換
core-find-case-sensitive = 大文字と小文字を区別
