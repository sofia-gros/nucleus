//! 言語サーバーとの連携（docs/ARCHITECTURE.md §7）。
//! どのサーバーをどう起動するかは Extension（`language.servers`）が宣言し、ここで起動・同期・配布する。
//! サーバーの起動と要求の待ち合わせはバックグラウンドで行い、UI スレッドは止めない。

use std::{
    cell::Cell,
    collections::{BTreeMap, HashMap, HashSet},
    path::{Path, PathBuf},
    rc::Rc,
    sync::Arc,
    time::Duration,
};

use anyhow::Result;
use editor::EditorPanel;
use futures::StreamExt as _;
use gpui_kit::{
    App, AppContext as _, Context, Entity, Task, WeakEntity, Window,
    component::{
        WindowExt as _,
        input::{CompletionProvider, DefinitionProvider, HoverProvider, Rope, RopeExt as _},
        notification::Notification,
    },
};
use i18n::Translate as _;
use lsp::{LanguageServer, ServerConfig, ServerNotification};
use lsp_types::{
    CompletionContext, CompletionParams, CompletionResponse, GotoDefinitionParams, GotoDefinitionResponse, HoverParams,
    LocationLink, Position, TextDocumentIdentifier, TextDocumentPositionParams, Uri,
    notification::{DidChangeTextDocument, DidCloseTextDocument, DidOpenTextDocument, DidSaveTextDocument},
    request::{Completion, GotoDefinition, HoverRequest},
};

use crate::Workspace;

enum Slot {
    Starting(Vec<WeakEntity<EditorPanel>>),
    Running(Arc<LanguageServer>),
    Failed,
}

/// 言語サーバーの状態（ステータスバー表示用）。
#[derive(Clone, Debug, PartialEq)]
pub enum ServerStatus {
    Starting,
    Ready,
    /// 進捗（インデックス作成中など）。
    Busy(String),
    Failed,
}

/// 1つの文書と言語サーバーの同期状態。UI スレッドだけで使う。
pub(crate) struct DocSync {
    server: Arc<LanguageServer>,
    uri: Uri,
    editor_version: Rc<Cell<u64>>,
    synced: Cell<u64>,
    lsp_version: Cell<i32>,
}

impl DocSync {
    /// 編集があれば全文を送る（要求の直前にも呼んで、最新の内容で答えてもらう）。
    fn sync(&self, text: &Rope) {
        let v = self.editor_version.get();
        if v == self.synced.get() {
            return;
        }
        self.synced.set(v);
        self.lsp_version.set(self.lsp_version.get() + 1);
        self.server
            .notify::<DidChangeTextDocument>(lsp_types::DidChangeTextDocumentParams {
                text_document: lsp_types::VersionedTextDocumentIdentifier {
                    uri: self.uri.clone(),
                    version: self.lsp_version.get(),
                },
                content_changes: vec![lsp_types::TextDocumentContentChangeEvent {
                    range: None,
                    range_length: None,
                    text: text.to_string(),
                }],
            });
    }

    fn position(&self, text: &Rope, offset: usize) -> Position {
        let p = text.offset_to_position(offset);
        if self.server.uses_utf32() {
            p
        } else {
            utf32_to_utf16(text, p)
        }
    }

    fn text_position(&self, text: &Rope, offset: usize) -> TextDocumentPositionParams {
        TextDocumentPositionParams {
            text_document: TextDocumentIdentifier { uri: self.uri.clone() },
            position: self.position(text, offset),
        }
    }
}

/// 言語サーバーの位置を、テキストのバイト位置にする。
fn lsp_offset(text: &Rope, p: Position, utf32: bool) -> usize {
    if utf32 {
        return text.position_to_offset(&p);
    }
    let line = text.slice_line(p.line as usize);
    let mut units = 0usize;
    let mut chars = 0u32;
    for c in line.chars() {
        if units >= p.character as usize {
            break;
        }
        units += c.len_utf16();
        chars += 1;
    }
    text.position_to_offset(&Position::new(p.line, chars))
}

