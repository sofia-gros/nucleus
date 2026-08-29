/// エディタに対するアクション定義モジュール

use gpui::actions;

// エディタの編集・移動・保存・LSPアクション
actions!(
    editor,
    [
        Undo,
        Redo,
        Save,
        MoveLeft,
        MoveRight,
        MoveUp,
        MoveDown,
        MoveToBeginningOfLine,
        MoveToEndOfLine,
        MoveToTop,
        MoveToBottom,
        SelectLeft,
        SelectRight,
        SelectUp,
        SelectDown,
        SelectAll,
        InsertNewline,
        Delete,
        Backspace,
        DeleteLine,
        Cut,
        Copy,
        Paste,
        GoToDefinition,
        ShowHover,
        FindReferences,
        Rename,
        QuickFix,
        FormatDocument
    ]
);
