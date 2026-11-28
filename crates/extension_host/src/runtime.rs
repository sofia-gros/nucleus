//! 段階3: WASM（Component Model）の Extension を動かす（docs/ARCHITECTURE.md §4.3, §5）。
//!
//! - Extension ごとに専用のスレッドを1本持ち、その上で wasmtime の Store を動かす。UI スレッドは止めない。
//! - ホスト関数は manifest の権限を毎回確かめる。権限がなければエラーを返す（WASM 側で扱える）。
//! - エディタや通知など UI に触る関数は [`UiRequest`] を UI スレッドに送り、返事を待つ。
//! - 1回の呼び出しは [`CALL_TIMEOUT`] まで、メモリは [`MEMORY_LIMIT`] まで。超えたら打ち切る。
//! - WASI は p2。ファイル・ネットワーク・環境変数は渡さない（標準出力・標準エラーはログに流す）。

use std::{
    collections::HashMap,
    path::{Component, Path, PathBuf},
    sync::{
        Arc, OnceLock, RwLock,
        mpsc::{self, Receiver, Sender},
    },
    time::{Duration, Instant},
};

use futures::channel::{mpsc as async_mpsc, oneshot};
use wasmtime::{
    Config, Engine, Store, StoreLimits, StoreLimitsBuilder,
    component::{Component as WasmComponent, HasSelf, Linker, ResourceTable},
};
use wasmtime_wasi::{WasiCtx, WasiCtxBuilder, WasiCtxView, WasiView};

use crate::manifest::Manifest;

mod bindings {
    wasmtime::component::bindgen!({
        path: "../extension_api/wit",
        world: "extension",
    });
}

use bindings::nucleus::extension::host::{self, Document, LogLevel, NotificationLevel, ProcessOutput};
/// WIT が作った ui@1 の型（ホストの中だけで使い、外へは crate::ui の型にして渡す）。
use bindings::nucleus::extension::{events as wit_events, ui as wit_ui};

pub use crate::api::{ActiveDocument, EditorViewInfo, FileState, NoticeLevel, UiRequest};
pub use crate::ui::{self, WorkspaceEvent};

/// 1回の呼び出し（activate / run-command）に使える時間。
pub const CALL_TIMEOUT: Duration = Duration::from_secs(5);
/// 1つの Extension が使えるメモリ。
pub const MEMORY_LIMIT: usize = 64 << 20;
/// epoch を進める間隔。
const TICK: Duration = Duration::from_millis(50);
/// `run-process` の出力の上限（それぞれ）。
const PROCESS_OUTPUT_LIMIT: usize = 4 << 20;

/// UI 側とのつながり。ExtensionStore が1つ持つ。
#[derive(Clone)]
pub struct HostContext {
    ui: async_mpsc::UnboundedSender<UiRequest>,
    root: Arc<RwLock<Option<PathBuf>>>,
    /// 利用者が変えた Extension の設定（Extension の ID → キー → 値）。
    settings: Arc<RwLock<ExtensionSettings>>,
}

/// Extension の ID → 設定のキー → 値。
pub type ExtensionSettings = std::collections::BTreeMap<String, serde_json::Map<String, serde_json::Value>>;

/// `render` の結果: 要素の木と、canvas に足す文字（canvas の key ごと）。
#[derive(Clone, Debug, Default)]
pub struct Rendered {
    pub nodes: Vec<ui::Node>,
    pub texts: HashMap<String, Vec<ui::TextLine>>,
}

impl HostContext {
    pub fn new() -> (HostContext, async_mpsc::UnboundedReceiver<UiRequest>) {
        let (tx, rx) = async_mpsc::unbounded();
        (
            HostContext {
                ui: tx,
                root: Arc::new(RwLock::new(None)),
                settings: Arc::default(),
            },
            rx,
        )
    }

    /// ワークスペースのルート（ファイル関数の基準）を設定する。
    pub fn set_workspace_root(&self, root: Option<PathBuf>) {
        if let Ok(mut r) = self.root.write() {
            *r = root;
        }
    }

