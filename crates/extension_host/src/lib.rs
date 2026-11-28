//! Extension の読み込みと、起動後の段階的な反映（docs/ARCHITECTURE.md §4, §5）。

pub mod api;
pub mod loader;
pub mod manifest;
pub mod registry;
pub mod runtime;
mod store;
pub mod ui;

pub use api::{ActiveDocument, EditorViewInfo, FileState, NoticeLevel, UiRequest};
pub use loader::{FailedExtension, LoadedExtension, discover, user_extensions_dir};
pub use manifest::Manifest;
pub use runtime::HostContext;
pub use store::{ExtensionCommand, ExtensionStore, ExtensionView, Stage, WasmHost, add_installed, start};
pub use ui::WorkspaceEvent;
