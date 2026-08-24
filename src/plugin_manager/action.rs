pub enum PluginAction {
    OpenFile { path: String },
    OpenTab { title: String, content: String },
    CloseTab { title: String },
    FocusEditor,
    ShowNotification { message: String },
    OpenPanel { id: String },
    ClosePanel { id: String },
    SetStatusBarItem { id: String, text: String },
}
