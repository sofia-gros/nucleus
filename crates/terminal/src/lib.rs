//! ターミナル（docs/ARCHITECTURE.md §7）。シェルと VT の解釈はホスト（host クレート）が行い、
//! 変わった行が remote の知らせで届く。ここはそれを gpui で描き、キー入力をホストへ送る。

pub mod keys;
mod model;

pub use model::{GridSize, Terminal, TerminalEvent};
mod element;
mod view;

pub use view::{TerminalView, init};
