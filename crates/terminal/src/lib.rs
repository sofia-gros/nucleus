//! ターミナル（docs/ARCHITECTURE.md §7）。alacritty_terminal で VT を解釈し、gpui で描く。
//! PTY の読み書きは alacritty の EventLoop（専用スレッド）が行い、UI には更新の通知だけが届く。

pub mod keys;
mod model;

pub use model::{GridSize, Spawned, Terminal, TerminalEvent, spawn};
mod element;
mod view;

pub use view::{TerminalView, init};