/// コードポイント単位の位置を UTF-16 単位にする（UTF-16 しか話せないサーバー向け）。
fn utf32_to_utf16(text: &Rope, p: Position) -> Position {
    let line = text.slice_line(p.line as usize);
    let units: usize = line.chars().take(p.character as usize).map(char::len_utf16).sum();
    Position::new(p.line, units as u32)
}

#[derive(Default)]
pub(crate) struct LanguageSupport {
    servers: HashMap<(String, PathBuf), Slot>,
    docs: HashMap<PathBuf, Rc<DocSync>>,
    pub diagnostics: BTreeMap<PathBuf, Vec<lsp_types::Diagnostic>>,
    pub status: BTreeMap<String, ServerStatus>,
    reported_failures: HashSet<String>,
    /// 出した言語サーバーのメッセージ（同じものを何度も出さない）。
    shown_messages: HashSet<String>,
    sync_task: Option<Task<()>>,
}

impl LanguageSupport {
    pub fn counts(&self) -> (usize, usize) {
        let mut errors = 0;
        let mut warnings = 0;
        for d in self.diagnostics.values().flatten() {
            match d.severity {
                Some(lsp_types::DiagnosticSeverity::ERROR) | None => errors += 1,
                Some(lsp_types::DiagnosticSeverity::WARNING) => warnings += 1,
                _ => {}
            }
        }
        (errors, warnings)
    }
}

/// 言語に対応するサーバーの設定（有効な Extension の宣言から探す）。
fn server_for(language: &str, cx: &App) -> Option<ServerConfig> {
    let store = extension_host::ExtensionStore::get(cx)?;
    store.enabled().find_map(|e| {
        e.manifest
            .contributes
            .language_servers
            .iter()
            .find(|s| s.language == language)
            .map(|s| ServerConfig {
                name: s.command.clone(),
                command: s.command.clone(),
                args: s.args.clone(),
                initialization_options: s.initialization_options.clone(),
            })
    })
}

impl Workspace {
    /// エディタを開いたとき: 言語サーバーがあれば起動（または既存のものに接続）する。
    pub(crate) fn lsp_attach(&mut self, editor: Entity<EditorPanel>, window: &mut Window, cx: &mut Context<Self>) {
        let language = editor.read(cx).language().to_string();
        let Some(config) = server_for(&language, cx) else {
            return;
        };
        let path = editor.read(cx).path().to_path_buf();
        let root = self
            .project
            .read(cx)
            .root()
            .unwrap_or_else(|| path.parent().map(Path::to_path_buf).unwrap_or_default());
        let key = (config.name.clone(), root.clone());
        match self.lsp.servers.get_mut(&key) {
            Some(Slot::Running(server)) => {
                let server = server.clone();
                self.lsp_open_doc(server, editor, window, cx);
            }
            Some(Slot::Starting(waiting)) => waiting.push(editor.downgrade()),
            Some(Slot::Failed) => {}
            None => {
                self.lsp
                    .servers
                    .insert(key.clone(), Slot::Starting(vec![editor.downgrade()]));
                self.lsp.status.insert(config.name.clone(), ServerStatus::Starting);
                let (tx, mut rx) = futures::channel::mpsc::unbounded::<ServerNotification>();
                let name = config.name.clone();
                let start = cx.background_spawn(LanguageServer::start(config, root, tx));
                // サーバーからの通知を受け取り続ける。
                cx.spawn_in(window, {
                    let name = name.clone();
                    async move |this, cx| {
                        while let Some(n) = rx.next().await {
                            if this
                                .update_in(cx, |this, window, cx| this.lsp_notification(&name, n, window, cx))
                                .is_err()
                            {
                                break;
                            }
                        }
                    }
                })
                .detach();
                cx.spawn_in(window, async move |this, cx| {
                    let result = start.await;
                    this.update_in(cx, |this, window, cx| {
                        let waiting = match this.lsp.servers.remove(&key) {
                            Some(Slot::Starting(w)) => w,
                            _ => Vec::new(),
                        };
                        match result {
                            Ok(server) => {
                                log::info!("言語サーバー `{name}` を起動した");
                                this.lsp.servers.insert(key, Slot::Running(server.clone()));
                                this.lsp.status.insert(name, ServerStatus::Ready);
                                for e in waiting.into_iter().filter_map(|w| w.upgrade()) {
                                    this.lsp_open_doc(server.clone(), e, window, cx);
                                }
                            }
                            Err(e) => {
                                log::error!("言語サーバー `{name}` を起動できなかった: {e:#}");
                                this.lsp.servers.insert(key, Slot::Failed);
                                this.lsp.status.insert(name.clone(), ServerStatus::Failed);
                                if this.lsp.reported_failures.insert(name.clone()) {
                                    let msg = cx.t_args(
                                        "core.notify.lsp-failed",
                                        &[("server", name.into()), ("error", format!("{e:#}").into())],
                                    );
                                    window.push_notification(Notification::warning(msg), cx);
                                }
                            }
                        }
                        cx.notify();
                    })
                    .ok();
                })
                .detach();
            }
        }
        cx.notify();
    }

