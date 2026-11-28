//! LSP クライアント（docs/ARCHITECTURE.md §7）。どのサーバーをどう起動するかは Extension
//! （`language.servers` 権限）が宣言し、通信は Core が行う。サーバーを動かすのはホスト（host クレート）で、
//! ここは JSON-RPC の要求と応答の対応付けだけを受け持つ（ブラウザ版でも動く）。

mod client;

pub use client::{LanguageServer, Link, ServerConfig, ServerNotification, Spawn, path_to_uri, uri_to_path};
pub use lsp_types;
