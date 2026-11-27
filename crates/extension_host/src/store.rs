//! 起動後の段階的な反映（docs/ARCHITECTURE.md §5）。
//! 段階1（データ系の権限）をここで行う。段階2（UI）と段階3（WASM）は P5・P6 で加わる。

use std::{collections::HashMap, sync::Arc, time::Instant};

use gpui_kit::{App, AppContext as _, Global};
use theme::ExtTokenSet;

use crate::loader::{self, FailedExtension, LoadedExtension};

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

    pub fn enabled(&self) -> impl Iterator<Item = &Arc<LoadedExtension>> {
        self.extensions
            .iter()
            .filter(|e| !self.disabled.contains(&e.manifest.id))
    }
}

/// 段階1を始める。発見と読み込みはバックグラウンド、反映は UI スレッドで1回だけ行う。
pub fn start(disabled: Vec<String>, cx: &mut App) {
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
}