    fn lsp_open_doc(
        &mut self,
        server: Arc<LanguageServer>,
        editor: Entity<EditorPanel>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let path = editor.read(cx).path().to_path_buf();
        let Ok(uri) = lsp::path_to_uri(&path) else { return };
        let text = editor.read(cx).state().read(cx).text().clone();
        let doc = Rc::new(DocSync {
            server: server.clone(),
            uri: uri.clone(),
            editor_version: editor.read(cx).edit_version(),
            synced: Cell::new(editor.read(cx).edit_version().get()),
            lsp_version: Cell::new(1),
        });
        server.notify::<DidOpenTextDocument>(lsp_types::DidOpenTextDocumentParams {
            text_document: lsp_types::TextDocumentItem {
                uri,
                language_id: editor.read(cx).language().to_string(),
                version: 1,
                text: text.to_string(),
            },
        });
        self.lsp.docs.insert(path.clone(), doc.clone());

        // 言語機能の連携口を、エディタ部品（gpui-kit）に登録する。
        let caps = &server.capabilities;
        let triggers: Vec<String> = caps
            .completion_provider
            .as_ref()
            .and_then(|c| c.trigger_characters.clone())
            .unwrap_or_default();
        let has_completion = caps.completion_provider.is_some();
        let has_hover = caps.hover_provider.is_some();
        let has_definition = caps.definition_provider.is_some();
        let ws = cx.entity().downgrade();
        let this_path = path.clone();
        editor.update(cx, |e, cx| {
            e.state().update(cx, |s, cx| {
                let lsp = s.lsp_mut();
                if has_completion {
                    lsp.completion_provider = Some(Rc::new(LspCompletion {
                        doc: doc.clone(),
                        triggers,
                    }));
                }
                if has_hover {
                    lsp.hover_provider = Some(Rc::new(LspHover { doc: doc.clone() }));
                }
                if has_definition {
                    lsp.definition_provider = Some(Rc::new(LspDefinition { doc: doc.clone() }));
                    lsp.show_document = Some(Rc::new(move |params, window, cx| {
                        // 別のファイルへのジャンプは Workspace が開く。同じファイルなら gpui-kit が移動する。
                        let Some(target) = lsp::uri_to_path(&params.uri) else {
                            return false;
                        };
                        if target == this_path {
                            return false;
                        }
                        let pos = params.selection.map(|r| r.start).unwrap_or_default();
                        ws.update(cx, |ws, cx| ws.open_file_at(target, Some(pos), window, cx))
                            .is_ok()
                    }));
                }
                cx.notify();
            });
        });
        // すでに届いている診断を表示する。
        if let Some(d) = self.lsp.diagnostics.get(&path).cloned() {
            editor.update(cx, |e, cx| e.set_diagnostics(&d, cx));
        }
        let _ = window;
    }

