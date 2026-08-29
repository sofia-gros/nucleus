#![recursion_limit = "512"]

/// Nucleus IDE コアライブラリ

pub mod editor;
pub mod plugin_manager;
pub mod workspace;
pub mod settings;
pub mod file_system;
pub mod project;
pub mod lsp;
pub mod terminal;
pub mod process;
pub mod keybindings;
pub mod theme;
pub mod search;
pub mod debug;
pub mod util;
