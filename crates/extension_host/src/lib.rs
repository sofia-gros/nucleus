//! Extension の読み込みと、起動後の段階的な反映（docs/ARCHITECTURE.md §4, §5）。

pub mod loader;
pub mod manifest;
mod store;

pub use loader::{FailedExtension, LoadedExtension, discover, user_extensions_dir};
pub use manifest::Manifest;
pub use store::{ExtensionStore, Stage, start};