    /// 編集されたら、少し待ってから（まとめて）全文を送る。
    pub(crate) fn lsp_edited(&mut self, cx: &mut Context<Self>) {
        if self.lsp.sync_task.is_some() || self.lsp.docs.is_empty() {
            return;
        }
        self.lsp.sync_task = Some(cx.spawn(async move |this, cx| {
            cx.background_executor().timer(Duration::from_millis(150)).await;
            this.update(cx, |this, cx| {
                this.lsp.sync_task = None;
                for (path, doc) in this.lsp.docs.clone() {
                    if let Some(editor) = this.editors.get(&path).and_then(WeakEntity::upgrade) {
                        let text = editor.read(cx).state().read(cx).text().clone();
                        doc.sync(&text);
                    }
                }
            })
            .ok();
        }));
    }

    /// 保存の前の整形（`editor.format_on_save`）。言語サーバーが整形できなければ None。
    #[allow(clippy::type_complexity)]
    pub(crate) fn lsp_format(
        &self,
        path: &Path,
        cx: &App,
    ) -> Option<Task<Result<Vec<(std::ops::Range<usize>, String)>>>> {
        let doc = self.lsp.docs.get(path)?.clone();
        let can_format = match &doc.server.capabilities.document_formatting_provider {
            Some(lsp_types::OneOf::Left(b)) => *b,
            Some(lsp_types::OneOf::Right(_)) => true,
            None => false,
        };
        if !can_format {
            return None;
        }
        let editor = self.editors.get(path)?.upgrade()?;
        let rope = editor.read(cx).state().read(cx).text().clone();
        doc.sync(&rope);
        let s = settings::ActiveSettings::get(cx);
        let params = lsp_types::DocumentFormattingParams {
            text_document: TextDocumentIdentifier { uri: doc.uri.clone() },
            options: lsp_types::FormattingOptions {
                tab_size: s.editor.tab_size as u32,
                insert_spaces: !s.editor.hard_tabs,
                ..Default::default()
            },
            work_done_progress_params: Default::default(),
        };
        let server = doc.server.clone();
        let utf32 = server.uses_utf32();
        Some(cx.background_spawn(async move {
            let edits = server
                .request::<lsp_types::request::Formatting>(params)
                .await?
                .unwrap_or_default();
            let mut out: Vec<_> = edits
                .into_iter()
                .map(|e| {
                    let start = lsp_offset(&rope, e.range.start, utf32);
                    let end = lsp_offset(&rope, e.range.end, utf32);
                    (start..end, e.new_text)
                })
                .collect();
            out.sort_by_key(|e| std::cmp::Reverse(e.0.start));
            Ok(out)
        }))
    }

    pub(crate) fn lsp_saved(&mut self, path: &Path) {
        if let Some(doc) = self.lsp.docs.get(path) {
            doc.server
                .notify::<DidSaveTextDocument>(lsp_types::DidSaveTextDocumentParams {
                    text_document: TextDocumentIdentifier { uri: doc.uri.clone() },
                    text: None,
                });
        }
    }

    pub(crate) fn lsp_closed(&mut self, path: &Path) {
        if let Some(doc) = self.lsp.docs.remove(path) {
            doc.server
                .notify::<DidCloseTextDocument>(lsp_types::DidCloseTextDocumentParams {
                    text_document: TextDocumentIdentifier { uri: doc.uri.clone() },
                });
        }
    }

