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
    FileSystemRead { plugin_id: String, req_id: String, path: String },
    FileSystemWrite { plugin_id: String, req_id: String, path: String, content: String },
    FileSystemReadComplete { req_id: String, content: Option<String>, error: Option<String> },
    FileSystemWriteComplete { req_id: String, error: Option<String> },
    RegisterCommand { plugin_id: String, command: String },
}
