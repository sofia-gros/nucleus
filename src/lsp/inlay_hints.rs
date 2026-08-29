/// LSP インレイヒントおよびシグネチャヘルプのデータモデル

/// 単一のインレイヒント情報
#[derive(Clone, Debug, PartialEq)]
pub struct InlayHintItem {
    pub line: usize,
    pub column: usize,
    pub label: String,
    pub kind: InlayHintKind,
}

/// インレイヒントの種類
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InlayHintKind {
    Type,
    Parameter,
}

/// シグネチャヘルプ情報
#[derive(Clone, Debug, PartialEq, Default)]
pub struct SignatureHelpState {
    pub is_visible: bool,
    pub label: String,
    pub doc: Option<String>,
    pub active_parameter: usize,
    pub parameters: Vec<String>,
}

impl SignatureHelpState {
    pub fn new() -> Self {
        Self::default()
    }
}
