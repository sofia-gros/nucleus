/// キーバインディングシステムの管理モジュール

use gpui::{App, KeyBinding};
use crate::editor::actions::*;
use crate::workspace::{OpenFileFinder, OpenCommandPalette, ToggleBottomPanel, ToggleLeftSidebar, ToggleRightSidebar};

/// アプリケーション全体の標準キーバインディングを登録する
pub fn init_keybindings(cx: &mut App) {
    cx.bind_keys([
        // ワークスペース UI 操作
        KeyBinding::new("ctrl-b", ToggleLeftSidebar, None),
        KeyBinding::new("ctrl-j", ToggleBottomPanel, None),
        KeyBinding::new("ctrl-r", ToggleRightSidebar, None),
        KeyBinding::new("ctrl-p", OpenFileFinder, None),
        KeyBinding::new("ctrl-shift-p", OpenCommandPalette, None),

        // エディタ操作
        KeyBinding::new("ctrl-s", Save, None),
        KeyBinding::new("ctrl-z", Undo, None),
        KeyBinding::new("ctrl-y", Redo, None),
        KeyBinding::new("ctrl-shift-z", Redo, None),
        KeyBinding::new("ctrl-a", SelectAll, None),
        KeyBinding::new("left", MoveLeft, None),
        KeyBinding::new("right", MoveRight, None),
        KeyBinding::new("up", MoveUp, None),
        KeyBinding::new("down", MoveDown, None),
        KeyBinding::new("shift-left", SelectLeft, None),
        KeyBinding::new("shift-right", SelectRight, None),
        KeyBinding::new("shift-up", SelectUp, None),
        KeyBinding::new("shift-down", SelectDown, None),
        KeyBinding::new("backspace", Backspace, None),
        KeyBinding::new("delete", Delete, None),
        KeyBinding::new("enter", InsertNewline, None),

        // LSP 操作
        KeyBinding::new("f12", GoToDefinition, None),
        KeyBinding::new("shift-f12", FindReferences, None),
        KeyBinding::new("f2", Rename, None),
        KeyBinding::new("ctrl-.", QuickFix, None),
        KeyBinding::new("shift-alt-f", FormatDocument, None),
    ]);
}