    /// 利用者の設定を入れ替える。値が変わったら true。
    pub fn set_extension_settings(&self, settings: ExtensionSettings) -> bool {
        match self.settings.write() {
            Ok(mut s) if *s != settings => {
                *s = settings;
                true
            }
            _ => false,
        }
    }

    fn setting(&self, extension: &str, key: &str) -> Option<serde_json::Value> {
        self.settings.read().ok()?.get(extension)?.get(key).cloned()
    }

    fn root(&self) -> Option<PathBuf> {
        self.root.read().ok().and_then(|r| r.clone())
    }
}

enum Call {
    RunCommand {
        id: String,
        reply: oneshot::Sender<Result<(), String>>,
    },
    Render {
        view: String,
        reply: oneshot::Sender<Result<Rendered, String>>,
    },
    UiEvent {
        view: String,
        key: String,
        event: ui::UiEvent,
        reply: oneshot::Sender<Result<(), String>>,
    },
    Workspace(WorkspaceEvent),
}

/// 起動した WASM Extension。落としたらスレッドも終わる。
pub struct WasmExtension {
    pub id: String,
    calls: Sender<Call>,
}

impl WasmExtension {
    /// コマンドを実行する。結果は WASM 側の戻り値か、打ち切りなどのエラー。
    pub fn run_command(&self, id: &str) -> oneshot::Receiver<Result<(), String>> {
        let (reply, rx) = oneshot::channel();
        if let Err(mpsc::SendError(Call::RunCommand { reply, .. })) = self.calls.send(Call::RunCommand {
            id: id.to_string(),
            reply,
        }) {
            let _ = reply.send(Err("Extension は止まっている".into()));
        }
        rx
    }

    /// ビューの中身を描く（要素の木を返す）。
    pub fn render(&self, view: &str) -> oneshot::Receiver<Result<Rendered, String>> {
        let (reply, rx) = oneshot::channel();
        if let Err(mpsc::SendError(Call::Render { reply, .. })) = self.calls.send(Call::Render {
            view: view.to_string(),
            reply,
        }) {
            let _ = reply.send(Err("Extension は止まっている".into()));
        }
        rx
    }

    /// ビューの中のイベントを渡す。
    pub fn ui_event(&self, view: &str, key: &str, event: ui::UiEvent) -> oneshot::Receiver<Result<(), String>> {
        let (reply, rx) = oneshot::channel();
        if let Err(mpsc::SendError(Call::UiEvent { reply, .. })) = self.calls.send(Call::UiEvent {
            view: view.to_string(),
            key: key.to_string(),
            event,
            reply,
        }) {
            let _ = reply.send(Err("Extension は止まっている".into()));
        }
        rx
    }

    /// ワークスペースで起きたことを知らせる（返事は待たない）。
    pub fn workspace_event(&self, event: WorkspaceEvent) {
        let _ = self.calls.send(Call::Workspace(event));
    }
}

fn engine() -> Result<&'static Engine, String> {
    static ENGINE: OnceLock<Result<Engine, String>> = OnceLock::new();
    ENGINE
        .get_or_init(|| {
            let mut config = Config::new();
            config.epoch_interruption(true);
            let engine = Engine::new(&config).map_err(|e| e.to_string())?;
            let ticker = engine.clone();
            std::thread::Builder::new()
                .name("wasm-epoch".into())
                .spawn(move || {
                    loop {
                        std::thread::sleep(TICK);
                        ticker.increment_epoch();
                    }
                })
                .map_err(|e| e.to_string())?;
            Ok(engine)
        })
        .as_ref()
        .map_err(Clone::clone)
}

fn deadline_ticks() -> u64 {
    (CALL_TIMEOUT.as_millis() / TICK.as_millis()) as u64
}

