//! テーマと Token。Nucleus の色はすべてここを通る（docs/ARCHITECTURE.md §3）。

mod cache;
mod color;
mod ext;
mod file;
mod gpui_bridge;
mod resolve;
mod value;

include!(concat!(env!("OUT_DIR"), "/token.rs"));

use std::sync::Arc;

use gpui_kit::{App, AppContext as _};

pub use color::Color;
pub use ext::{ExtTokenDecl, ExtTokenId, ExtTokenSet, TokenKind};
pub use file::{Fonts, Mode, ThemeFile, builtin};
pub use gpui_bridge::{ActiveTheme, TokenColors, install, kit_config_json};
pub use resolve::{Diagnostics, ResolvedTheme, Snapshot, resolve};
pub use value::ValueExpr;

/// 起動時の初期化。
///
/// キャッシュがあればそれで（なければ同梱テーマを解決して）すぐに有効にする。
/// 拡張 Token を含む正式な解決は、拡張の読み込み後に [`apply`] で行う。
pub fn init(preferred: Mode, cx: &mut App) {
    let resolved = cache::load()
        .filter(|t| t.mode == preferred)
        .or_else(|| {
            resolve(&builtin(preferred), &ExtTokenSet::default())
                .ok()
                .map(|(t, _)| t)
        })
        .expect("bundled theme resolves");
    install(Arc::new(resolved), cx);
}

/// テーマと拡張 Token を解決して有効にする。キャッシュの保存はバックグラウンドで行う。
pub fn apply(theme: &ThemeFile, ext: &ExtTokenSet, cx: &mut App) -> Diagnostics {
    match resolve(theme, ext) {
        Ok((resolved, diag)) => {
            for w in &diag.warnings {
                log::warn!("{w}");
            }
            let resolved = Arc::new(resolved);
            let unchanged = cx.try_global::<ActiveTheme>().is_some_and(|a| *a.resolved == *resolved);
            if !unchanged {
                install(resolved.clone(), cx);
            }
            cx.background_spawn(async move {
                if let Err(e) = cache::save(&resolved) {
                    log::warn!("テーマのキャッシュを保存できなかった: {e}");
                }
            })
            .detach();
            diag
        }
        Err(diag) => {
            for e in &diag.errors {
                log::error!("{e}");
            }
            diag
        }
    }
}
