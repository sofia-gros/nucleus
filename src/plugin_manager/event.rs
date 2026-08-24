pub enum PluginEvent {
    FileOpened { path: String },
    FileSaved { path: String },
    SelectionChanged,
    EditorChanged,
    WorkspaceChanged,
    TerminalStarted,
    TerminalExited,
}
