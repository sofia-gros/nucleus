//! 起動後の段階的な反映（docs/ARCHITECTURE.md §5）。
//! 段階1（データ系の権限）と段階3（WASM の起動）をここで行う。段階2（UI）は P6 で加わる。

use std::{collections::HashMap, sync::Arc, time::Instant};

use gpui_kit::{App, AppContext as _, Global};
use theme::ExtTokenSet;

use crate::{
    loader::{self, FailedExtension, LoadedExtension},
    manifest::{CommandContribution, ViewContribution},
    runtime::{self, HostContext, UiRequest, WasmExtension, WorkspaceEvent},
};

/// Extension が提供するコマンド。
#[derive(Clone, Debug)]
pub struct ExtensionCommand {
    pub extension: String,
    pub command: CommandContribution,
}

/// 段階3: 起動した WASM の Extension（gpui の Global）。
pub struct WasmHost {
    pub context: HostContext,
    /// UI へのお願い。workspace が最初に1回だけ取り出す。
    requests: Option<futures::channel::mpsc::UnboundedReceiver<UiRequest>>,
    pub running: HashMap<String, Arc<WasmExtension>>,
    /// 起動できなかったもの（ID, 理由）。
    pub failed: Vec<(String, String)>,
    /// 起動中の数。
    pub starting: usize,
}

impl Global for WasmHost {}

impl WasmHost {
    pub fn take_requests(cx: &mut App) -> Option<futures::channel::mpsc::UnboundedReceiver<UiRequest>> {
        if !cx.has_global::<WasmHost>() {
            return None;
        }
        cx.global_mut::<WasmHost>().requests.take()
    }

    /// コマンドを実行する。終わったら結果が返る。
    pub fn run_command(
        extension: &str,
        command: &str,
        cx: &App,
    ) -> Option<futures::channel::oneshot::Receiver<Result<(), String>>> {
        let ext = cx.try_global::<WasmHost>()?.running.get(extension)?;
        Some(ext.run_command(command))
    }

    pub fn get(extension: &str, cx: &App) -> Option<Arc<WasmExtension>> {
        cx.try_global::<WasmHost>()?.running.get(extension).cloned()
    }

    /// 起動中か（まだ起動していないなら true、起動に失敗したら false）。
    pub fn is_pending(extension: &str, cx: &App) -> bool {
        cx.try_global::<WasmHost>()
            .is_some_and(|h| !h.running.contains_key(extension) && !h.failed.iter().any(|(id, _)| id == extension))
    }

    pub fn failure(extension: &str, cx: &App) -> Option<String> {
        let h = cx.try_global::<WasmHost>()?;
        h.failed.iter().find(|(id, _)| id == extension).map(|(_, e)| e.clone())
    }

    /// 起動中のすべての Extension に知らせる。
    pub fn broadcast(event: WorkspaceEvent, cx: &App) {
        if let Some(h) = cx.try_global::<WasmHost>() {
            for ext in h.running.values() {
                ext.workspace_event(event.clone());
            }
        }
    }
}

/// Extension が提供するビュー。
#[derive(Clone, Debug)]
pub struct ExtensionView {
    pub extension: String,
    pub view: ViewContribution,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stage {
    /// データ系の権限（テーマ・Token・文言・言語）を読み込み中。
    Loading,
    Ready,
}

/// 読み込んだ Extension の一覧（gpui の Global）。変化は `cx.observe_global::<ExtensionStore>` で受け取る。
pub struct ExtensionStore {
    pub stage: Stage,
    pub extensions: Vec<Arc<LoadedExtension>>,
    pub failed: Vec<Arc<FailedExtension>>,
    pub disabled: Vec<String>,
    pub tokens: ExtTokenSet,
    /// 拡張子（小文字）→ 言語名
    pub languages_by_extension: HashMap<String, String>,
    /// ファイル名 → 言語名
    pub languages_by_file_name: HashMap<String, String>,
    /// 段階1にかかった時間（ミリ秒）。
    pub load_ms: u128,
}

impl Global for ExtensionStore {}

impl ExtensionStore {
    pub fn get(cx: &App) -> Option<&ExtensionStore> {
        cx.try_global::<ExtensionStore>()
    }

    /// 有効な Extension が提供するテーマ。
    pub fn themes(&self) -> impl Iterator<Item = &theme::ThemeFile> {
        self.enabled().flat_map(|e| e.themes.iter())
    }

    /// 有効な Extension が提供するビュー。
    pub fn views(&self) -> Vec<ExtensionView> {
        self.enabled()
            .flat_map(|e| {
                e.manifest.contributes.views.iter().map(|v| ExtensionView {
                    extension: e.manifest.id.clone(),
                    view: v.clone(),
                })
            })
            .collect()
    }