/// Extension を起動する（コンパイルと activate をそのスレッドで行う）。
/// 結果（activate の成否）は戻り値の Receiver に届く。
pub fn spawn(
    manifest: Manifest,
    wasm: Arc<[u8]>,
    host: HostContext,
) -> (WasmExtension, oneshot::Receiver<Result<Duration, String>>) {
    let (calls_tx, calls_rx) = mpsc::channel();
    let (ready_tx, ready_rx) = oneshot::channel();
    let id = manifest.id.clone();
    let thread_name = format!("ext:{id}");
    let spawned = std::thread::Builder::new().name(thread_name).spawn(move || {
        let started = Instant::now();
        match Instance::new(manifest, &wasm, host) {
            Ok(mut inst) => {
                let r = inst.activate();
                let ok = r.is_ok();
                let _ = ready_tx.send(r.map(|_| started.elapsed()));
                if ok {
                    inst.serve(calls_rx);
                }
            }
            Err(e) => {
                let _ = ready_tx.send(Err(e));
            }
        }
    });
    if let Err(e) = spawned {
        log::error!("Extension `{id}` のスレッドを作れなかった: {e}");
    }
    (WasmExtension { id, calls: calls_tx }, ready_rx)
}

struct State {
    manifest: Manifest,
    host: HostContext,
    /// `render` の中で `canvas-text` で足された文字。
    canvas_texts: HashMap<String, Vec<ui::TextLine>>,
    wasi: WasiCtx,
    table: ResourceTable,
    limits: StoreLimits,
}

impl WasiView for State {
    fn ctx(&mut self) -> WasiCtxView<'_> {
        WasiCtxView {
            ctx: &mut self.wasi,
            table: &mut self.table,
        }
    }
}

struct Instance {
    store: Store<State>,
    bindings: bindings::Extension,
}

impl Instance {
    fn new(manifest: Manifest, wasm: &[u8], host: HostContext) -> Result<Instance, String> {
        let engine = engine()?;
        let component = WasmComponent::new(engine, wasm).map_err(|e| format!("WASM を読めない: {e}"))?;
        let mut linker: Linker<State> = Linker::new(engine);
        wasmtime_wasi::p2::add_to_linker_sync(&mut linker).map_err(|e| e.to_string())?;
        bindings::Extension::add_to_linker::<_, HasSelf<_>>(&mut linker, |s| s).map_err(|e| e.to_string())?;
        // 標準出力・標準エラーは捨てる（ログには `host.log` を使う）。ファイル・ネットワークは渡さない。
        let wasi = WasiCtxBuilder::new().build();
        let state = State {
            manifest,
            host,
            canvas_texts: HashMap::new(),
            wasi,
            table: ResourceTable::new(),
            limits: StoreLimitsBuilder::new()
                .memory_size(MEMORY_LIMIT)
                .instances(16)
                .tables(64)
                .memories(16)
                .build(),
        };
        let mut store = Store::new(engine, state);
        store.limiter(|s| &mut s.limits);
        store.set_epoch_deadline(deadline_ticks());
        let bindings = bindings::Extension::instantiate(&mut store, &component, &linker)
            .map_err(|e| format!("起動できない: {e}"))?;
        Ok(Instance { store, bindings })
    }

    fn activate(&mut self) -> Result<(), String> {
        self.store.set_epoch_deadline(deadline_ticks());
        match self.bindings.call_activate(&mut self.store) {
            Ok(r) => r,
            Err(e) => Err(trap_message(e)),
        }
    }