    fn lsp_notification(&mut self, server: &str, n: ServerNotification, window: &mut Window, cx: &mut Context<Self>) {
        match n.method.as_str() {
            "textDocument/publishDiagnostics" => {
                let Ok(p) = serde_json::from_value::<lsp_types::PublishDiagnosticsParams>(n.params) else {
                    return;
                };
                let Some(path) = lsp::uri_to_path(&p.uri) else { return };
                if let Some(editor) = self.editors.get(&path).and_then(WeakEntity::upgrade) {
                    editor.update(cx, |e, cx| e.set_diagnostics(&p.diagnostics, cx));
                }
                if p.diagnostics.is_empty() {
                    self.lsp.diagnostics.remove(&path);
                } else {
                    self.lsp.diagnostics.insert(path, p.diagnostics);
                }
                self.refresh_problems(cx);
            }
            "window/showMessage" => {
                let Ok(p) = serde_json::from_value::<lsp_types::ShowMessageParams>(n.params) else {
                    return;
                };
                let text = format!("{server}: {}", p.message);
                // サーバーは同じ知らせを何度も送ってくることがある（ファイルを開くたびなど）。1回だけ出す。
                if !self.lsp.shown_messages.insert(text.clone()) {
                    return;
                }
                let note = match p.typ {
                    lsp_types::MessageType::ERROR => Notification::error(text),
                    lsp_types::MessageType::WARNING => Notification::warning(text),
                    _ => Notification::info(text),
                };
                window.push_notification(note, cx);
            }
            "$/progress" => {
                let Ok(p) = serde_json::from_value::<lsp_types::ProgressParams>(n.params) else {
                    return;
                };
                let lsp_types::ProgressParamsValue::WorkDone(w) = p.value;
                let status = match w {
                    lsp_types::WorkDoneProgress::Begin(b) => ServerStatus::Busy(b.title),
                    lsp_types::WorkDoneProgress::Report(r) => match r.message {
                        Some(m) => ServerStatus::Busy(m),
                        None => return,
                    },
                    lsp_types::WorkDoneProgress::End(_) => ServerStatus::Ready,
                };
                self.lsp.status.insert(server.to_string(), status);
            }
            _ => return,
        }
        cx.notify();
    }
}

struct LspCompletion {
    doc: Rc<DocSync>,
    triggers: Vec<String>,
}

impl CompletionProvider for LspCompletion {
    fn completions(
        &self,
        text: &Rope,
        offset: usize,
        trigger: CompletionContext,
        _: &mut Window,
        cx: &mut App,
    ) -> Task<Result<CompletionResponse>> {
        self.doc.sync(text);
        let params = CompletionParams {
            text_document_position: self.doc.text_position(text, offset),
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
            context: Some(trigger),
        };
        let server = self.doc.server.clone();
        cx.background_spawn(async move {
            Ok(server
                .request::<Completion>(params)
                .await?
                .unwrap_or(CompletionResponse::Array(Vec::new())))
        })
    }

    fn is_completion_trigger(&self, _: usize, new_text: &str, _: &mut App) -> bool {
        let Some(c) = new_text.chars().last() else { return false };
        c.is_alphanumeric() || c == '_' || self.triggers.iter().any(|t| new_text.ends_with(t.as_str()))
    }
}

struct LspHover {
    doc: Rc<DocSync>,
}

impl HoverProvider for LspHover {
    fn hover(
        &self,
        text: &Rope,
        offset: usize,
        _: &mut Window,
        cx: &mut App,
    ) -> Task<Result<Option<lsp_types::Hover>>> {
        if !settings::ActiveSettings::get(cx).editor.hover {
            return Task::ready(Ok(None));
        }
        self.doc.sync(text);
        let params = HoverParams {
            text_document_position_params: self.doc.text_position(text, offset),
            work_done_progress_params: Default::default(),
        };
        let server = self.doc.server.clone();
        cx.background_spawn(async move { server.request::<HoverRequest>(params).await })
    }
}

struct LspDefinition {
    doc: Rc<DocSync>,
}

impl DefinitionProvider for LspDefinition {
    fn definitions(&self, text: &Rope, offset: usize, _: &mut Window, cx: &mut App) -> Task<Result<Vec<LocationLink>>> {
        self.doc.sync(text);
        let params = GotoDefinitionParams {
            text_document_position_params: self.doc.text_position(text, offset),
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
        };
        let server = self.doc.server.clone();
        cx.background_spawn(async move {
            let links = match server.request::<GotoDefinition>(params).await? {
                None => Vec::new(),
                Some(GotoDefinitionResponse::Link(l)) => l,
                Some(GotoDefinitionResponse::Scalar(loc)) => vec![to_link(loc)],
                Some(GotoDefinitionResponse::Array(locs)) => locs.into_iter().map(to_link).collect(),
            };
            Ok(links)
        })
    }
}

fn to_link(loc: lsp_types::Location) -> LocationLink {
    LocationLink {
        origin_selection_range: None,
        target_uri: loc.uri,
        target_range: loc.range,
        target_selection_range: loc.range,
    }
}
