# vendor

gpui-kit 0.11.0 の部品のうち 2 つを、ここに取り込んで手直ししている（ルートの `Cargo.toml` の `[patch.crates-io]`）。
手直しはどれも小さく、`Nucleus:` で始まるコメントを付けてある（`grep -rn "Nucleus" vendor` で探せる）。
gpui-kit を上げるときは、新しい版を取り込み直して、下の手直しを入れ直す。

## gpui-base 0.11.0

- `src/input/editor/indent.rs`: ルーラー（`layout_rulers`、`set_rulers`）と、カーソルを足すキーの切り替え
  （`set_multi_cursor_with_ctrl`）。設定の `editor.rulers` と `editor.multi_cursor_modifier`。
- `src/input/base/state.rs`: 上の 2 つの状態（`rulers`、`multi_cursor_with_ctrl`）と、マウスを押したときの
  Alt と Ctrl/Cmd の入れ替え（`on_mouse_down` の始め）。
- `src/input/base/element.rs`: ルーラーを描く（インデントの線のすぐ後）。

## gpui-component 0.11.0

- `src/input/gutter_style.rs`: 行番号の相対表示（`with_relative_line_numbers`）。設定の `editor.relative_line_numbers`。