    fn serve(&mut self, calls: Receiver<Call>) {
        while let Ok(call) = calls.recv() {
            self.store.set_epoch_deadline(deadline_ticks());
            let (trap, invalidate) = match call {
                Call::RunCommand { id, reply } => {
                    let r = flat(self.bindings.call_run_command(&mut self.store, &id));
                    let trap = trapped(&r);
                    let _ = reply.send(r);
                    (trap, true)
                }
                Call::Render { view, reply } => {
                    self.store.data_mut().canvas_texts.clear();
                    let r = self
                        .bindings
                        .call_render(&mut self.store, &view)
                        .map_err(trap_message)
                        .map(|nodes| nodes.into_iter().map(convert::node).collect::<Vec<_>>())
                        .and_then(|nodes| validate_tree(&nodes).map(|_| nodes))
                        .map(|nodes| Rendered {
                            nodes,
                            texts: std::mem::take(&mut self.store.data_mut().canvas_texts),
                        });
                    let trap = trapped(&r);
                    let _ = reply.send(r);
                    (trap, false)
                }
                Call::UiEvent {
                    view,
                    key,
                    event,
                    reply,
                } => {
                    let event = convert::ui_event(event);
                    let r = flat(self.bindings.call_handle_ui_event(&mut self.store, &view, &key, &event));
                    let trap = trapped(&r);
                    let _ = reply.send(r);
                    (trap, true)
                }
                Call::Workspace(event) => {
                    let event = convert::workspace_event(event);
                    let r = self
                        .bindings
                        .call_on_event(&mut self.store, &event)
                        .map_err(trap_message);
                    if let Err(e) = &r {
                        log::warn!("Extension `{}`: {e}", self.store.data().manifest.id);
                    }
                    (trapped(&r), true)
                }
            };
            if trap {
                // 打ち切られた Instance は中身が壊れているかもしれないので、それ以上使わない。
                log::warn!("Extension `{}` を止めた", self.store.data().manifest.id);
                return;
            }
            if invalidate && !self.store.data().manifest.contributes.views.is_empty() {
                let data = self.store.data();
                let _ = data.host.ui.unbounded_send(UiRequest::Invalidate {
                    extension: data.manifest.id.clone(),
                });
            }
        }
    }
}

fn flat(r: wasmtime::Result<Result<(), String>>) -> Result<(), String> {
    match r {
        Ok(r) => r,
        Err(e) => Err(trap_message(e)),
    }
}

fn trapped<T>(r: &Result<T, String>) -> bool {
    r.as_ref().is_err_and(|e| e.starts_with(TRAP_PREFIX))
}

pub use crate::ui::validate_tree;

/// WIT の型と crate::ui の型の入れ替え。
mod convert {
    use super::{ui, wit_events, wit_ui};

    fn tone(t: wit_ui::Tone) -> ui::Tone {
        match t {
            wit_ui::Tone::Default => ui::Tone::Default,
            wit_ui::Tone::Muted => ui::Tone::Muted,
            wit_ui::Tone::Accent => ui::Tone::Accent,
            wit_ui::Tone::Success => ui::Tone::Success,
            wit_ui::Tone::Warning => ui::Tone::Warning,
            wit_ui::Tone::Danger => ui::Tone::Danger,
            wit_ui::Tone::Info => ui::Tone::Info,
        }
    }

    fn size(s: wit_ui::Size) -> ui::Size {
        match s {
            wit_ui::Size::None => ui::Size::None,
            wit_ui::Size::Xs => ui::Size::Xs,
            wit_ui::Size::Sm => ui::Size::Sm,
            wit_ui::Size::Md => ui::Size::Md,
            wit_ui::Size::Lg => ui::Size::Lg,
        }
    }

    fn button(b: wit_ui::ButtonProps) -> ui::ButtonProps {
        ui::ButtonProps {
            key: b.key,
            label: b.label,
            icon: b.icon,
            style: match b.style {
                wit_ui::ButtonVariant::Primary => ui::ButtonVariant::Primary,
                wit_ui::ButtonVariant::Secondary => ui::ButtonVariant::Secondary,
                wit_ui::ButtonVariant::Ghost => ui::ButtonVariant::Ghost,
                wit_ui::ButtonVariant::Danger => ui::ButtonVariant::Danger,
            },
            disabled: b.disabled,
            tooltip: b.tooltip,
        }
    }

