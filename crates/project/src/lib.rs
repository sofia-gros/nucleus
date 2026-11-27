//! プロジェクト（開いているフォルダ）のモデル。gpui に依存しない。
//!
//! フォルダの中身は、ツリーで展開されたときにそのフォルダだけを読む（docs/ARCHITECTURE.md §7）。
//! 読み込みは呼び出し側がバックグラウンドで [`read_dir`] を実行し、結果を [`ProjectTree::set_children`] で渡す。

mod tree;
mod watcher;

pub use tree::{DirState, Entry, EntryKind, ProjectTree, Row, read_dir};
pub use watcher::DirWatcher;
