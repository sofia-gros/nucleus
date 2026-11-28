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
core-command-open-settings = 設定: 設定を開く
core-command-open-settings-json = 設定: 設定ファイル（JSON）を開く
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
core-extensions-settings = 設定
core-extensions-on = オン
core-extensions-off = オフ
core-extensions-open = 拡張機能を開く
core-extensions-updates-available = 拡張機能の更新が { $count } 件あります: { $list }
core-status-extension-updates = 更新 { $count } 件
core-extensions-updated-notify = { $name } を { $version } に更新しました。
core-extensions-number-invalid = 数を入れてください（{ $range }）

# Menus
core-menu-item-new-file = 新しいファイル
core-menu-item-open-recent = 最近開いたフォルダ
core-menu-item-undo = 元に戻す
core-menu-item-redo = やり直す
core-menu-item-cut = 切り取り
core-menu-item-copy = コピー
core-menu-item-paste = 貼り付け
core-menu-item-indent = インデント
core-menu-item-outdent = インデントを戻す
core-menu-item-select-all = すべて選択
core-menu-item-select-to-line-start = 行頭まで選択
core-menu-item-select-to-line-end = 行末まで選択
core-menu-item-find = 検索
core-menu-item-replace = 置換
core-menu-item-find-next = 次を検索
core-menu-item-find-previous = 前を検索
core-menu-item-find-replace = ファイル内の検索・置換
core-menu-item-find-next-previous = 次 / 前を検索
core-menu-item-go-to-definition = 定義へ移動
core-menu-item-toggle-docks = 左 / 下 / 右のドックを開閉
core-menu-item-documentation = ドキュメント
core-menu-item-keyboard-shortcuts = キーボードショートカット
core-menu-item-report-issue = 問題を報告
core-menu-item-about = Nucleus について
core-clone-choose-folder = フォルダを選ぶ…
core-clone-hint = 次に選ぶフォルダの中に、新しいフォルダを作ってクローンします。
core-clone-started = { $url } をクローンしています…
core-clone-failed = クローンできませんでした: { $error }
core-recent-missing = { $path } はもうありません。