    pub fn node(n: wit_ui::Node) -> ui::Node {
        use wit_ui::Kind as W;
        let kind = match n.kind {
            W::Stack(p) => ui::Kind::Stack(ui::StackProps {
                direction: match p.direction {
                    wit_ui::Direction::Horizontal => ui::Direction::Horizontal,
                    wit_ui::Direction::Vertical => ui::Direction::Vertical,
                },
                gap: size(p.gap),
                padding: size(p.padding),
                grow: p.grow,
                spread: p.spread,
            }),
            W::Scroll => ui::Kind::Scroll,
            W::Text(p) => ui::Kind::Text(ui::TextProps {
                content: p.content,
                tone: tone(p.tone),
                size: size(p.size),
                bold: p.bold,
                mono: p.mono,
            }),
            W::Icon(p) => ui::Kind::Icon(ui::IconProps {
                name: p.name,
                tone: tone(p.tone),
            }),
            W::Button(p) => ui::Kind::Button(button(p)),
            W::Input(p) => ui::Kind::Input(ui::InputProps {
                key: p.key,
                placeholder: p.placeholder,
                value: p.value,
                revision: p.revision,
                multiline: p.multiline,
            }),
            W::ListItem(p) => ui::Kind::ListItem(ui::ListItemProps {
                key: p.key,
                label: p.label,
                detail: p.detail,
                icon: p.icon,
                icon_tone: tone(p.icon_tone),
                badge: p.badge,
                badge_tone: tone(p.badge_tone),
                selected: p.selected,
                actions: p.actions.into_iter().map(button).collect(),
            }),
            W::Section(p) => ui::Kind::Section(ui::SectionProps {
                key: p.key,
                title: p.title,
                count: p.count,
                collapsed: p.collapsed,
                actions: p.actions.into_iter().map(button).collect(),
            }),
            W::Divider => ui::Kind::Divider,
            W::Skeleton => ui::Kind::Skeleton,
            W::Canvas(p) => ui::Kind::Canvas(ui::CanvasProps {
                key: p.key,
                width: p.width,
                rects: p
                    .rects
                    .into_iter()
                    .map(|r| ui::DrawRect {
                        x: r.x,
                        y: r.y,
                        width: r.width,
                        height: r.height,
                        color: r.color,
                        alpha: r.alpha,
                    })
                    .collect(),
            }),
        };
        ui::Node {
            kind,
            children: n.children,
        }
    }

    pub fn text_line(l: wit_ui::TextLine) -> ui::TextLine {
        ui::TextLine {
            x: l.x,
            y: l.y,
            size: l.size,
            alpha: l.alpha,
            spans: l
                .spans
                .into_iter()
                .map(|s| ui::TextSpan {
                    text: s.text,
                    color: s.color,
                })
                .collect(),
        }
    }

    pub fn ui_event(e: ui::UiEvent) -> wit_ui::UiEvent {
        match e {
            ui::UiEvent::Click => wit_ui::UiEvent::Click,
            ui::UiEvent::Submit(s) => wit_ui::UiEvent::Submit(s),
            ui::UiEvent::Change(s) => wit_ui::UiEvent::Change(s),
            ui::UiEvent::Toggle(b) => wit_ui::UiEvent::Toggle(b),
            ui::UiEvent::Pointer(p) => wit_ui::UiEvent::Pointer(wit_ui::PointerEvent {
                kind: match p.kind {
                    ui::PointerKind::Down => wit_ui::PointerKind::Down,
                    ui::PointerKind::Drag => wit_ui::PointerKind::Drag,
                    ui::PointerKind::Up => wit_ui::PointerKind::Up,
                },
                x: p.x,
                y: p.y,
                height: p.height,
            }),
        }
    }

    pub fn workspace_event(e: ui::WorkspaceEvent) -> wit_events::WorkspaceEvent {
        match e {
            ui::WorkspaceEvent::RootChanged(r) => wit_events::WorkspaceEvent::RootChanged(r),
            ui::WorkspaceEvent::FileSaved(p) => wit_events::WorkspaceEvent::FileSaved(p),
            ui::WorkspaceEvent::WindowFocused => wit_events::WorkspaceEvent::WindowFocused,
        }
    }
}

