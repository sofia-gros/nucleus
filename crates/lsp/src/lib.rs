//! LSP クライアント（docs/ARCHITECTURE.md §7）。どのサーバーをどう起動するかは Extension
//! （`language.servers` 権限）が宣言し、通信は Core が行う。読み書きは専用スレッドで行い、UI を止めない。

mod client;
mod transport;

pub use client::{LanguageServer, ServerConfig, ServerNotification, path_to_uri, uri_to_path};
pub use lsp_types;