# Settings
core-settings-title = 設定
core-settings-general = 全般
core-settings-editor = テキストエディタ
core-settings-extensions = 拡張機能
core-settings-search = 設定を検索
core-settings-open-json = settings.json を開く
core-settings-invalid = この値は使えません。
core-settings-system-language = OS に合わせる
core-settings-more-in-json = キーの割り当てと、言語ごとの設定（"[rust]": {"{"} "editor.tab_size": 2 {"}"}）は settings.json で変えられます。
core-settings-item-theme = 配色テーマ
core-settings-item-theme-description = ウィンドウ全体の色。拡張機能でテーマを足せます。
core-settings-item-language = 表示言語
core-settings-item-language-description = メニューやメッセージの言語。言語パックで増やせます。
core-settings-item-restore-session = 前回の状態を復元
core-settings-item-restore-session-description = 起動したときに、前回のフォルダとファイルを開き直します。
core-settings-item-editor-font-family = フォント
core-settings-item-editor-font-family-description = エディタで使うフォント。
core-settings-item-editor-font-size = 文字の大きさ
core-settings-item-editor-font-size-description = px で指定します（8〜40）。
core-settings-item-editor-tab-size = タブの幅
core-settings-item-editor-tab-size-description = タブを何文字分で表示するか（1〜16）。
core-settings-item-editor-hard-tabs = タブ文字を入れる
core-settings-item-editor-hard-tabs-description = 空白の代わりにタブ文字を入れます。
core-settings-item-editor-soft-wrap = 折り返し
core-settings-item-editor-soft-wrap-description = 長い行をエディタの端で折り返します。
core-settings-item-editor-line-numbers = 行番号
core-settings-item-editor-line-numbers-description = 左端に行番号を出します。
core-settings-item-editor-indent-guides = インデントの線
core-settings-item-editor-indent-guides-description = インデントの深さごとに縦線を引きます。
core-settings-item-check-extension-updates = 拡張機能の更新を確かめる
core-settings-item-check-extension-updates-description = 起動したときに、入れた拡張機能の新しい版があるか確かめます。
core-settings-item-extensions-registry = 拡張機能の一覧の場所
core-settings-item-extensions-registry-description = 拡張機能を探す index.json。空にすると公式の一覧を使います。
core-settings-files = ファイル
core-settings-workbench = ワークベンチ
core-settings-explorer = エクスプローラー
core-settings-search-category = 検索
core-settings-terminal = ターミナル
core-settings-list-add = パターンを入れて Enter で追加
core-settings-list-add-number = 列の番号を入れて Enter で追加
core-settings-value-ctrl-cmd = Ctrl（macOS は Cmd）
core-settings-value-alt = Alt
core-settings-value-left = 左
core-settings-value-right = 右
core-settings-item-editor-relative-line-numbers = 行番号の相対表示
core-settings-item-editor-relative-line-numbers-description = 行番号を、カーソルのある行からの距離で出します。カーソルの行は本来の番号です。
core-settings-item-editor-rulers = ルーラー
core-settings-item-editor-rulers-description = この列に縦の目安線を引きます（例: 80 と 120）。
core-settings-item-editor-multi-cursor-modifier = カーソルを足すキー
core-settings-item-editor-multi-cursor-modifier-description = クリックでカーソルを足すときに押すキー。もう一方のキーは定義へ移動です。Ctrl+D で次の同じ文字を、Ctrl+Shift+L ですべての同じ文字を選び、Ctrl+Alt+↑↓ で上下にカーソルを足せます。
core-settings-item-workbench-side-bar-location = サイドバーの位置
core-settings-item-workbench-side-bar-location-description = ファイルツリーと検索を置く側。アウトラインや拡張機能のビュー（Git など）は反対側に置きます。
core-settings-value-none = なし
core-settings-value-same = 元の行と同じ
core-settings-value-all = すべて
core-settings-value-blink = 点滅する
core-settings-value-solid = 点滅しない
core-settings-value-off = オフ
core-settings-value-after-delay = 一定時間後
core-settings-value-on-focus-change = フォーカスが外れたとき
core-settings-value-auto = 自動（OS に合わせる）
core-settings-value-lf = LF
core-settings-value-crlf = CRLF
core-settings-value-default = フォルダが先
core-settings-value-mixed = 混ぜる
core-settings-value-files-first = ファイルが先
core-settings-value-type = 種類ごと
core-settings-item-window-zoom-level = 拡大率
core-settings-item-window-zoom-level-description = ウィンドウ全体の大きさ。0 が標準で、1 増える（減る）ごとに 20% 大きく（小さく）なります。
core-settings-item-editor-line-height = 行の高さ
core-settings-item-editor-line-height-description = 1 行の高さを、文字の大きさの何倍にするか（1〜3）。
core-settings-item-editor-detect-indentation = インデントの自動検出
core-settings-item-editor-detect-indentation-description = ファイルを開いたとき、中身からタブの幅とタブ文字を使うかを決めます。
core-settings-item-editor-wrapping-indent = 折り返しのインデント
core-settings-item-editor-wrapping-indent-description = 折り返した行をどこから始めるか。
core-settings-item-editor-render-whitespace = 空白文字の表示
core-settings-item-editor-render-whitespace-description = 空白とタブを記号で見せます。
core-settings-item-editor-folding = 折りたたみ
core-settings-item-editor-folding-description = コードのまとまりを折りたためるようにします。
core-settings-item-editor-auto-closing-brackets = 括弧を自動で閉じる
core-settings-item-editor-auto-closing-brackets-description = 開き括弧や引用符を入れたら、閉じる方も入れます。
core-settings-item-editor-auto-indent = 自動インデント
core-settings-item-editor-auto-indent-description = Enter で改行したとき、言語に合わせて字下げします。
core-settings-item-editor-scroll-beyond-last-line = 最終行より下へスクロール
core-settings-item-editor-scroll-beyond-last-line-description = 最後の行より下までスクロールできるようにします。
core-settings-item-editor-cursor-surrounding-lines = カーソルの上下の余白
core-settings-item-editor-cursor-surrounding-lines-description = カーソルの上下に、少なくとも見せておく行数（0 で標準）。
core-settings-item-editor-cursor-blinking = カーソルの点滅
core-settings-item-editor-cursor-blinking-description = カーソルを点滅させるかどうか。
core-settings-item-editor-highlight-line-only-when-focused = 使っているときだけ行を強調
core-settings-item-editor-highlight-line-only-when-focused-description = カーソルのある行の強調を、そのエディタを使っているときだけにします。
core-settings-item-editor-format-on-save = 保存時に整形
core-settings-item-editor-format-on-save-description = 保存するとき、言語サーバーでファイルを整形します。
core-settings-item-editor-hover = ホバー表示
core-settings-item-editor-hover-description = コードにマウスを乗せたとき、言語サーバーの説明を出します。
core-settings-item-files-auto-save = 自動保存
core-settings-item-files-auto-save-description = Ctrl+S を押さなくても保存します。
core-settings-item-files-auto-save-delay = 自動保存までの時間
core-settings-item-files-auto-save-delay-description = 「一定時間後」のとき、最後に変更してから保存するまでの時間（ミリ秒）。
core-settings-item-files-trim-trailing-whitespace = 行末の空白を削除
core-settings-item-files-trim-trailing-whitespace-description = 保存するとき、行の終わりの空白とタブを消します。
core-settings-item-files-insert-final-newline = 最後に改行を入れる
core-settings-item-files-insert-final-newline-description = 保存するとき、ファイルの最後に改行がなければ足します。
core-settings-item-files-trim-final-newlines = 末尾の余分な改行を削除
core-settings-item-files-trim-final-newlines-description = 保存するとき、ファイルの最後の改行を 1 つだけにします。
core-settings-item-files-eol = 改行コード
core-settings-item-files-eol-description = 新しいファイルの改行コード。いまあるファイルは元の改行コードのままです。
core-settings-item-files-exclude = 除外するファイル
core-settings-item-files-exclude-description = ファイルツリー・Ctrl+P・検索に出さないファイルとフォルダの glob パターン（例: **/node_modules）。
core-settings-item-workbench-status-bar = ステータスバー
core-settings-item-workbench-status-bar-description = ウィンドウの下にステータスバーを出します。
core-settings-item-workbench-tree-indent = ツリーの字下げ
core-settings-item-workbench-tree-indent-description = ファイルツリーの 1 段の字下げ（px、4〜40）。
core-settings-item-workbench-tree-indent-guides = ツリーの字下げの線
core-settings-item-workbench-tree-indent-guides-description = ファイルツリーの段ごとに縦線を引きます。
core-settings-item-workbench-startup-welcome = 起動時のようこそ画面
core-settings-item-workbench-startup-welcome-description = 起動したときに、ようこそ画面を出します。
core-settings-item-explorer-confirm-delete = 削除の確認
core-settings-item-explorer-confirm-delete-description = ファイルやフォルダをゴミ箱へ移す前に確かめます。
core-settings-item-explorer-sort-order = 並べ方
core-settings-item-explorer-sort-order-description = ファイルツリーでのファイルとフォルダの並べ方。
core-settings-item-explorer-auto-reveal = 開いたファイルをツリーで表示
core-settings-item-explorer-auto-reveal-description = 使っているエディタのファイルを、ファイルツリーでも選びます（フォルダも開きます）。
core-settings-item-search-exclude = 検索から除外
core-settings-item-search-exclude-description = 検索しないファイルとフォルダの glob パターン（「除外するファイル」に加えて）。
core-settings-item-search-use-ignore-files = .gitignore に従う
core-settings-item-search-use-ignore-files-description = .gitignore などに書かれたファイルは検索しません。
core-settings-item-search-smart-case = 大文字小文字の自動判定
core-settings-item-search-smart-case-description = 検索する文字に大文字が入っているときだけ、大文字と小文字を区別します。
core-settings-item-search-max-results = 検索結果の上限
core-settings-item-search-max-results-description = この数だけ見つかったら検索をやめます。
core-settings-item-terminal-shell = シェル
core-settings-item-terminal-shell-description = 新しいターミナルで動かすプログラム（例: pwsh、cmd、bash）。空にすると OS の標準。
core-settings-item-terminal-shell-args = シェルの引数
core-settings-item-terminal-shell-args-description = シェルに渡す引数。
core-settings-item-terminal-font-family = フォント
core-settings-item-terminal-font-family-description = ターミナルのフォント。空にするとエディタと同じ。
core-settings-item-terminal-font-size = 文字の大きさ
core-settings-item-terminal-font-size-description = px で指定します（6〜72）。空にするとエディタと同じ。
core-settings-item-terminal-line-height = 行の高さ
core-settings-item-terminal-line-height-description = 1 行の高さを、文字の大きさの何倍にするか（1〜3）。
core-settings-item-terminal-scrollback = さかのぼれる行数
core-settings-item-terminal-scrollback-description = さかのぼって見られる行数。新しいターミナルから効きます。

# Project tree
core-tree-new-file = 新しいファイル…
core-tree-new-folder = 新しいフォルダ…
core-tree-rename = 名前を変更…
core-tree-delete = ごみ箱に移す
core-tree-copy-path = パスをコピー
core-tree-copy-relative-path = 相対パスをコピー
core-tree-open-in-terminal = ターミナルで開く
core-tree-ok = OK
core-tree-exists = { $name } はもうあります。
core-tree-invalid-name = / や \\ を含まない名前を入れてください。
core-tree-confirm-delete = { $name } をごみ箱に移しますか？
core-tree-delete-detail = ごみ箱から元に戻せます。
core-tree-exists-folder = { $dir } にはすでに { $name } という名前のフォルダがあります。
core-tree-exists-file = { $dir } にはすでに { $name } という名前のファイルがあります。