const TRAP_PREFIX: &str = "停止: ";

fn trap_message(e: wasmtime::Error) -> String {
    if let Some(trap) = e.downcast_ref::<wasmtime::Trap>() {
        match trap {
            wasmtime::Trap::Interrupt => {
                return format!("{TRAP_PREFIX}{} 秒以内に終わらなかった", CALL_TIMEOUT.as_secs());
            }
            other => return format!("{TRAP_PREFIX}{other}"),
        }
    }
    format!("{TRAP_PREFIX}{e:#}")
}

impl State {
    fn require(&self, category: &str, value: &str) -> Result<(), String> {
        if self.manifest.has(category, value) {
            Ok(())
        } else {
            Err(format!("`{category}: [{value}]` の権限がない"))
        }
    }

    fn ask<T>(&self, make: impl FnOnce(oneshot::Sender<T>) -> UiRequest) -> Result<T, String> {
        let (tx, rx) = oneshot::channel();
        self.host
            .ui
            .unbounded_send(make(tx))
            .map_err(|_| "UI が終わっている".to_string())?;
        futures::executor::block_on(rx).map_err(|_| "UI が応えなかった".to_string())
    }

    /// Extension が渡したパスを、ワークスペースの中の絶対パスにする。
    fn resolve(&self, path: &str) -> Result<PathBuf, String> {
        let root = self.host.root().ok_or("フォルダが開かれていない")?;
        confine(&root, path)
    }
}

/// `path` を `root` の中に閉じ込める（`..` や外への絶対パス、シンボリックリンクでの脱出を拒む）。
pub fn confine(root: &Path, path: &str) -> Result<PathBuf, String> {
    let p = Path::new(path);
    let joined = if p.is_absolute() { p.to_path_buf() } else { root.join(p) };
    let mut normal = PathBuf::new();
    for c in joined.components() {
        match c {
            Component::ParentDir => {
                if !normal.pop() {
                    return Err(format!("`{path}` はワークスペースの外"));
                }
            }
            Component::CurDir => {}
            other => normal.push(other),
        }
    }
    let root_real = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
    // 存在する一番深い親を実体に直して確かめる（リンクで外に出ていないか）。
    let mut probe = normal.clone();
    let real = loop {
        if let Ok(r) = probe.canonicalize() {
            break r;
        }
        if !probe.pop() {
            break normal.clone();
        }
    };
    if !normal.starts_with(root) && !normal.starts_with(&root_real) || !real.starts_with(&root_real) {
        return Err(format!("`{path}` はワークスペースの外"));
    }
    Ok(normal)
}

/// 1回の `render` で足せる文字の行の上限。
const MAX_TEXT_LINES: usize = 4000;

impl wit_ui::Host for State {
    fn canvas_text(&mut self, canvas_key: String, lines: Vec<wit_ui::TextLine>) {
        let lines: Vec<ui::TextLine> = lines.into_iter().map(convert::text_line).collect();
        let total: usize = self.canvas_texts.values().map(Vec::len).sum();
        let room = MAX_TEXT_LINES.saturating_sub(total);
        self.canvas_texts
            .entry(canvas_key)
            .or_default()
            .extend(lines.into_iter().take(room));
    }
}
impl bindings::nucleus::extension::events::Host for State {}

impl host::Host for State {
    fn log(&mut self, level: LogLevel, message: String) {
        let id = &self.manifest.id;
        match level {
            LogLevel::Debug => log::debug!("[{id}] {message}"),
            LogLevel::Info => log::info!("[{id}] {message}"),
            LogLevel::Warn => log::warn!("[{id}] {message}"),
            LogLevel::Error => log::error!("[{id}] {message}"),
        }
    }

