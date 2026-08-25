use serde_json::Value;

pub enum PluginAction {
    OpenFile { path: String },
    OpenTab { title: String, content: String },
    CloseTab { title: String },
    FocusEditor,
    ShowNotification { message: String },
    OpenPanel { id: String },
    ClosePanel { id: String },
    SetStatusBarItem { id: String, text: String },
    UpdateSetting { key: String, value: Value },
    InternalProcessOutput { id: String, stdout: String, code: i32 },
    TerminalWrite { text: String },
    TerminalClear,
}
