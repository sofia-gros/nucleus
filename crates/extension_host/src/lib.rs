//! Extension の読み込みと、起動後の段階的な反映（docs/ARCHITECTURE.md §4, §5）。

pub mod loader;
pub mod manifest;
pub mod runtime;
mod store;

pub use loader::{FailedExtension, LoadedExtension, discover, user_extensions_dir};
pub use manifest::Manifest;
pub use runtime::{ActiveDocument, HostContext, NoticeLevel, UiRequest, WorkspaceEvent, ui};
pub use store::{ExtensionCommand, ExtensionStore, ExtensionView, Stage, WasmHost, start};