    fn notify(&mut self, level: NotificationLevel, message: String) {
        let level = match level {
            NotificationLevel::Info => NoticeLevel::Info,
            NotificationLevel::Warning => NoticeLevel::Warning,
            NotificationLevel::Error => NoticeLevel::Error,
        };
        let _ = self.host.ui.unbounded_send(UiRequest::Notify {
            extension: self.manifest.name.clone(),
            level,
            message,
        });
    }

    fn workspace_root(&mut self) -> Option<String> {
        self.host.root().map(|p| p.display().to_string())
    }

    fn read_file(&mut self, path: String) -> Result<String, String> {
        self.require("workspace", "fs.read")?;
        let p = self.resolve(&path)?;
        std::fs::read_to_string(&p).map_err(|e| format!("`{path}`: {e}"))
    }

    fn write_file(&mut self, path: String, contents: String) -> Result<(), String> {
        self.require("workspace", "fs.write")?;
        let p = self.resolve(&path)?;
        std::fs::write(&p, contents).map_err(|e| format!("`{path}`: {e}"))
    }

    fn active_document(&mut self) -> Option<Document> {
        self.require("editor", "read").ok()?;
        let doc = self.ask(|reply| UiRequest::ActiveDocument { reply }).ok()??;
        Some(Document {
            path: doc.path,
            language: doc.language,
            text: doc.text,
        })
    }

    fn replace_active_text(&mut self, text: String) -> Result<(), String> {
        self.require("editor", "edit")?;
        self.ask(|reply| UiRequest::ReplaceActiveText { text, reply })?
    }

    fn open_file(&mut self, path: String, line: Option<u32>) -> Result<(), String> {
        let path = self.resolve(&path)?;
        if !path.is_file() {
            return Err(format!("`{}` がない", path.display()));
        }
        self.host
            .ui
            .unbounded_send(UiRequest::OpenFile { path, line })
            .map_err(|_| "UI が終わっている".to_string())
    }

    fn run_process_input(
        &mut self,
        command: String,
        args: Vec<String>,
        input: String,
    ) -> Result<ProcessOutput, String> {
        self.process(command, args, Some(input))
    }

    fn show_diff(&mut self, path: String, base: String, base_label: String) -> Result<(), String> {
        self.require("ui", "tab")?;
        let abs = self.resolve(&path)?;
        let root = self.host.root().unwrap_or_default();
        let rel = abs
            .strip_prefix(&root)
            .unwrap_or(&abs)
            .to_string_lossy()
            .replace('\\', "/");
        self.host
            .ui
            .unbounded_send(UiRequest::ShowDiff {
                extension: self.manifest.id.clone(),
                path: abs,
                rel,
                base,
                base_label,
            })
            .map_err(|_| "UI が終わっている".to_string())
    }

    fn editor_view(&mut self, editor: u32) -> Option<host::EditorState> {
        self.require("editor", "read").ok()?;
        let v = self.ask(|reply| UiRequest::EditorView { editor, reply }).ok()??;
        Some(host::EditorState {
            version: v.version,
            first_line: v.first_line,
            last_line: v.last_line,
            line_count: v.line_count,
            cursor_line: v.cursor_line,
            height: v.height,
        })
    }

    fn editor_text(&mut self, editor: u32) -> Option<String> {
        self.require("editor", "read").ok()?;
        self.ask(|reply| UiRequest::EditorText { editor, reply }).ok()?
    }

    fn scroll_editor(&mut self, editor: u32, line: u32) {
        if self.require("editor", "read").is_ok() {
            let _ = self.host.ui.unbounded_send(UiRequest::ScrollEditor { editor, line });
        }
    }

    fn run_process(&mut self, command: String, args: Vec<String>) -> Result<ProcessOutput, String> {
        self.process(command, args, None)
    }

