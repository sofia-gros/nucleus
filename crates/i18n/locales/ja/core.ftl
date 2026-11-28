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

## ターミナル
core-terminal-exited = プロセスが終了しました。Enter で再起動します。
core-terminal-spawn-failed = ターミナルを起動できませんでした: { $error }
core-command-new-terminal = ターミナル: 新しいターミナル
core-command-toggle-terminal = ターミナル: 表示の切り替え

## 拡張機能・テーマ・表示言語
core-status-extensions-starting = 拡張機能を読み込み中…
core-notify-extension-failed = 拡張機能を読み込めませんでした: { $error }
core-extension-view-failed = このビューを表示できませんでした。
core-notify-extension-not-running = 拡張機能 { $id } は動いていません。
core-notify-extension-command-failed = コマンド { $command } が失敗しました: { $error }
core-command-select-theme = テーマ: テーマを選択…
core-command-select-language = 表示言語: 表示言語を選択…
core-palette-themes-placeholder = テーマを選択…
core-palette-languages-placeholder = 表示言語を選択…
core-palette-current = 使用中

## 言語サーバー
core-notify-lsp-failed = 言語サーバー { $server } を起動できませんでした: { $error }
core-problems-none = 問題は見つかっていません。

# プロジェクト検索
core-command-project-search = プロジェクト内を検索
core-search-placeholder = 検索
core-search-replace-placeholder = 置換
core-search-include-placeholder = 対象のファイル（例: src/**/*.rs）
core-search-searching = 検索中…
core-search-no-folder = フォルダを開くと検索できます。
core-search-no-results = 見つかりませんでした。
core-search-summary = { $files } 個のファイルに { $count } 件
core-search-summary-truncated = { $files } 個のファイルに { $count } 件以上（最初の分だけ表示）
core-search-regex = 正規表現
core-search-whole-word = 単語単位
core-search-replace-all = すべて置換
core-search-confirm-replace = { $files } 個のファイルの { $count } 件を「{ $replacement }」に置き換えますか？
core-search-replaced = { $files } 個のファイルの { $count } 件を置き換えました。

# Diff
core-diff-no-changes = 変更はありません。
core-diff-previous-hunk = 前の変更
core-diff-next-hunk = 次の変更
core-diff-open-file = ファイルを開く
core-diff-hunks = { $count } 個の変更
core-diff-stage-hunk = 変更をステージ
core-diff-revert-hunk = 変更を元に戻す

# Extensions
core-command-open-extensions = 拡張機能: 管理
core-extensions-title = 拡張機能
core-extensions-tab-installed = インストール済み
core-extensions-tab-browse = 探す
core-extensions-search = 拡張機能を検索
core-extensions-builtin = 組み込み
core-extensions-installed = インストール済み
core-extensions-disabled = 無効
core-extensions-restart-required = 再起動で反映
core-extensions-enable = 有効にする
core-extensions-disable = 無効にする
core-extensions-install = インストール
core-extensions-installing = インストール中…
core-extensions-update = { $version } に更新
core-extensions-uninstall = アンインストール
core-extensions-loading = 一覧を読み込んでいます…
core-extensions-load-failed = 一覧を読み込めませんでした: { $error }
core-extensions-retry = 読み込み直す
core-extensions-empty = 見つかりませんでした。
core-extensions-confirm-install = { $name } { $version } をインストールしますか？
core-extensions-permissions = この拡張機能が使う権限:
core-extensions-no-permissions = • 権限なし
core-extensions-strong-note = ⚠ は強い権限（ファイルの書き換え・プログラムの実行・通信）です。
core-extensions-installed-notify = { $name } をインストールしました。
core-extensions-install-failed = { $name } をインストールできませんでした: { $error }
core-extensions-confirm-uninstall = { $name } をアンインストールしますか？ Nucleus の再起動で消えます。
core-extensions-uninstalled-notify = { $name } をアンインストールしました。再起動すると反映されます。
