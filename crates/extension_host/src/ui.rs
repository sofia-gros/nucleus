//! ui@1 の型（Extension が返す要素の木など）。WIT（`crates/extension_api/wit/nucleus.wit` の `ui`）と同じ形。
//!
//! WASM を動かすホストは wasmtime の型を使い、ここの型に移して画面へ送る（runtime.rs の `convert`）。
//! 画面側（ブラウザ版を含む）は wasmtime なしで、ここの型だけを使う。

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Tone {
    Default,
    Muted,
    Accent,
    Success,
    Warning,
    Danger,
    Info,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Size {
    None,
    Xs,
    Sm,
    Md,
    Lg,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Direction {
    Horizontal,
    Vertical,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ButtonVariant {
    Primary,
    Secondary,
    Ghost,
    Danger,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct StackProps {
    pub direction: Direction,
    pub gap: Size,
    pub padding: Size,
    pub grow: bool,
    pub spread: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TextProps {
    pub content: String,
    pub tone: Tone,
    pub size: Size,
    pub bold: bool,
    pub mono: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct IconProps {
    pub name: String,
    pub tone: Tone,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ButtonProps {
    pub key: String,
    pub label: Option<String>,
    pub icon: Option<String>,
    pub style: ButtonVariant,
    pub disabled: bool,
    pub tooltip: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct InputProps {
    pub key: String,
    pub placeholder: String,
    pub value: String,
    pub revision: u32,
    pub multiline: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ListItemProps {
    pub key: String,
    pub label: String,
    pub detail: Option<String>,
    pub icon: Option<String>,
    pub icon_tone: Tone,
    pub badge: Option<String>,
    pub badge_tone: Tone,
    pub selected: bool,
    pub actions: Vec<ButtonProps>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DrawRect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub color: String,
    pub alpha: f32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CanvasProps {
    pub key: String,
    pub width: f32,
    pub rects: Vec<DrawRect>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SectionProps {
    pub key: String,
    pub title: String,
    pub count: Option<u32>,
    pub collapsed: bool,
    pub actions: Vec<ButtonProps>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Kind {
    Stack(StackProps),
    Scroll,
    Text(TextProps),
    Icon(IconProps),
    Button(ButtonProps),
    Input(InputProps),
    ListItem(ListItemProps),
    Section(SectionProps),
    Divider,
    Skeleton,
    Canvas(CanvasProps),
}

/// 木は平らな配列で表す。0 番が根、`children` は配列の添字。
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Node {
    pub kind: Kind,
    pub children: Vec<u32>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TextSpan {
    pub text: String,
    pub color: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TextLine {
    pub x: f32,
    pub y: f32,
    pub size: f32,
    pub alpha: f32,
    pub spans: Vec<TextSpan>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum PointerKind {
    Down,
    Drag,
    Up,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct PointerEvent {
    pub kind: PointerKind,
    pub x: f32,
    pub y: f32,
    pub height: f32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum UiEvent {
    Click,
    Submit(String),
    Change(String),
    Toggle(bool),
    Pointer(PointerEvent),
}

/// ワークスペースで起きたこと（WIT の `events.workspace-event`）。
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum WorkspaceEvent {
    RootChanged(Option<String>),
    FileSaved(String),
    WindowFocused,
}

/// 要素の木の上限。
const MAX_NODES: usize = 10_000;

/// 要素の木が正しいか（添字が範囲内、親が1つ、循環しない）。
pub fn validate_tree(nodes: &[Node]) -> Result<(), String> {
    if nodes.is_empty() {
        return Err("要素の木が空".into());
    }
    if nodes.len() > MAX_NODES {
        return Err(format!("要素が多すぎる（{MAX_NODES} まで）"));
    }
    let mut seen = vec![false; nodes.len()];
    seen[0] = true;
    for (i, n) in nodes.iter().enumerate() {
        for &c in &n.children {
            let c = c as usize;
            if c >= nodes.len() || c <= i || seen[c] {
                return Err(format!("要素 {i} の子 {c} が正しくない"));
            }
            seen[c] = true;
        }
    }
    Ok(())
}