    fn decorate_files(&mut self, marks: Vec<host::FileMark>) {
        if self.require("ui", "tree.decorate").is_err() {
            return;
        }
        let Some(root) = self.host.root() else { return };
        let marks = marks
            .into_iter()
            .take(20_000)
            .filter_map(|m| {
                let path = confine(&root, &m.path).ok()?;
                let state = match m.state {
                    host::FileState::Modified => FileState::Modified,
                    host::FileState::Added => FileState::Added,
                    host::FileState::Untracked => FileState::Untracked,
                    host::FileState::Deleted => FileState::Deleted,
                    host::FileState::Renamed => FileState::Renamed,
                    host::FileState::Conflicted => FileState::Conflicted,
                    host::FileState::Ignored => FileState::Ignored,
                };
                Some((path, state))
            })
            .collect();
        let _ = self.host.ui.unbounded_send(UiRequest::DecorateFiles {
            extension: self.manifest.id.clone(),
            marks,
        });
    }

    fn get_setting(&mut self, key: String) -> Option<String> {
        let decl = self.manifest.contributes.settings.iter().find(|s| s.key == key)?;
        let value = self
            .host
            .setting(&self.manifest.id, &key)
            .filter(|v| decl.accepts(v))
            .unwrap_or_else(|| decl.default.clone());
        serde_json::to_string(&value).ok()
    }
}

impl State {
    fn process(&mut self, command: String, args: Vec<String>, input: Option<String>) -> Result<ProcessOutput, String> {
        if command.contains('/') || command.contains('\\') {
            return Err("コマンドは名前で指定する（パス不可）".into());
        }
        self.require("process", &command)?;
        use std::process::Stdio;
        let mut cmd = background::command(&command);
        cmd.args(&args)
            .stdin(if input.is_some() { Stdio::piped() } else { Stdio::null() })
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        if let Some(root) = self.host.root() {
            cmd.current_dir(root);
        }
        cmd.env("GIT_TERMINAL_PROMPT", "0");
        let mut child = cmd.spawn().map_err(|e| format!("`{command}`: {e}"))?;
        // 出力の読み取りと詰まらないように、書き込みは別のスレッドで行う。
        let writer = match (input, child.stdin.take()) {
            (Some(input), Some(mut stdin)) => Some(std::thread::spawn(move || {
                use std::io::Write as _;
                let _ = stdin.write_all(input.as_bytes());
            })),
            _ => None,
        };
        let out = child.wait_with_output().map_err(|e| format!("`{command}`: {e}"))?;
        if let Some(w) = writer {
            let _ = w.join();
        }
        let text = |b: &[u8]| String::from_utf8_lossy(&b[..b.len().min(PROCESS_OUTPUT_LIMIT)]).into_owned();
        Ok(ProcessOutput {
            status: out.status.code().unwrap_or(-1),
            stdout: text(&out.stdout),
            stderr: text(&out.stderr),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node(children: Vec<u32>) -> ui::Node {
        ui::Node {
            kind: ui::Kind::Divider,
            children,
        }
    }

    #[test]
    fn trees_must_be_well_formed() {
        assert!(validate_tree(&[node(vec![1, 2]), node(vec![]), node(vec![])]).is_ok());
        assert!(validate_tree(&[]).is_err());
        // 範囲外・自分より前・2つの親
        assert!(validate_tree(&[node(vec![3])]).is_err());
        assert!(validate_tree(&[node(vec![1]), node(vec![0])]).is_err());
        assert!(validate_tree(&[node(vec![1, 2]), node(vec![2]), node(vec![])]).is_err());
    }

    #[test]
    fn paths_are_confined_to_the_workspace() {
        let d = tempfile::tempdir().unwrap();
        let root = d.path();
        std::fs::create_dir_all(root.join("src")).unwrap();
        assert_eq!(confine(root, "src/a.rs").unwrap(), root.join("src/a.rs"));
        assert_eq!(confine(root, "./src/../b.txt").unwrap(), root.join("b.txt"));
        assert!(confine(root, "../outside").is_err());
        assert!(confine(root, "/etc/passwd").is_err());
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink("/etc", root.join("link")).unwrap();
            assert!(confine(root, "link/passwd").is_err());
        }
    }
}