    /// 有効な Extension が提供するコマンド。
    pub fn commands(&self) -> Vec<ExtensionCommand> {
        self.enabled()
            .flat_map(|e| {
                e.manifest.contributes.commands.iter().map(|c| ExtensionCommand {
                    extension: e.manifest.id.clone(),
                    command: c.clone(),
                })
            })
            .collect()
    }

    pub fn enabled(&self) -> impl Iterator<Item = &Arc<LoadedExtension>> {
        self.extensions
            .iter()
            .filter(|e| !self.disabled.contains(&e.manifest.id))
    }
}

/// 段階1を始める。発見と読み込みはバックグラウンド、反映は UI スレッドで1回だけ行う。
pub fn start(disabled: Vec<String>, cx: &mut App) {
    let (context, requests) = HostContext::new();
    cx.set_global(WasmHost {
        context,
        requests: Some(requests),
        running: HashMap::new(),
        failed: Vec::new(),
        starting: 0,
    });
    cx.set_global(ExtensionStore {
        stage: Stage::Loading,
        extensions: Vec::new(),
        failed: Vec::new(),
        disabled: disabled.clone(),
        tokens: ExtTokenSet::default(),
        languages_by_extension: HashMap::new(),
        languages_by_file_name: HashMap::new(),
        load_ms: 0,
    });
    let started = Instant::now();
    let task = cx.background_spawn(async move { loader::discover(loader::user_extensions_dir().as_deref()) });
    cx.spawn(async move |cx| {
        let (loaded, failed) = task.await;
        cx.update(|cx| apply(loaded, failed, disabled, started, cx));
    })
    .detach();
}

fn apply(
    loaded: Vec<LoadedExtension>,
    failed: Vec<FailedExtension>,
    disabled: Vec<String>,
    started: Instant,
    cx: &mut App,
) {
    let mut tokens = ExtTokenSet::default();
    let mut by_ext = HashMap::new();
    let mut by_name = HashMap::new();
    for e in loaded.iter().filter(|e| !disabled.contains(&e.manifest.id)) {
        for w in &e.warnings {
            log::warn!("Extension `{}`: {w}", e.manifest.id);
        }
        for (key, decl) in &e.manifest.tokens {
            if let Err(err) = tokens.declare(&e.manifest.id, key, decl.clone()) {
                log::warn!("Extension `{}`: {err}", e.manifest.id);
            }
        }
        for m in &e.messages {
            if let Err(errs) = i18n::add_messages(m.clone(), cx) {
                for err in errs {
                    log::warn!("Extension `{}`: {err}", e.manifest.id);
                }
            }
        }
        for l in &e.manifest.contributes.languages {
            for x in &l.extensions {
                by_ext.insert(x.to_lowercase(), l.name.clone());
            }
            for f in &l.file_names {
                by_name.insert(f.clone(), l.name.clone());
            }
        }
    }
    for f in &failed {
        log::error!(
            "Extension を読み込めなかった（{}）: {}",
            f.location,
            f.errors.join(" / ")
        );
    }
    let load_ms = started.elapsed().as_millis();
    log::info!(
        "Extension: {} 個を {load_ms}ms で読み込んだ（失敗 {} 個）",
        loaded.len(),
        failed.len()
    );
    let wasm: Vec<(crate::Manifest, Arc<[u8]>)> = loaded
        .iter()
        .filter(|e| !disabled.contains(&e.manifest.id))
        .filter_map(|e| Some((e.manifest.clone(), e.wasm.clone()?)))
        .collect();
    cx.set_global(ExtensionStore {
        stage: Stage::Ready,
        extensions: loaded.into_iter().map(Arc::new).collect(),
        failed: failed.into_iter().map(Arc::new).collect(),
        disabled,
        tokens,
        languages_by_extension: by_ext,
        languages_by_file_name: by_name,
        load_ms,
    });
    start_wasm(wasm, cx);
}

/// 段階3: WASM の Extension をそれぞれのスレッドで起動する。
fn start_wasm(list: Vec<(crate::Manifest, Arc<[u8]>)>, cx: &mut App) {
    let Some(host) = cx.try_global::<WasmHost>().map(|h| h.context.clone()) else {
        return;
    };
    for (manifest, wasm) in list {
        let id = manifest.id.clone();
        let (ext, ready) = runtime::spawn(manifest, wasm, host.clone());
        cx.global_mut::<WasmHost>().starting += 1;
        cx.spawn(async move |cx| {
            let result = ready.await.unwrap_or_else(|_| Err("起動中に止まった".into()));
            cx.update(|cx| {
                let h = cx.global_mut::<WasmHost>();
                h.starting -= 1;
                match result {
                    Ok(took) => {
                        log::info!("Extension `{id}` を {}ms で起動した", took.as_millis());
                        h.running.insert(id, Arc::new(ext));
                    }
                    Err(e) => {
                        log::error!("Extension `{id}` を起動できなかった: {e}");
                        h.failed.push((id, e));
                    }
                }
            });
        })
        .detach();
    }
}
