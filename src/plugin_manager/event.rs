pub enum PluginEvent {
    FileOpened { path: String },
    FileSaved { path: String },
    ProcessOutput { id: String, stdout: String },
    ProcessExited { id: String, code: i32 },
    SelectionChanged,
    EditorChanged,
    WorkspaceChanged,
    TerminalStarted,
    TerminalExited,
    FileSystemReadComplete { req_id: String, content: Option<String>, error: Option<String> },
    FileSystemWriteComplete { req_id: String, error: Option<String> },
}
