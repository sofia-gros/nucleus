//! プロジェクト（開いているフォルダ）のモデル。gpui にも OS にも依存しない（ブラウザ版でも使う）。
//!
//! フォルダの中身は、ツリーで展開されたときにそのフォルダだけを読む（docs/ARCHITECTURE.md §7）。
//! 読み込みはホストに頼み（`remote::Client::read_dir`）、結果を [`ProjectTree::set_children`] で渡す。

mod exclude;
pub mod search;
mod tree;

pub use exclude::Excludes;
pub use tree::{DirState, Entry, EntryKind, ProjectTree, Row, sort_entries};
